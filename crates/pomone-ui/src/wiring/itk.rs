//! ITK editor wiring (Epic 2, story 2.5). The editor lives on the crop detail
//! (Cultures master-detail): it lists the selected crop's activities and offers
//! add / edit / reorder / delete. Persistence + ordering live in `itk_view`.

use std::cell::RefCell;
use std::rc::Rc;

use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

use pomone_app::{
    count_generated_tasks_for_activity, delete_itk_activity, itk_implement_options,
    itk_method_options, list_itk_activities, list_task_type_options, move_itk_activity,
    save_itk_activity, AppError, ItkActivityInput,
};

use crate::forms::localize_app_error;
use crate::generated::{ItkActivityRow as SlintItkActivityRow, MainWindow};
use crate::state::PendingDelete;
use crate::UiState;

/// Register the ITK editor callbacks. Standard wiring shape (see `wiring/mod.rs`).
// One callback block per gesture (save / move / edit / cancel / delete); clippy's
// 100-line cap is too tight for a screen wiring five related callbacks.
#[allow(clippy::too_many_lines)]
pub(crate) fn wire_itk(window: &MainWindow, state: &Rc<RefCell<UiState>>) {
    // Save (add or update) — reads the form-* state Rust owns via the widgets.
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_itk_save(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let mut s = state.borrow_mut();
            let Some(crop_id) = selected_crop_id(&window, &s) else {
                return;
            };
            let editing_id = window.get_itk_form_editing_id().to_string();
            let type_idx = usize::try_from(window.get_itk_form_type_index().max(0)).unwrap_or(0);
            let method_idx =
                usize::try_from(window.get_itk_form_method_index().max(0)).unwrap_or(0);
            let impl_idx =
                usize::try_from(window.get_itk_form_implement_index().max(0)).unwrap_or(0);

            let input = ItkActivityInput {
                id: editing_id,
                crop_id,
                task_type_id: s
                    .itk_type_option_ids
                    .get(type_idx)
                    .cloned()
                    .unwrap_or_default(),
                offset_days: window.get_itk_form_offset().to_string(),
                // Index 0 of the method/implement models is the "— none —" entry.
                method_id: s
                    .itk_method_option_ids
                    .get(method_idx)
                    .cloned()
                    .unwrap_or_default(),
                implement_id: s
                    .itk_implement_option_ids
                    .get(impl_idx)
                    .cloned()
                    .unwrap_or_default(),
                label: window.get_itk_form_label().to_string(),
                notes: String::new(),
            };
            match s
                .runtime
                .block_on(async { save_itk_activity(s.app.repo(), &input).await })
            {
                Ok(_) => {
                    reset_form(&window);
                    window.set_status_text(SharedString::from(""));
                    window.set_status_is_error(false);
                    if let Err(e) = refresh_itk(&window, &mut s) {
                        tracing::error!(error = %e, "failed to refresh ITK after save");
                    }
                }
                Err(e) => {
                    let msg = localize_app_error(s.app.i18n(), &e);
                    window.set_status_text(SharedString::from(msg));
                    window.set_status_is_error(true);
                }
            }
        });
    }

    // Reorder.
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_itk_move(move |id, up| {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let mut s = state.borrow_mut();
            if let Err(e) = s
                .runtime
                .block_on(async { move_itk_activity(s.app.repo(), &id, up).await })
            {
                tracing::error!(error = %e, "failed to reorder ITK activity");
                return;
            }
            if let Err(e) = refresh_itk(&window, &mut s) {
                tracing::error!(error = %e, "failed to refresh ITK after move");
            }
        });
    }

    // Edit — prefill the form from the activity row.
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_itk_edit(move |id| {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let s = state.borrow();
            let Some(row) = s.itk_activity_rows.iter().find(|r| r.id == id.as_str()) else {
                return;
            };
            let type_idx = index_of(&s.itk_type_option_ids, &row.task_type_id).unwrap_or(0);
            // "— none —" is index 0, so an unset method/implement lands there.
            let method_idx = index_of(&s.itk_method_option_ids, &row.method_id).unwrap_or(0);
            let impl_idx = index_of(&s.itk_implement_option_ids, &row.implement_id).unwrap_or(0);
            window.set_itk_form_editing_id(SharedString::from(row.id.clone()));
            window.set_itk_form_type_index(i32::try_from(type_idx).unwrap_or(0));
            window.set_itk_form_offset(SharedString::from(row.offset_days.clone()));
            window.set_itk_form_method_index(i32::try_from(method_idx).unwrap_or(0));
            window.set_itk_form_implement_index(i32::try_from(impl_idx).unwrap_or(0));
            window.set_itk_form_label(SharedString::from(row.label.clone()));
        });
    }

    // Cancel edit — back to add mode.
    {
        let weak = window.as_weak();
        window.on_itk_cancel_edit(move || {
            if let Some(window) = weak.upgrade() {
                reset_form(&window);
            }
        });
    }

    // Delete — routed through the shared confirm dialog with a data-danger
    // message naming how many generated tasks reference the activity.
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_itk_remove(move |id| {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let mut s = state.borrow_mut();
            let count = s
                .runtime
                .block_on(async { count_generated_tasks_for_activity(s.app.repo(), &id).await })
                .unwrap_or(0);
            let msg = if count == 0 {
                s.app.i18n().t("itk-delete-confirm")
            } else {
                let mut args = fluent::FluentArgs::new();
                args.set("count", i64::try_from(count).unwrap_or(i64::MAX));
                s.app.i18n().t_args("itk-delete-confirm-referenced", &args)
            };
            s.pending_delete = Some(PendingDelete::ItkActivity(id.to_string()));
            window.set_confirm_message(SharedString::from(msg));
            window.set_confirm_visible(true);
        });
    }
}

