//! Task form (create/edit/delete a task or recurring series) wiring — extracted from `main.rs` (story 0.4). Shared
//! helpers stay reachable through `crate::…` re-exports.

use std::cell::RefCell;
use std::rc::Rc;

use anyhow::{Context, Result};

use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

use chrono::{Datelike, Local, NaiveDate};
use fluent::FluentArgs;
use pomone_app::{
    create_recurring_task, create_task, get_task_for_edit, list_planting_choices,
    list_task_type_options, recurrence_unit_str, update_task, AppError, PlantingChoice,
    TaskEditForm, TaskTypeOption,
};
use pomone_domain::RecurrenceUnit;

use crate::{
    localize_app_error, refresh_after_task_form, today_iso, FormError, PendingDelete, UiState,
};

use crate::generated::MainWindow;

/// Register the task-form callbacks on the window. Called once
/// from `main()`; standard wiring shape — see `wiring/mod.rs`.
pub(crate) fn wire_task_form(window: &MainWindow, state: &Rc<RefCell<UiState>>) {
    // Click on "+ Nouvelle tâche" header button → reset the form and
    // switch to the task-form page in create mode.
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_task_new_requested(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let mut s = state.borrow_mut();
            "tasks".clone_into(&mut s.task_form_previous_page);
            if let Err(e) = open_task_form_for_create(&window, &mut s) {
                tracing::error!(error = %e, "failed to open task form for create");
            }
        });
    }
    // Task form: Save (create OR update depending on is_edit_mode).
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_task_form_save(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let mut s = state.borrow_mut();
            match try_save_task_form(&window, &mut s) {
                Ok(()) => {
                    let prev = s.task_form_previous_page.clone();
                    window.set_current_page(SharedString::from(prev.clone()));
                    refresh_after_task_form(&window, &mut s, &prev);
                }
                Err(e) => {
                    let (text, is_err) = render_task_form_error(s.app.i18n(), e);
                    window.set_task_form_status_text(text);
                    window.set_task_form_status_is_error(is_err);
                }
            }
        });
    }
    // Task form: Cancel → drop changes and route back.
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_task_form_cancel(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let prev = state.borrow().task_form_previous_page.clone();
            window.set_current_page(SharedString::from(prev));
            window.set_task_form_status_text(SharedString::from(""));
        });
    }
    // Task form: Delete the task in edit mode.
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_task_form_delete(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let mut s = state.borrow_mut();
            let task_id = s.editing_task_id.clone();
            if task_id.is_empty() {
                return; // shouldn't happen — Delete is hidden in create mode.
            }
            s.pending_delete = Some(PendingDelete::Task(task_id));
            window.set_confirm_message(SharedString::from(s.app.i18n().t("confirm-delete-task")));
            window.set_confirm_visible(true);
        });
    }
}

/// Refresh the task form's dropdown models (task types + plantings) from
/// the DB and store the parallel UUID strings in `state` so the form's
/// `save` handler can resolve indices back to typed IDs.
///
/// Planting list is prefixed with a "— Aucun —" sentinel (empty UUID) so
/// the user can opt out of attaching the task to a planting.
fn populate_task_form_options(window: &MainWindow, state: &mut UiState) -> Result<()> {
    // Recurrence units (3 entries, never change after first call) — done
    // here so it piggy-backs on every form open and we don't need a
    // separate boot-time hook.
    populate_recurrence_units(window, state);

    let i18n = state.app.i18n();
    let none_label = i18n.t("task-form-planting-none");

    let (type_opts, planting_opts): (Vec<TaskTypeOption>, Vec<PlantingChoice>) = state
        .runtime
        .block_on(async {
            let types = list_task_type_options(state.app.repo()).await?;
            let plantings = list_planting_choices(state.app.repo()).await?;
            Ok::<_, AppError>((types, plantings))
        })
        .context("failed to load task form options")?;

    state.task_form_type_ids = type_opts.iter().map(|t| t.id.clone()).collect();
    let type_labels: Vec<SharedString> = type_opts
        .into_iter()
        .map(|t| SharedString::from(t.name))
        .collect();
    window.set_task_form_type_labels(ModelRc::new(VecModel::from(type_labels)));

    let mut planting_ids: Vec<String> = Vec::with_capacity(planting_opts.len() + 1);
    let mut planting_labels: Vec<SharedString> = Vec::with_capacity(planting_opts.len() + 1);
    planting_ids.push(String::new());
    planting_labels.push(SharedString::from(none_label));
    for p in planting_opts {
        planting_ids.push(p.id);
        planting_labels.push(SharedString::from(p.label));
    }
    state.task_form_planting_ids = planting_ids;
    window.set_task_form_planting_labels(ModelRc::new(VecModel::from(planting_labels)));
    Ok(())
}

