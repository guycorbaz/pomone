//! Shared confirmation dialog — dispatches every pending destructive action wiring — extracted from `main.rs` (story 0.4). Shared
//! helpers stay reachable through `crate::…` re-exports.

use std::cell::RefCell;
use std::rc::Rc;

use slint::{ComponentHandle, SharedString};

use pomone_app::{
    delete_crop, delete_family, delete_location, delete_strata, delete_task, delete_task_type,
    delete_treatment, delete_variety, services, AppError,
};
use pomone_domain::PlantingId;

use crate::{
    localize_app_error, parse_id, refresh_after_task_form, refresh_bed_usage, refresh_cultures,
    refresh_families, refresh_locations, refresh_planting_detail, refresh_plantings,
    refresh_strata, refresh_task_calendar, refresh_task_types, refresh_varieties_of_selected_crop,
    render_family_form_error, render_form_error, render_task_form_error,
    render_task_type_form_error, reset_crop_form_to_create, reset_families_form_to_create,
    reset_task_types_form_to_create, reset_variety_form_to_create, FormError, PendingDelete,
    UiState,
};

use crate::generated::MainWindow;

/// Register the confirm callbacks on the window. Called once
/// from `main()`; standard wiring shape — see `wiring/mod.rs`.
pub(crate) fn wire_confirm(window: &MainWindow, state: &Rc<RefCell<UiState>>) {
    // --- Confirmation dialog: run or drop the pending destructive action ---
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_confirm_accepted(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            window.set_confirm_visible(false);
            let mut s = state.borrow_mut();
            match s.pending_delete.take() {
                Some(PendingDelete::Strata(id)) => do_delete_strata(&window, &mut s, &id),
                Some(PendingDelete::Task(id)) => do_delete_task(&window, &mut s, &id),
                Some(PendingDelete::TaskType(id)) => do_delete_task_type(&window, &mut s, &id),
                Some(PendingDelete::Planting(id)) => do_delete_planting(&window, &mut s, &id),
                Some(PendingDelete::Crop(id)) => do_delete_crop(&window, &mut s, &id),
                Some(PendingDelete::Variety(id)) => do_delete_variety(&window, &mut s, &id),
                Some(PendingDelete::Location(id)) => do_delete_location(&window, &mut s, &id),
                Some(PendingDelete::Family(id)) => do_delete_family(&window, &mut s, &id),
                Some(PendingDelete::Treatment(id)) => do_delete_treatment_row(&window, &mut s, &id),
                Some(PendingDelete::ItkActivity(id)) => {
                    crate::wiring::itk::do_delete_itk_activity(&window, &mut s, &id);
                }
                None => {}
            }
        });
    }
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_confirm_cancelled(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            window.set_confirm_visible(false);
            state.borrow_mut().pending_delete = None;
        });
    }
}

/// Execute a confirmed crop deletion (issue #86 follow-up). On the FK guard
/// (`crop_in_use`) we show a localized message; otherwise we refresh and, if we
/// were editing the deleted crop, drop back to create mode.
fn do_delete_crop(window: &MainWindow, s: &mut UiState, id: &str) {
    let result: Result<(), AppError> = s
        .runtime
        .block_on(async { delete_crop(s.app.repo(), id).await });
    match result {
        Ok(()) => {
            window.set_status_text(SharedString::from(s.app.i18n().t("status-crop-deleted")));
            window.set_status_is_error(false);
            if s.editing_crop_id == id {
                reset_crop_form_to_create(window, s);
            }
            if window.get_selected_crop_index() >= 0 {
                window.set_selected_crop_index(-1);
            }
            if let Err(e) = refresh_cultures(window, s) {
                tracing::error!(error = %e, "failed to refresh cultures after crop delete");
            }
        }
        Err(AppError::Inconsistent(ref msg)) if msg == "crop_in_use" => {
            window.set_status_text(SharedString::from(s.app.i18n().t("error-crop-in-use")));
            window.set_status_is_error(true);
        }
        Err(e) => {
            let (text, is_err) = render_form_error(s.app.i18n(), FormError::Service(e));
            window.set_status_text(text);
            window.set_status_is_error(is_err);
        }
    }
}