/// The confirm-dialog accept handler for an ITK activity delete.
pub(crate) fn do_delete_itk_activity(window: &MainWindow, s: &mut UiState, id: &str) {
    let result: Result<(), AppError> = s
        .runtime
        .block_on(async { delete_itk_activity(s.app.repo(), id).await });
    match result {
        Ok(()) => {
            window.set_status_text(SharedString::from(""));
            window.set_status_is_error(false);
            if let Err(e) = refresh_itk(window, s) {
                tracing::error!(error = %e, "failed to refresh ITK after delete");
            }
        }
        Err(e) => {
            let msg = localize_app_error(s.app.i18n(), &e);
            window.set_status_text(SharedString::from(msg));
            window.set_status_is_error(true);
        }
    }
}

/// Rebuild the ITK editor for the currently-selected crop: the activity list
/// and the three picker models (+ their parallel id lists). Called when a crop
/// is selected and after any ITK edit. A no-op (empty lists) when no crop is
/// selected.
pub(crate) fn refresh_itk(window: &MainWindow, state: &mut UiState) -> anyhow::Result<()> {
    let i18n = state.app.i18n();
    let none_label = i18n.t("itk-none");

    // Options are crop-independent; the activity list is per-crop.
    let (activities, types, methods, implements) = {
        let crop_id = selected_crop_id(window, state);
        state.runtime.block_on(async {
            let types = list_task_type_options(state.app.repo()).await?;
            let methods = itk_method_options(state.app.repo()).await?;
            let implements = itk_implement_options(state.app.repo()).await?;
            let activities = match crop_id {
                Some(cid) => list_itk_activities(state.app.repo(), &cid, state.app.i18n()).await?,
                None => Vec::new(),
            };
            Ok::<_, AppError>((activities, types, methods, implements))
        })?
    };

    // Type picker: labels + ids (no "none" — a type is required).
    let type_labels: Vec<SharedString> = types
        .iter()
        .map(|t| SharedString::from(t.name.clone()))
        .collect();
    state.itk_type_option_ids = types.iter().map(|t| t.id.clone()).collect();
    window.set_itk_type_options(ModelRc::new(VecModel::from(type_labels)));

    // Method / implement pickers: a leading "— none —" entry (id ""), so index 0
    // means "unset". `has_*` hides the picker entirely when the catalog is empty.
    let (method_labels, method_ids) = with_none(&none_label, &methods);
    let (impl_labels, impl_ids) = with_none(&none_label, &implements);
    window.set_itk_has_methods(!methods.is_empty());
    window.set_itk_has_implements(!implements.is_empty());
    state.itk_method_option_ids = method_ids;
    state.itk_implement_option_ids = impl_ids;
    window.set_itk_method_options(ModelRc::new(VecModel::from(method_labels)));
    window.set_itk_implement_options(ModelRc::new(VecModel::from(impl_labels)));

    let rows: Vec<SlintItkActivityRow> = activities
        .iter()
        .map(|a| SlintItkActivityRow {
            id: SharedString::from(a.id.clone()),
            offset_label: SharedString::from(a.offset_label.clone()),
            task_type_name: SharedString::from(a.task_type_name.clone()),
            method_name: SharedString::from(a.method_name.clone()),
            implement_name: SharedString::from(a.implement_name.clone()),
            label: SharedString::from(a.label.clone()),
        })
        .collect();
    window.set_itk_activities(ModelRc::new(VecModel::from(rows)));
    state.itk_activity_rows = activities;
    Ok(())
}

// --- helpers ---

fn selected_crop_id(window: &MainWindow, state: &UiState) -> Option<String> {
    let idx = window.get_selected_crop_index();
    usize::try_from(idx)
        .ok()
        .and_then(|i| state.crop_ids.get(i).cloned())
}

fn reset_form(window: &MainWindow) {
    window.set_itk_form_editing_id(SharedString::from(""));
    window.set_itk_form_type_index(0);
    window.set_itk_form_offset(SharedString::from(""));
    window.set_itk_form_method_index(0);
    window.set_itk_form_implement_index(0);
    window.set_itk_form_label(SharedString::from(""));
}

fn index_of(ids: &[String], id: &str) -> Option<usize> {
    ids.iter().position(|x| x == id)
}

/// Build a picker's `(labels, ids)` with a leading "— none —" entry (empty id).
fn with_none(
    none_label: &str,
    options: &[pomone_app::ItkOption],
) -> (Vec<SharedString>, Vec<String>) {
    let mut labels = vec![SharedString::from(none_label)];
    let mut ids = vec![String::new()];
    for o in options {
        labels.push(SharedString::from(o.name.clone()));
        ids.push(o.id.clone());
    }
    (labels, ids)
}
