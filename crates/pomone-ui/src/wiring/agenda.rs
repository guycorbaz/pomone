//! Agenda screen (pending/completed task list) wiring — extracted from `main.rs` (story 0.4). Shared
//! helpers stay reachable through `crate::…` re-exports.

use std::cell::RefCell;
use std::rc::Rc;

use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

use chrono::Local;
use pomone_app::{
    extend_series_if_needed, mark_task_done, parse_id, reopen_task, skip_task, AppError,
};
use pomone_domain::field_event::SkipReason;
use pomone_domain::TaskId;

use crate::forms::localize_app_error;
use crate::{open_task_form_for_edit, refresh_agenda, UiState};

use crate::generated::MainWindow;

/// Register the agenda callbacks on the window. Called once
/// from `main()`; standard wiring shape — see `wiring/mod.rs`.
// One callback block per gesture keeps the flow linear; clippy's 100-line cap
// is too tight for a screen that now wires navigation + four settle gestures.
#[allow(clippy::too_many_lines)]
pub(crate) fn wire_agenda(window: &MainWindow, state: &Rc<RefCell<UiState>>) {
    // --- Agenda navigation + row click ---
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_navigate_agenda(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            {
                // Top up open-ended recurring series so the agenda's upcoming
                // window doesn't miss occurrences that haven't been materialized.
                let s = state.borrow();
                let today = Local::now().date_naive();
                if let Err(e) = s
                    .runtime
                    .block_on(async { extend_series_if_needed(s.app.repo(), today).await })
                {
                    tracing::warn!(error = %e, "failed to extend recurring task series");
                }
            }
            if let Err(e) = refresh_agenda(&window, &mut state.borrow_mut()) {
                tracing::error!(error = %e, "failed to refresh agenda");
            }
            window.set_current_page(SharedString::from("agenda"));
            window.set_status_text(SharedString::from(""));
            window.set_status_is_error(false);
        });
    }
    // Click on an agenda row → open the shared task edit form, routing back
    // to the agenda on save/cancel/delete.
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_agenda_task_clicked(move |task_id_str| {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let mut s = state.borrow_mut();
            "agenda".clone_into(&mut s.task_form_previous_page);
            if let Err(e) = open_task_form_for_edit(&window, &mut s, &task_id_str) {
                tracing::error!(error = %e, "failed to open task form from agenda");
            }
        });
    }
    // ⋯ menu → «Marquer fait»: record a Done fact in place, then refresh.
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_agenda_mark_done(move |task_id_str| {
            let Some(window) = weak.upgrade() else {
                return;
            };
            settle(
                &window,
                &state,
                &task_id_str,
                "status-task-done",
                |repo, id, on, at| Box::pin(async move { mark_task_done(repo, id, on, at).await }),
            );
        });
    }
    // ⋯ menu → «Corriger»: reopen any settled state (an explicit correction).
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_agenda_reopen(move |task_id_str| {
            let Some(window) = weak.upgrade() else {
                return;
            };
            settle(
                &window,
                &state,
                &task_id_str,
                "status-task-reopened",
                |repo, id, on, at| Box::pin(async move { reopen_task(repo, id, on, at).await }),
            );
        });
    }
    // ⋯ menu → «Abandonner…»: open the skip-reason dialog for this task.
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_agenda_skip_requested(move |task_id_str| {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let mut s = state.borrow_mut();
            s.agenda_skip_target = task_id_str.to_string();
            // Populate the closed-set reason picker from the enum's canonical
            // order so the on-screen list can't drift from the domain.
            let i18n = s.app.i18n();
            let keys: Vec<String> = SkipReason::ALL
                .iter()
                .map(|r| r.as_str().to_owned())
                .collect();
            let labels: Vec<SharedString> = SkipReason::ALL
                .iter()
                .map(|r| SharedString::from(i18n.t(&format!("skip-reason-{}", r.as_str()))))
                .collect();
            s.agenda_skip_reason_keys = keys;
            window.set_skip_dialog_reason_labels(ModelRc::new(VecModel::from(labels)));
            window.set_skip_dialog_reason_index(0);
            window.set_skip_dialog_note(SharedString::from(""));
            window.set_skip_dialog_visible(true);
        });
    }
    // Skip dialog «Abandonner»: read the chosen reason + note, record the skip.
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_skip_dialog_accepted(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let (task_id_str, reason, note) = {
                let s = state.borrow();
                let idx =
                    usize::try_from(window.get_skip_dialog_reason_index().max(0)).unwrap_or(0);
                let reason = s
                    .agenda_skip_reason_keys
                    .get(idx)
                    .and_then(|k| SkipReason::from_literal(k))
                    .unwrap_or(SkipReason::Other);
                let note_text = window.get_skip_dialog_note().to_string();
                let note = if note_text.trim().is_empty() {
                    None
                } else {
                    Some(note_text.trim().to_owned())
                };
                (s.agenda_skip_target.clone(), reason, note)
            };
            window.set_skip_dialog_visible(false);
            settle(
                &window,
                &state,
                &task_id_str,
                "status-task-skipped",
                move |repo, id, on, at| {
                    let note = note.clone();
                    Box::pin(async move { skip_task(repo, id, on, reason, note, at).await })
                },
            );
            state.borrow_mut().agenda_skip_target.clear();
        });
    }
    // Skip dialog «Annuler»: dismiss without recording anything.
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_skip_dialog_cancelled(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            window.set_skip_dialog_visible(false);
            state.borrow_mut().agenda_skip_target.clear();
        });
    }
}

/// Shared tail for every ⋯-menu settle gesture: parse the id, run `op` (a fact
/// write) on today's date with an injected `recorded_at`, surface any error as a
/// localized status, and refresh the list. `on`/`recorded_at` follow the story
/// 1.3 rule — only the UI reads the clock.
fn settle<F>(
    window: &MainWindow,
    state: &Rc<RefCell<UiState>>,
    task_id_str: &str,
    ok_key: &str,
    op: F,
) where
    F: FnOnce(
        &dyn pomone_db::Repository,
        TaskId,
        chrono::NaiveDate,
        chrono::NaiveDateTime,
    )
        -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), AppError>> + '_>>,
{
    let mut s = state.borrow_mut();
    let task_id: TaskId = match parse_id(task_id_str) {
        Ok(id) => id,
        Err(e) => {
            tracing::error!(error = %e, "invalid task id for settle gesture");
            return;
        }
    };
    let now = Local::now();
    let today = now.date_naive();
    let recorded_at = now.naive_local();
    let result = s
        .runtime
        .block_on(async { op(s.app.repo(), task_id, today, recorded_at).await });
    match result {
        Ok(()) => {
            let msg = s.app.i18n().t(ok_key);
            window.set_status_text(SharedString::from(msg));
            window.set_status_is_error(false);
        }
        Err(e) => {
            let msg = localize_app_error(s.app.i18n(), &e);
            window.set_status_text(SharedString::from(msg));
            window.set_status_is_error(true);
        }
    }
    if let Err(e) = refresh_agenda(window, &mut s) {
        tracing::error!(error = %e, "failed to refresh agenda after settle");
    }
}