/// Execute a confirmed variety deletion. Blocked (localized) when a planting
/// uses the variety; otherwise refresh the crop's variety list + the catalog
/// counts.
fn do_delete_variety(window: &MainWindow, s: &mut UiState, id: &str) {
    let result: Result<(), AppError> = s
        .runtime
        .block_on(async { delete_variety(s.app.repo(), id).await });
    match result {
        Ok(()) => {
            window.set_status_text(SharedString::from(s.app.i18n().t("status-variety-deleted")));
            window.set_status_is_error(false);
            if s.editing_variety_id == id {
                reset_variety_form_to_create(window, s);
            }
            if let Err(e) = refresh_varieties_of_selected_crop(window, s) {
                tracing::error!(error = %e, "failed to refresh varieties after delete");
            }
            // Variety counts on the crop rows change too.
            if let Err(e) = refresh_cultures(window, s) {
                tracing::error!(error = %e, "failed to refresh cultures after variety delete");
            }
        }
        Err(AppError::Inconsistent(ref msg)) if msg == "variety_in_use" => {
            window.set_status_text(SharedString::from(s.app.i18n().t("error-variety-in-use")));
            window.set_status_is_error(true);
        }
        Err(e) => {
            let (text, is_err) = render_form_error(s.app.i18n(), FormError::Service(e));
            window.set_status_text(text);
            window.set_status_is_error(is_err);
        }
    }
}

/// Execute a confirmed location deletion (Lieux screen). Blocked (localized)
/// when the location holds child locations or plantings; otherwise refresh the
/// tree.
fn do_delete_location(window: &MainWindow, s: &mut UiState, id: &str) {
    let result: Result<(), AppError> = s
        .runtime
        .block_on(async { delete_location(s.app.repo(), id).await });
    match result {
        Ok(()) => {
            window.set_status_text(SharedString::from(
                s.app.i18n().t("status-location-deleted"),
            ));
            window.set_status_is_error(false);
            if let Err(e) = refresh_locations(window, s) {
                tracing::error!(error = %e, "failed to refresh locations after delete");
            }
        }
        Err(AppError::Inconsistent(ref msg)) if msg == "location_in_use" => {
            window.set_status_text(SharedString::from(s.app.i18n().t("error-location-in-use")));
            window.set_status_is_error(true);
        }
        Err(e) => {
            let (text, is_err) = render_form_error(s.app.i18n(), FormError::Service(e));
            window.set_status_text(text);
            window.set_status_is_error(is_err);
        }
    }
}

/// Execute a confirmed strata deletion (issue #61) — the body that used to run
/// inline in `on_delete_strata`, now gated behind the confirmation dialog.
fn do_delete_strata(window: &MainWindow, s: &mut UiState, id: &str) {
    let result: Result<(), AppError> = s
        .runtime
        .block_on(async { delete_strata(s.app.repo(), id).await });
    match result {
        Ok(()) => {
            window.set_strata_status_text(SharedString::from(
                s.app.i18n().t("status-strata-deleted"),
            ));
            window.set_strata_status_is_error(false);
            if let Err(e) = refresh_strata(window, s) {
                tracing::error!(error = %e, "failed to refresh strata after delete");
            }
            refresh_bed_usage(window, &s.app, &s.runtime);
        }
        Err(e) => {
            let (text, is_err) = render_form_error(s.app.i18n(), FormError::Service(e));
            window.set_strata_status_text(text);
            window.set_strata_status_is_error(is_err);
        }
    }
}

/// Execute a confirmed task deletion (issue #61), routing back to the page the
/// task form was opened from.
fn do_delete_task(window: &MainWindow, s: &mut UiState, task_id: &str) {
    let result = s
        .runtime
        .block_on(async { delete_task(s.app.repo(), task_id).await });
    match result {
        Ok(()) => {
            let prev = s.task_form_previous_page.clone();
            window.set_current_page(SharedString::from(prev.clone()));
            refresh_after_task_form(window, s, &prev);
        }
        Err(e) => {
            let (text, is_err) = render_task_form_error(s.app.i18n(), FormError::Service(e));
            window.set_task_form_status_text(text);
            window.set_task_form_status_is_error(is_err);
        }
    }
}

