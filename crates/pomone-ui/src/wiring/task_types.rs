//! Task-types catalog screen wiring — extracted from `main.rs` (story 0.4). Shared
//! helpers stay reachable through `crate::…` re-exports.

use std::cell::RefCell;
use std::rc::Rc;

use anyhow::{Context, Result};

use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

use fluent::FluentArgs;
use pomone_app::{
    create_task_type, get_task_type_for_edit, list_task_category_options, list_task_types_admin,
    update_task_type, AppError, TaskCategoryOption, TaskTypeAdminRow, TaskTypeEditForm,
};

use crate::{color_chooser_palette, parse_hex_color, FormError, PendingDelete, UiState};

use crate::generated::{MainWindow, TaskTypeAdminItem as SlintTaskTypeAdminItem};

/// Register the task-types callbacks on the window. Called once
/// from `main()`; standard wiring shape — see `wiring/mod.rs`.
pub(crate) fn wire_task_types(window: &MainWindow, state: &Rc<RefCell<UiState>>) {
    // --- Task Types catalog: navigation in (from Task Calendar header) ---
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_navigate_task_types(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let mut s = state.borrow_mut();
            if let Err(e) = open_task_types_for_create(&window, &mut s) {
                tracing::error!(error = %e, "failed to open task types page");
            }
        });
    }
    // --- Task Types: Back button → return to the Task Calendar ---
    {
        let weak = window.as_weak();
        window.on_navigate_task_types_back(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            window.set_current_page(SharedString::from("tasks"));
            window.set_task_types_status_text(SharedString::from(""));
        });
    }
    // --- Task Types: Save (create OR update based on is_edit_mode) ---
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_task_types_save(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let mut s = state.borrow_mut();
            match try_save_task_type_form(&window, &mut s) {
                Ok(()) => {
                    if let Err(e) = refresh_task_types(&window, &mut s) {
                        tracing::error!(error = %e, "failed to refresh task types after save");
                        return;
                    }
                    // Reset back to create mode so the user can chain creations.
                    reset_task_types_form_to_create(&window, &mut s);
                }
                Err(e) => {
                    let (text, is_err) = render_task_type_form_error(s.app.i18n(), e);
                    window.set_task_types_status_text(text);
                    window.set_task_types_status_is_error(is_err);
                }
            }
        });
    }
    // --- Task Types: Cancel edit (return form to create mode) ---
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_task_types_cancel_edit(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let mut s = state.borrow_mut();
            reset_task_types_form_to_create(&window, &mut s);
        });
    }
    // --- Task Types: Edit a row → pre-fill the form in edit mode ---
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_task_types_edit_row(move |id| {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let mut s = state.borrow_mut();
            if let Err(e) = open_task_type_form_for_edit(&window, &mut s, &id) {
                tracing::error!(error = %e, "failed to open task type edit form");
            }
        });
    }
    // --- Task Types: Delete a row (blocked at DB layer if in use) ---
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_task_types_delete_row(move |id| {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let mut s = state.borrow_mut();
            s.pending_delete = Some(PendingDelete::TaskType(id.to_string()));
            window.set_confirm_message(SharedString::from(
                s.app.i18n().t("confirm-delete-task-type"),
            ));
            window.set_confirm_visible(true);
        });
    }
}

/// Reload the Task Types admin list from the DB and push it to Slint.
/// Stores the parallel id table so click callbacks can resolve a row id
/// back to a typed `TaskTypeId`.
pub(crate) fn refresh_task_types(window: &MainWindow, state: &mut UiState) -> Result<()> {
    let rows: Vec<TaskTypeAdminRow> = state
        .runtime
        .block_on(async { list_task_types_admin(state.app.repo()).await })
        .context("failed to load task types")?;
    state.task_type_admin_ids = rows.iter().map(|r| r.id.clone()).collect();

    let i18n = state.app.i18n();
    let items: Vec<SlintTaskTypeAdminItem> = rows
        .into_iter()
        .map(|r| {
            let cat_label = i18n.t(&format!("category-{}", r.category));
            SlintTaskTypeAdminItem {
                id: SharedString::from(r.id),
                name: SharedString::from(r.name),
                category_label: SharedString::from(cat_label),
                color: parse_hex_color(&r.color),
                color_hex: SharedString::from(r.color),
                in_use: r.in_use,
            }
        })
        .collect();
    window.set_task_types_items(ModelRc::new(VecModel::from(items)));
    Ok(())
}

/// Reset the catalog form to a blank "create" state and clear the status banner.
pub(crate) fn reset_task_types_form_to_create(window: &MainWindow, state: &mut UiState) {
    state.editing_task_type_id.clear();
    window.set_task_types_is_edit_mode(false);
    window.set_task_types_form_name(SharedString::from(""));
    window.set_task_types_form_color(SharedString::from("#3C6E47"));
    window.set_task_types_form_color_preview(parse_hex_color("#3C6E47"));
    window.set_task_types_category_index(0);
    window.set_task_types_status_text(SharedString::from(""));
    window.set_task_types_status_is_error(false);
}

