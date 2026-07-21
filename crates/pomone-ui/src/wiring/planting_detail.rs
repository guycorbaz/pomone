//! Planting-detail screen (status, harvests, treatments) wiring — extracted from `main.rs` (story 0.3). Shared helpers
//! (`UiState`, refreshes, delete executors, error rendering) stay in the
//! crate root and are reached through `crate::…`.

use std::cell::RefCell;
use std::rc::Rc;

use slint::{ComponentHandle, SharedString};

use pomone_app::{parse_id, services, AppError};
use pomone_domain::{PlantingId, PlantingStatus};

use crate::generated::MainWindow;
use crate::{
    localize_app_error, open_task_form_for_edit, optional_text, refresh_planting_detail,
    refresh_plantings, refresh_task_calendar, render_form_error, validate_iso_date,
    validate_optional_decimal, validate_positive_decimal, validate_required_name, validate_year,
    FormError, PendingDelete, UiState,
};

/// Register every planting-detail-screen callback on the window. Called once from
/// `main()`; standard wiring shape — see `wiring/mod.rs`.
#[allow(clippy::too_many_lines)]
pub(crate) fn wire_planting_detail(window: &MainWindow, state: &Rc<RefCell<UiState>>) {
    // --- Record yearly harvest from the detail screen ---
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_record_harvest(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let mut s = state.borrow_mut();
            match try_record_harvest(&window, &mut s) {
                Ok(()) => {
                    let i18n = s.app.i18n();
                    window.set_harvest_status_text(SharedString::from(
                        i18n.t("status-harvest-recorded"),
                    ));
                    window.set_harvest_status_is_error(false);
                    window.set_new_harvest_year(SharedString::from(""));
                    window.set_new_harvest_expected(SharedString::from(""));
                    window.set_new_harvest_actual(SharedString::from(""));
                    window.set_new_harvest_notes(SharedString::from(""));
                    let pid = s.detail_planting_id.clone();
                    if let Err(e) = refresh_planting_detail(&window, &mut s, &pid) {
                        tracing::error!(error = %e, "failed to refresh detail after harvest");
                    }
                }
                Err(e) => {
                    let (text, is_err) = render_form_error(s.app.i18n(), e);
                    window.set_harvest_status_text(text);
                    window.set_harvest_status_is_error(is_err);
                }
            }
        });
    }

    // --- Record a phytosanitary treatment from the detail screen (issue #82) ---
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_record_treatment(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let mut s = state.borrow_mut();
            match try_record_treatment(&window, &mut s) {
                Ok(()) => {
                    window.set_new_treatment_date(SharedString::from(""));
                    window.set_new_treatment_substance(SharedString::from(""));
                    window.set_new_treatment_product(SharedString::from(""));
                    window.set_new_treatment_dose(SharedString::from(""));
                    window.set_new_treatment_unit(SharedString::from(""));
                    window.set_new_treatment_notes(SharedString::from(""));
                    let pid = s.detail_planting_id.clone();
                    // Refresh first: it clears the status banners, so the
                    // success message must be set afterwards to survive.
                    if let Err(e) = refresh_planting_detail(&window, &mut s, &pid) {
                        tracing::error!(error = %e, "failed to refresh detail after treatment");
                    }
                    window.set_treatment_status_text(SharedString::from(
                        s.app.i18n().t("status-treatment-recorded"),
                    ));
                    window.set_treatment_status_is_error(false);
                }
                Err(e) => {
                    let (text, is_err) = render_form_error(s.app.i18n(), e);
                    window.set_treatment_status_text(text);
                    window.set_treatment_status_is_error(is_err);
                }
            }
        });
    }

    // --- Delete a treatment row (goes through the shared confirm dialog) ---
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_delete_treatment(move |id| {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let mut s = state.borrow_mut();
            s.pending_delete = Some(PendingDelete::Treatment(id.to_string()));
            window.set_confirm_message(SharedString::from(
                s.app.i18n().t("confirm-delete-treatment"),
            ));
            window.set_confirm_visible(true);
        });
    }

    // --- Detail "Back" button ---
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_detail_go_back(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let mut s = state.borrow_mut();
            let target = s.detail_previous_page.clone();
            // Refresh the destination so it reflects any change made while
            // browsing the detail. The unified calendar ("tasks") is the only
            // non-plantings origin (milestone click); everything else lands on
            // the plantings list.
            if target == "tasks" {
                if let Err(e) = refresh_task_calendar(&window, &mut s) {
                    tracing::error!(error = %e, "refresh calendar on back");
                }
            } else if let Err(e) = refresh_plantings(&window, &mut s) {
                tracing::error!(error = %e, "refresh plantings on back");
            }
            window.set_current_page(SharedString::from(target));
            window.set_status_text(SharedString::from(""));
            window.set_status_is_error(false);
        });
    }

    // --- Detail: change planting life-cycle status (issue #63) ---
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_detail_change_status(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let mut s = state.borrow_mut();
            let id = s.detail_planting_id.clone();
            let status = status_from_index(window.get_detail_status_index());
            // A terminal status carries the date the planting stopped occupying
            // its ground; going back to Active clears it (story 3.4, FR24/FR26).
            // An unparseable date is refused here rather than silently dropped —
            // a missing date would leave a dead planting on the capacity curve.
            let terminated_on = if status == PlantingStatus::Active {
                None
            } else {
                match validate_iso_date(&window.get_detail_terminated_on_text(), s.app.i18n()) {
                    Ok(date) => Some(date),
                    Err(e) => {
                        let (text, is_err) = render_form_error(s.app.i18n(), e);
                        window.set_detail_lifecycle_status_text(text);
                        window.set_detail_lifecycle_status_is_error(is_err);
                        return;
                    }
                }
            };
            let result: Result<(), AppError> = s.runtime.block_on(async {
                let pid: PlantingId = parse_id(&id)?;
                services::set_planting_status(s.app.repo(), pid, status, terminated_on).await
            });
            match result {
                Ok(()) => {
                    if let Err(e) = refresh_planting_detail(&window, &mut s, &id) {
                        tracing::error!(error = %e, "refresh detail after status change");
                    }
                    window.set_detail_lifecycle_status_text(SharedString::from(
                        s.app.i18n().t("status-planting-status-updated"),
                    ));
                    window.set_detail_lifecycle_status_is_error(false);
                }
                Err(e) => {
                    let msg = localize_app_error(s.app.i18n(), &e);
                    window.set_detail_lifecycle_status_text(SharedString::from(msg));
                    window.set_detail_lifecycle_status_is_error(true);
                }
            }
        });
    }

    // --- Detail: request planting deletion (issue #63) ---
    // Routed through the shared confirmation dialog; the activity guard lives
    // in the service, so a planting with history is refused with a localized
    // message rather than silently wiped.
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_detail_delete_planting(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let mut s = state.borrow_mut();
            let id = s.detail_planting_id.clone();
            s.pending_delete = Some(PendingDelete::Planting(id));
            window.set_confirm_message(SharedString::from(
                s.app.i18n().t("confirm-delete-planting"),
            ));
            window.set_confirm_visible(true);
        });
    }
    // Click on a task row in the planting-detail task list → open the same
    // edit form, but remember to route back to the detail page on save/cancel.
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_detail_task_clicked(move |task_id_str| {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let mut s = state.borrow_mut();
            "planting-detail".clone_into(&mut s.task_form_previous_page);
            if let Err(e) = open_task_form_for_edit(&window, &mut s, &task_id_str) {
                tracing::error!(error = %e, "failed to open task form from planting detail");
            }
        });
    }
}

