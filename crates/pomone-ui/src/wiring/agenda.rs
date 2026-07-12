//! Agenda screen (pending/completed task list) wiring — extracted from `main.rs` (story 0.4). Shared
//! helpers stay reachable through `crate::…` re-exports.

use std::cell::RefCell;
use std::rc::Rc;

use slint::{ComponentHandle, SharedString};

use chrono::Local;
use pomone_app::extend_series_if_needed;

use crate::{open_task_form_for_edit, refresh_agenda, UiState};

use crate::generated::MainWindow;

/// Register the agenda callbacks on the window. Called once
/// from `main()`; standard wiring shape — see `wiring/mod.rs`.
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
}
