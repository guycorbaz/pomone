//! Task-calendar screen (month grid, filters, milestones, reschedule) wiring — extracted from `main.rs` (story 0.4). Shared
//! helpers stay reachable through `crate::…` re-exports.

use std::cell::RefCell;
use std::rc::Rc;

use slint::{ComponentHandle, SharedString};

use chrono::{Datelike, Days, Local};
use pomone_app::{extend_series_if_needed, reschedule_task};

use crate::{
    all_category_keys, first_of_month, open_planting_detail, open_task_form_for_edit, parse_id,
    refresh_task_calendar, weekday_offset_mon, UiState,
};

use crate::generated::MainWindow;

/// Register the task-calendar callbacks on the window. Called once
/// from `main()`; standard wiring shape — see `wiring/mod.rs`.
#[allow(clippy::too_many_lines)]
pub(crate) fn wire_task_calendar(window: &MainWindow, state: &Rc<RefCell<UiState>>) {
    // --- Task Calendar navigation + completion toggle ---
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_navigate_tasks(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            {
                let s = state.borrow();
                let today = Local::now().date_naive();
                if let Err(e) = s
                    .runtime
                    .block_on(async { extend_series_if_needed(s.app.repo(), today).await })
                {
                    tracing::warn!(error = %e, "failed to extend recurring task series");
                }
            }
            if let Err(e) = refresh_task_calendar(&window, &mut state.borrow_mut()) {
                tracing::error!(error = %e, "failed to refresh task calendar");
            }
            window.set_current_page(SharedString::from("tasks"));
            window.set_status_text(SharedString::from(""));
            window.set_status_is_error(false);
        });
    }
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_task_prev_month(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            {
                let mut s = state.borrow_mut();
                let (y, m) = prev_month(s.task_calendar_year, s.task_calendar_month);
                s.task_calendar_year = y;
                s.task_calendar_month = m;
            }
            if let Err(e) = refresh_task_calendar(&window, &mut state.borrow_mut()) {
                tracing::error!(error = %e, "failed to refresh task calendar");
            }
        });
    }
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_task_next_month(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            {
                let mut s = state.borrow_mut();
                let (y, m) = next_month(s.task_calendar_year, s.task_calendar_month);
                s.task_calendar_year = y;
                s.task_calendar_month = m;
            }
            if let Err(e) = refresh_task_calendar(&window, &mut state.borrow_mut()) {
                tracing::error!(error = %e, "failed to refresh task calendar");
            }
        });
    }
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_task_go_today(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            {
                let mut s = state.borrow_mut();
                let now = Local::now().date_naive();
                s.task_calendar_year = now.year();
                s.task_calendar_month = now.month();
            }
            if let Err(e) = refresh_task_calendar(&window, &mut state.borrow_mut()) {
                tracing::error!(error = %e, "failed to refresh task calendar");
            }
        });
    }
    // Click on a filter chip → toggle that category in the active set.
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_task_toggle_category(move |key| {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let mut s = state.borrow_mut();
            let k = key.to_string();
            if s.task_filter_categories.contains(&k) {
                s.task_filter_categories.remove(&k);
            } else {
                s.task_filter_categories.insert(k);
            }
            if let Err(e) = refresh_task_calendar(&window, &mut s) {
                tracing::error!(error = %e, "failed to refresh task calendar after filter toggle");
            }
        });
    }
    // Click on "Tout afficher" → restore all 8 categories in the filter set.
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_task_select_all_categories(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let mut s = state.borrow_mut();
            s.task_filter_categories = all_category_keys().into_iter().collect();
            if let Err(e) = refresh_task_calendar(&window, &mut s) {
                tracing::error!(error = %e, "failed to refresh task calendar after select-all");
            }
        });
    }
    // Toggle the crop-cycle milestone family (the second filter group).
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_task_toggle_milestones(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let mut s = state.borrow_mut();
            s.show_milestones = !s.show_milestones;
            if let Err(e) = refresh_task_calendar(&window, &mut s) {
                tracing::error!(error = %e, "failed to refresh task calendar after milestone toggle");
            }
        });
    }
    // Click on an existing task pill → load the task into the form and
    // switch to the task-form page in edit mode.
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_task_edit_requested(move |task_id_str| {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let mut s = state.borrow_mut();
            "tasks".clone_into(&mut s.task_form_previous_page);
            if let Err(e) = open_task_form_for_edit(&window, &mut s, &task_id_str) {
                tracing::error!(error = %e, "failed to open task form for edit");
            }
        });
    }
    // Drag a task pill onto another day → reschedule it. The page hands us the
    // drop point in the day-grid's local frame plus the per-cell pitch on each
    // axis; we derive the 0..41 cell index and map it to that grid date (same
    // grid_start math as `refresh_task_calendar`).
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_task_rescheduled(move |task_id_str, x, y, pitch_x, pitch_y| {
            let Some(window) = weak.upgrade() else {
                return;
            };
            // Degenerate geometry (zero-size grid mid-layout) → ignore.
            if !(pitch_x > 0.0 && pitch_y > 0.0) {
                return;
            }
            // Clamp keeps each axis inside the 7×6 grid, so the floored
            // values are tiny non-negative integers — the cast can't lose data.
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let index = {
                let col = (x / pitch_x).floor().clamp(0.0, 6.0) as i64;
                let row = (y / pitch_y).floor().clamp(0.0, 5.0) as i64;
                u64::try_from(row * 7 + col).unwrap_or(0)
            };

            let mut s = state.borrow_mut();
            let task_id: pomone_domain::TaskId = match parse_id(&task_id_str) {
                Ok(id) => id,
                Err(e) => {
                    tracing::error!(error = %e, "invalid task id on drop");
                    return;
                }
            };
            let first = first_of_month(s.task_calendar_year, s.task_calendar_month);
            let lead = weekday_offset_mon(first);
            let Some(grid_start) = first.checked_sub_days(Days::new(u64::from(lead))) else {
                tracing::error!("task calendar grid underflow on drop");
                return;
            };
            let Some(target) = grid_start.checked_add_days(Days::new(index)) else {
                tracing::error!("task calendar grid overflow on drop");
                return;
            };
            let result = s
                .runtime
                .block_on(async { reschedule_task(s.app.repo(), task_id, target).await });
            if let Err(e) = result {
                tracing::error!(error = %e, "failed to reschedule task");
                return;
            }
            if let Err(e) = refresh_task_calendar(&window, &mut s) {
                tracing::error!(error = %e, "failed to refresh task calendar after reschedule");
            }
        });
    }
    // Click on a read-only milestone pill → route to its planting (the
    // milestone is derived from the planting's schedule, not editable here).
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_task_milestone_clicked(move |planting_id_str| {
            let Some(window) = weak.upgrade() else {
                return;
            };
            open_planting_detail(&window, &mut state.borrow_mut(), &planting_id_str, "tasks");
        });
    }
}

/// Step `(year, month)` back one calendar month, wrapping at January.
fn prev_month(year: i32, month: u32) -> (i32, u32) {
    if month == 1 {
        (year - 1, 12)
    } else {
        (year, month - 1)
    }
}

/// Step `(year, month)` forward one calendar month, wrapping at December.
fn next_month(year: i32, month: u32) -> (i32, u32) {
    if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    }
}