/// Reset the form to "create" mode and switch to the task-form page.
fn open_task_form_for_create(window: &MainWindow, state: &mut UiState) -> Result<()> {
    populate_task_form_options(window, state)?;
    state.editing_task_id.clear();
    window.set_task_form_is_edit_mode(false);
    window.set_task_form_is_part_of_series(false);
    window.set_task_form_type_index(0);
    window.set_task_form_planting_index(0);
    let today = today_iso();
    window.set_task_form_planned_on_text(SharedString::from(today.clone()));
    window.set_task_form_notes_text(SharedString::from(""));
    window.set_task_form_completed(false);
    window.set_task_form_status_text(SharedString::from(""));
    window.set_task_form_status_is_error(false);
    // Recurrence sub-section defaults: unchecked, "every 7 days", end date
    // pre-filled with today + 1 year (the user can clear it for open-ended).
    window.set_task_form_recurring(false);
    window.set_task_form_recurrence_interval_text(SharedString::from("7"));
    window.set_task_form_recurrence_unit_index(0);
    window.set_task_form_recurrence_end_on_text(SharedString::from(default_end_date_iso(&today)));
    window.set_current_page(SharedString::from("task-form"));
    Ok(())
}

/// Load the task into the form, switch to "edit" mode, and route to the page.
pub(crate) fn open_task_form_for_edit(
    window: &MainWindow,
    state: &mut UiState,
    task_id_str: &str,
) -> Result<()> {
    populate_task_form_options(window, state)?;

    let form: TaskEditForm = state
        .runtime
        .block_on(async { get_task_for_edit(state.app.repo(), task_id_str).await })
        .context("failed to load task for edit")?;

    let type_idx = state
        .task_form_type_ids
        .iter()
        .position(|id| id == &form.task_type_id)
        .map_or(0, |i| i32::try_from(i).unwrap_or(0));
    let planting_idx = state
        .task_form_planting_ids
        .iter()
        .position(|id| id == &form.planting_id)
        .map_or(0, |i| i32::try_from(i).unwrap_or(0));

    state.editing_task_id.clone_from(&form.task_id);
    window.set_task_form_is_edit_mode(true);
    window.set_task_form_is_part_of_series(form.is_part_of_series);
    window.set_task_form_type_index(type_idx);
    window.set_task_form_planting_index(planting_idx);
    window.set_task_form_planned_on_text(SharedString::from(form.planned_on));
    window.set_task_form_notes_text(SharedString::from(form.notes));
    window.set_task_form_completed(form.completed);
    window.set_task_form_status_text(SharedString::from(""));
    window.set_task_form_status_is_error(false);
    // Recurrence sub-section is hidden in edit mode — reset flags so that
    // the next "+ Nouvelle tâche" doesn't see leftover state.
    window.set_task_form_recurring(false);
    window.set_current_page(SharedString::from("task-form"));
    Ok(())
}

