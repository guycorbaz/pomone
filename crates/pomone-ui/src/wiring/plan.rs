//! Plan-grid wiring (Epic 2, story 2.3). The grid emits `commit` with a row's
//! current cell strings; here we resolve the row id + picked variety id, then
//! validate + persist through `plan_view`. An invalid commit leaves the cells
//! as typed (no refresh) and explains in the status bar; a valid one refreshes.

use std::cell::RefCell;
use std::rc::Rc;

use slint::{ComponentHandle, SharedString};

use pomone_app::{delete_plan_line, duplicate_plan_line, save_plan_line, PlanLineInput};

use crate::forms::localize_app_error;
use crate::{refresh_plan, UiState};

use crate::generated::MainWindow;

/// Register the plan-grid callbacks. Standard wiring shape (see `wiring/mod.rs`).
// One callback block per gesture (navigate / commit / remove / add); clippy's
// 100-line cap is too tight for a screen wiring four related callbacks.
#[allow(clippy::too_many_lines)]
pub(crate) fn wire_plan(window: &MainWindow, state: &Rc<RefCell<UiState>>) {
    // Navigate → load the grid.
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_navigate_plan(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            if let Err(e) = refresh_plan(&window, &mut state.borrow_mut()) {
                tracing::error!(error = %e, "failed to refresh plan grid");
            }
            window.set_current_page(SharedString::from("plan"));
            window.set_plan_status_text(SharedString::from(""));
            window.set_plan_status_is_error(false);
        });
    }

    // Commit a row: resolve id + variety, validate + persist.
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_plan_commit(
            move |row_index, variety_index, series, bed_meters, stagger, first_on, draft, notes| {
                let Some(window) = weak.upgrade() else {
                    return;
                };
                let mut s = state.borrow_mut();

                let id = usize::try_from(row_index)
                    .ok()
                    .and_then(|i| s.plan_rows.get(i))
                    .map(|r| r.id.clone())
                    .unwrap_or_default();
                let variety_id = usize::try_from(variety_index)
                    .ok()
                    .and_then(|i| s.plan_variety_option_ids.get(i))
                    .cloned()
                    .unwrap_or_default();

                let input = PlanLineInput {
                    id,
                    variety_id,
                    series: series.to_string(),
                    bed_meters: bed_meters.to_string(),
                    stagger_days: stagger.to_string(),
                    first_on: first_on.to_string(),
                    draft,
                    notes: notes.to_string(),
                };
                let result = s
                    .runtime
                    .block_on(async { save_plan_line(s.app.repo(), &input).await });
                match result {
                    Ok(saved_id) => {
                        s.plan_last_edited_id = saved_id;
                        window.set_plan_status_text(SharedString::from(""));
                        window.set_plan_status_is_error(false);
                        if let Err(e) = refresh_plan(&window, &mut s) {
                            tracing::error!(error = %e, "failed to refresh plan grid after commit");
                        }
                    }
                    Err(e) => {
                        // Keep the cells as typed (no refresh) — the row stays open.
                        let msg = localize_app_error(s.app.i18n(), &e);
                        window.set_plan_status_text(SharedString::from(msg));
                        window.set_plan_status_is_error(true);
                    }
                }
            },
        );
    }

    // Delete a row.
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_plan_remove(move |row_index| {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let mut s = state.borrow_mut();
            let Some(id) = usize::try_from(row_index)
                .ok()
                .and_then(|i| s.plan_rows.get(i))
                .map(|r| r.id.clone())
            else {
                return;
            };
            if let Err(e) = s
                .runtime
                .block_on(async { delete_plan_line(s.app.repo(), &id).await })
            {
                let msg = localize_app_error(s.app.i18n(), &e);
                window.set_plan_status_text(SharedString::from(msg));
                window.set_plan_status_is_error(true);
                return;
            }
            if let Err(e) = refresh_plan(&window, &mut s) {
                tracing::error!(error = %e, "failed to refresh plan grid after delete");
            }
        });
    }

    // Add a line: a valid draft placeholder (first variety, 1 series × 1 m) the
    // user then edits in place. Needs at least one variety to reference.
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_plan_add_line(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let mut s = state.borrow_mut();
            let Some(variety_id) = s.plan_variety_option_ids.first().cloned() else {
                let msg = s.app.i18n().t("plan-no-variety");
                window.set_plan_status_text(SharedString::from(msg));
                window.set_plan_status_is_error(true);
                return;
            };
            let input = PlanLineInput {
                id: String::new(),
                variety_id,
                series: "1".to_owned(),
                bed_meters: "1".to_owned(),
                stagger_days: "0".to_owned(),
                first_on: String::new(),
                draft: true,
                notes: String::new(),
            };
            match s
                .runtime
                .block_on(async { save_plan_line(s.app.repo(), &input).await })
            {
                Ok(new_id) => s.plan_last_edited_id = new_id,
                Err(e) => {
                    let msg = localize_app_error(s.app.i18n(), &e);
                    window.set_plan_status_text(SharedString::from(msg));
                    window.set_plan_status_is_error(true);
                    return;
                }
            }
            window.set_plan_status_text(SharedString::from(""));
            window.set_plan_status_is_error(false);
            if let Err(e) = refresh_plan(&window, &mut s) {
                tracing::error!(error = %e, "failed to refresh plan grid after add");
            }
        });
    }

    // Duplicate a specific row (the ⧉ button).
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_plan_duplicate(move |row_index| {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let mut s = state.borrow_mut();
            let Some(id) = usize::try_from(row_index)
                .ok()
                .and_then(|i| s.plan_rows.get(i))
                .map(|r| r.id.clone())
            else {
                return;
            };
            duplicate_and_refresh(&window, &mut s, &id);
        });
    }

    // Ctrl+D — duplicate the last-touched line (session-aware).
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_plan_duplicate_current(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let mut s = state.borrow_mut();
            // Fall back to the last row if nothing was touched yet this session.
            let target = if s.plan_last_edited_id.is_empty() {
                s.plan_rows.last().map(|r| r.id.clone())
            } else {
                Some(s.plan_last_edited_id.clone())
            };
            let Some(id) = target else {
                return;
            };
            duplicate_and_refresh(&window, &mut s, &id);
        });
    }
}

/// Duplicate `id`, focus the copy (session resume), and refresh — shared by the
/// per-row ⧉ button and the Ctrl+D shortcut.
fn duplicate_and_refresh(window: &MainWindow, s: &mut UiState, id: &str) {
    match s
        .runtime
        .block_on(async { duplicate_plan_line(s.app.repo(), id).await })
    {
        Ok(new_id) => {
            s.plan_last_edited_id = new_id;
            window.set_plan_status_text(SharedString::from(""));
            window.set_plan_status_is_error(false);
        }
        Err(e) => {
            let msg = localize_app_error(s.app.i18n(), &e);
            window.set_plan_status_text(SharedString::from(msg));
            window.set_plan_status_is_error(true);
            return;
        }
    }
    if let Err(e) = refresh_plan(window, s) {
        tracing::error!(error = %e, "failed to refresh plan grid after duplicate");
    }
}