/// First-time entry into the catalog page: load categories + list, blank form.
fn open_task_types_for_create(window: &MainWindow, state: &mut UiState) -> Result<()> {
    window.set_task_types_color_palette(ModelRc::new(VecModel::from(color_chooser_palette())));
    populate_task_type_categories(window, state);
    refresh_task_types(window, state)?;
    reset_task_types_form_to_create(window, state);
    window.set_current_page(SharedString::from("task-types"));
    Ok(())
}

/// Load one type into the form and switch to edit mode (category locked).
fn open_task_type_form_for_edit(window: &MainWindow, state: &mut UiState, id: &str) -> Result<()> {
    populate_task_type_categories(window, state);
    refresh_task_types(window, state)?;
    let form: TaskTypeEditForm = state
        .runtime
        .block_on(async { get_task_type_for_edit(state.app.repo(), id).await })
        .context("failed to load task type for edit")?;

    let cat_idx = state
        .task_type_category_keys
        .iter()
        .position(|k| k == &form.category)
        .map_or(0, |i| i32::try_from(i).unwrap_or(0));

    state.editing_task_type_id.clone_from(&form.id);
    window.set_task_types_is_edit_mode(true);
    window.set_task_types_category_index(cat_idx);
    window.set_task_types_form_color_preview(parse_hex_color(&form.color));
    window.set_task_types_form_name(SharedString::from(form.name));
    window.set_task_types_form_color(SharedString::from(form.color));
    window.set_task_types_status_text(SharedString::from(""));
    window.set_task_types_status_is_error(false);
    Ok(())
}

/// Persist the form (create or update). Validation: non-empty name +
/// `#RGB` / `#RRGGBB` color (the domain re-validates both, but checking
/// here keeps the error message closer to the field).
fn try_save_task_type_form(window: &MainWindow, state: &mut UiState) -> Result<(), FormError> {
    let i18n = state.app.i18n();
    let name = window.get_task_types_form_name().to_string();
    if name.trim().is_empty() {
        return Err(FormError::Validation(i18n.t("error-name-required")));
    }
    let color = window.get_task_types_form_color().to_string();
    if color.trim().is_empty() {
        return Err(FormError::Validation(
            i18n.t("error-task-type-color-required"),
        ));
    }

    let is_edit = window.get_task_types_is_edit_mode();
    if is_edit {
        let id = state.editing_task_type_id.clone();
        if id.is_empty() {
            return Err(FormError::Validation(
                i18n.t("error-task-type-edit-id-missing"),
            ));
        }
        state
            .runtime
            .block_on(async {
                update_task_type(state.app.repo(), &id, name.trim(), color.trim()).await
            })
            .map_err(FormError::Service)?;
    } else {
        let cat_idx = window.get_task_types_category_index();
        let cat_idx_usize = usize::try_from(cat_idx.max(0)).unwrap_or(0);
        let category_key = state
            .task_type_category_keys
            .get(cat_idx_usize)
            .cloned()
            .ok_or_else(|| FormError::Validation(i18n.t("error-task-type-category-required")))?;
        state
            .runtime
            .block_on(async {
                create_task_type(state.app.repo(), name.trim(), &category_key, color.trim())
                    .await
                    .map(|_| ())
            })
            .map_err(FormError::Service)?;
    }
    Ok(())
}

/// Same shape as [`render_task_form_error`] but with the task-types-specific
/// service template, plus a special case for the `task_type_in_use` sentinel
/// returned by `delete_task_type` so the user sees a clear localized message
/// instead of the raw FK error.
pub(crate) fn render_task_type_form_error(
    i18n: &pomone_app::I18n,
    err: FormError,
) -> (SharedString, bool) {
    let msg = match err {
        FormError::Validation(text) => {
            let mut args = FluentArgs::new();
            args.set("message", text);
            i18n.t_args("status-validation-failed", &args)
        }
        FormError::Service(AppError::Inconsistent(ref code)) if code == "task_type_in_use" => {
            i18n.t("error-task-type-in-use")
        }
        FormError::Service(app_err) => {
            let mut args = FluentArgs::new();
            args.set("message", app_err.to_string());
            i18n.t_args("status-task-type-failed", &args)
        }
    };
    (SharedString::from(msg), true)
}

/// One-shot initialization of the category ComboBox model. Idempotent: a
/// second call is a no-op (we keep the eight canonical categories around
/// the entire session).
fn populate_task_type_categories(window: &MainWindow, state: &mut UiState) {
    if !state.task_type_category_keys.is_empty() {
        return;
    }
    let i18n = state.app.i18n();
    let opts: Vec<TaskCategoryOption> = list_task_category_options();
    state.task_type_category_keys = opts.iter().map(|o| o.key.clone()).collect();
    let labels: Vec<SharedString> = opts
        .iter()
        .map(|o| SharedString::from(i18n.t(&o.label_key)))
        .collect();
    window.set_task_types_category_labels(ModelRc::new(VecModel::from(labels)));
}