/// Persist the form contents — either as a new task or as an update to the
/// existing `state.editing_task_id`. Returns a `FormError` on validation or
/// service failure so the caller can route the message into the status banner.
// The branches (edit / recurring create / one-shot create) sit naturally
// in one function so the read flows top-to-bottom; clippy's 100-line cap
// is too tight for this kind of dispatcher.
#[allow(clippy::too_many_lines)]
fn try_save_task_form(window: &MainWindow, state: &mut UiState) -> Result<(), FormError> {
    let i18n = state.app.i18n();
    if state.task_form_type_ids.is_empty() {
        return Err(FormError::Validation(i18n.t("error-task-no-types")));
    }

    let type_idx = window.get_task_form_type_index();
    let type_idx_usize = usize::try_from(type_idx.max(0)).unwrap_or(0);
    let task_type_id = state
        .task_form_type_ids
        .get(type_idx_usize)
        .cloned()
        .ok_or_else(|| FormError::Validation(i18n.t("error-task-type-required")))?;

    let planting_idx = window.get_task_form_planting_index();
    let planting_idx_usize = usize::try_from(planting_idx.max(0)).unwrap_or(0);
    let planting_id = state
        .task_form_planting_ids
        .get(planting_idx_usize)
        .cloned()
        .unwrap_or_default();

    let planned_on = window.get_task_form_planned_on_text().to_string();
    if planned_on.trim().is_empty() {
        return Err(FormError::Validation(i18n.t("error-date-required")));
    }
    let notes = window.get_task_form_notes_text().to_string();
    let completed = window.get_task_form_completed();
    let now = Local::now();
    let today = now.date_naive();
    // The UI/CLI is the only layer allowed to read the clock; the app API takes
    // this injected `recorded_at` (story 1.3).
    let recorded_at = now.naive_local();

    let is_edit = window.get_task_form_is_edit_mode();
    if is_edit {
        let task_id = state.editing_task_id.clone();
        if task_id.is_empty() {
            return Err(FormError::Validation(i18n.t("error-task-edit-id-missing")));
        }
        state
            .runtime
            .block_on(async {
                update_task(
                    state.app.repo(),
                    &task_id,
                    &task_type_id,
                    &planned_on,
                    &notes,
                    completed,
                    today,
                    recorded_at,
                )
                .await
            })
            .map_err(FormError::Service)?;
    } else if window.get_task_form_recurring() {
        // Recurring-create path. Validation: interval must parse as a
        // positive integer; unit must be present in our key table.
        let interval_text = window.get_task_form_recurrence_interval_text().to_string();
        let interval: u32 = interval_text
            .trim()
            .parse()
            .map_err(|_| FormError::Validation(i18n.t("error-positive-required")))?;
        if interval == 0 {
            return Err(FormError::Validation(i18n.t("error-positive-required")));
        }
        let unit_idx = window.get_task_form_recurrence_unit_index();
        let unit_key = state
            .task_form_recurrence_unit_keys
            .get(usize::try_from(unit_idx.max(0)).unwrap_or(0))
            .cloned()
            .ok_or_else(|| FormError::Validation(i18n.t("error-recurrence-unit-required")))?;
        let end_text = window.get_task_form_recurrence_end_on_text().to_string();
        let end_arg = if end_text.trim().is_empty() {
            None
        } else {
            Some(end_text.clone())
        };

        state
            .runtime
            .block_on(async {
                create_recurring_task(
                    state.app.repo(),
                    &planting_id,
                    &task_type_id,
                    &planned_on,
                    &notes,
                    interval,
                    &unit_key,
                    end_arg.as_deref(),
                    today,
                )
                .await
                .map(|_| ())
            })
            .map_err(FormError::Service)?;
    } else {
        state
            .runtime
            .block_on(async {
                create_task(
                    state.app.repo(),
                    &planting_id,
                    &task_type_id,
                    &planned_on,
                    &notes,
                    completed,
                    today,
                    recorded_at,
                )
                .await
                .map(|_| ())
            })
            .map_err(FormError::Service)?;
    }
    Ok(())
}

/// One-shot initialization of the recurrence-unit ComboBox model.
/// Idempotent: a second call is a no-op (we keep the three units around
/// the entire session, with `"days"` always at index 0).
fn populate_recurrence_units(window: &MainWindow, state: &mut UiState) {
    if !state.task_form_recurrence_unit_keys.is_empty() {
        return;
    }
    let i18n = state.app.i18n();
    let units = [
        RecurrenceUnit::Days,
        RecurrenceUnit::Weeks,
        RecurrenceUnit::Months,
    ];
    state.task_form_recurrence_unit_keys = units
        .iter()
        .map(|u| recurrence_unit_str(*u).to_owned())
        .collect();
    let labels: Vec<SharedString> = units
        .iter()
        .map(|u| {
            let key = match u {
                RecurrenceUnit::Days => "recurrence-unit-days",
                RecurrenceUnit::Weeks => "recurrence-unit-weeks",
                RecurrenceUnit::Months => "recurrence-unit-months",
            };
            SharedString::from(i18n.t(key))
        })
        .collect();
    window.set_task_form_recurrence_unit_labels(ModelRc::new(VecModel::from(labels)));
}

/// Suggested series end-date: `planned_on + 1 year`. Falls back to
/// `planned_on` itself if the source isn't parseable (defensive — the
/// caller passes `today_iso()` or a freshly formatted date).
fn default_end_date_iso(start_iso: &str) -> String {
    match NaiveDate::parse_from_str(start_iso, "%Y-%m-%d") {
        Ok(d) => NaiveDate::from_ymd_opt(d.year() + 1, d.month(), d.day())
            .unwrap_or(d)
            .format("%Y-%m-%d")
            .to_string(),
        Err(_) => start_iso.to_owned(),
    }
}

/// Same as [`render_form_error`] but with a task-specific service template
/// so the status banner reads correctly when the failing operation is a
/// task save / delete rather than a planting one.
pub(crate) fn render_task_form_error(
    i18n: &pomone_app::I18n,
    err: FormError,
) -> (SharedString, bool) {
    let msg = match err {
        FormError::Validation(text) => {
            let mut args = FluentArgs::new();
            args.set("message", text);
            i18n.t_args("status-validation-failed", &args)
        }
        FormError::Service(app_err) => {
            let mut args = FluentArgs::new();
            args.set("message", localize_app_error(i18n, &app_err));
            i18n.t_args("status-task-failed", &args)
        }
    };
    (SharedString::from(msg), true)
}