/// Execute a confirmed task-type deletion (issue #61).
fn do_delete_task_type(window: &MainWindow, s: &mut UiState, id: &str) {
    let result = s
        .runtime
        .block_on(async { delete_task_type(s.app.repo(), id).await });
    match result {
        Ok(()) => {
            if let Err(e) = refresh_task_types(window, s) {
                tracing::error!(error = %e, "failed to refresh task types after delete");
            }
            // If we were editing the type that just got deleted, drop to create mode.
            if s.editing_task_type_id == id {
                reset_task_types_form_to_create(window, s);
            }
        }
        Err(e) => {
            let (text, is_err) = render_task_type_form_error(s.app.i18n(), FormError::Service(e));
            window.set_task_types_status_text(text);
            window.set_task_types_status_is_error(is_err);
        }
    }
}

/// Execute a confirmed planting deletion (issue #63). The service refuses if
/// the planting carries real activity, so on success we return to the list and
/// on the activity guard we surface the localized message in the detail's
/// life-cycle status line (the planting stays open).
fn do_delete_planting(window: &MainWindow, s: &mut UiState, id: &str) {
    let result: Result<(), AppError> = s.runtime.block_on(async {
        let pid: PlantingId = parse_id(id)?;
        services::delete_planting(s.app.repo(), pid).await
    });
    match result {
        Ok(()) => {
            let target = s.detail_previous_page.clone();
            if target == "tasks" {
                if let Err(e) = refresh_task_calendar(window, s) {
                    tracing::error!(error = %e, "refresh calendar after planting delete");
                }
            } else if let Err(e) = refresh_plantings(window, s) {
                tracing::error!(error = %e, "refresh plantings after planting delete");
            }
            refresh_bed_usage(window, &s.app, &s.runtime);
            window.set_current_page(SharedString::from(target));
            window.set_status_text(SharedString::from(
                s.app.i18n().t("status-planting-deleted"),
            ));
            window.set_status_is_error(false);
        }
        Err(e) => {
            let msg = localize_app_error(s.app.i18n(), &e);
            window.set_detail_lifecycle_status_text(SharedString::from(msg));
            window.set_detail_lifecycle_status_is_error(true);
        }
    }
}

/// Execute a confirmed treatment deletion (issue #82) and repaint the detail
/// page. Named `_row` to avoid clashing with the app-layer `delete_treatment`.
fn do_delete_treatment_row(window: &MainWindow, s: &mut UiState, id: &str) {
    let result: Result<(), AppError> = s
        .runtime
        .block_on(async { delete_treatment(s.app.repo(), id).await });
    match result {
        Ok(()) => {
            let pid = s.detail_planting_id.clone();
            // Refresh first — it clears the status banners (see the record
            // callback), so the success message must be set afterwards.
            if let Err(e) = refresh_planting_detail(window, s, &pid) {
                tracing::error!(error = %e, "failed to refresh detail after treatment delete");
            }
            window.set_treatment_status_text(SharedString::from(
                s.app.i18n().t("status-treatment-deleted"),
            ));
            window.set_treatment_status_is_error(false);
        }
        Err(e) => {
            let (text, is_err) = render_form_error(s.app.i18n(), FormError::Service(e));
            window.set_treatment_status_text(text);
            window.set_treatment_status_is_error(is_err);
        }
    }
}

/// Execute a confirmed family deletion.
fn do_delete_family(window: &MainWindow, s: &mut UiState, id: &str) {
    let result = s
        .runtime
        .block_on(async { delete_family(s.app.repo(), id).await });
    match result {
        Ok(()) => {
            if let Err(e) = refresh_families(window, s) {
                tracing::error!(error = %e, "failed to refresh families after delete");
            }
            if s.editing_family_id == id {
                reset_families_form_to_create(window, s);
            }
        }
        Err(e) => {
            let (text, is_err) = render_family_form_error(s.app.i18n(), FormError::Service(e));
            window.set_families_status_text(text);
            window.set_families_status_is_error(is_err);
        }
    }
}