/// Read the harvest form fields, validate them, then call the existing
/// `record_yearly_harvest` service. The form expects a year (required) and
/// optional expected/actual kg + notes; either yield being set is enough
/// to make the entry useful.
fn try_record_harvest(window: &MainWindow, state: &mut UiState) -> Result<(), FormError> {
    let i18n = state.app.i18n();
    if state.detail_planting_id.is_empty() {
        return Err(FormError::Service(AppError::Inconsistent(
            "no planting selected for harvest record".into(),
        )));
    }
    let planting_id: PlantingId =
        parse_id(&state.detail_planting_id).map_err(FormError::Service)?;
    let year = validate_year(&window.get_new_harvest_year(), i18n)?;
    // The yield fields are in the display unit (issue #29); storage stays kg.
    let mass_unit = state.app.mass_unit();
    let expected = validate_optional_decimal(&window.get_new_harvest_expected(), i18n)?
        .map(|v| mass_unit.to_kg(v));
    let actual = validate_optional_decimal(&window.get_new_harvest_actual(), i18n)?
        .map(|v| mass_unit.to_kg(v));
    let notes = optional_text(&window.get_new_harvest_notes());

    state
        .runtime
        .block_on(async {
            let mut request = services::YearlyHarvestRequest::new(planting_id, year);
            if let Some(expected) = expected {
                request = request.with_expected_yield(expected);
            }
            if let Some(actual) = actual {
                request = request.with_actual_yield(actual);
            }
            if let Some(notes) = notes {
                request = request.with_notes(notes);
            }
            services::record_yearly_harvest(state.app.repo(), request)
                .await
                .map(|_| ())
        })
        .map_err(FormError::Service)
}

/// Validate + submit the "record a treatment" form on the detail page
/// (issue #82). Every field except notes is required.
fn try_record_treatment(window: &MainWindow, state: &mut UiState) -> Result<(), FormError> {
    let i18n = state.app.i18n();
    if state.detail_planting_id.is_empty() {
        return Err(FormError::Service(AppError::Inconsistent(
            "no planting selected for treatment record".into(),
        )));
    }
    let planting_id: PlantingId =
        parse_id(&state.detail_planting_id).map_err(FormError::Service)?;
    let applied_on = validate_iso_date(&window.get_new_treatment_date(), i18n)?;
    let substance = validate_required_name(&window.get_new_treatment_substance(), i18n)?;
    let product = validate_required_name(&window.get_new_treatment_product(), i18n)?;
    let dose = validate_positive_decimal(&window.get_new_treatment_dose(), i18n)?;
    let unit = validate_required_name(&window.get_new_treatment_unit(), i18n)?;
    let notes = optional_text(&window.get_new_treatment_notes());

    state
        .runtime
        .block_on(async {
            let mut request = services::TreatmentRequest::new(
                planting_id,
                applied_on,
                substance,
                product,
                dose,
                unit,
            );
            if let Some(notes) = notes {
                request = request.with_notes(notes);
            }
            services::record_treatment(state.app.repo(), request)
                .await
                .map(|_| ())
        })
        .map_err(FormError::Service)
}

/// Map the life-cycle status combo index to a [`PlantingStatus`] (issue #63).
/// Parallel to the `detail-status-options` model built at startup. Out-of-range
/// indices fall back to `Active`.
fn status_from_index(index: i32) -> PlantingStatus {
    match index {
        1 => PlantingStatus::Completed,
        2 => PlantingStatus::Failed,
        3 => PlantingStatus::Abandoned,
        _ => PlantingStatus::Active,
    }
}
