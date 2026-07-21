//! Plantings screen (list + Gantt + create form) wiring — extracted from `main.rs` (story 0.3). Shared helpers
//! (`UiState`, refreshes, delete executors, error rendering) stay in the
//! crate root and are reached through `crate::…`.

use std::cell::RefCell;
use std::rc::Rc;

use chrono::Local;
use slint::{ComponentHandle, SharedString};

use pomone_app::{parse_id, plantings_view, services, AppError};
use pomone_domain::{LocationId, StrataId, VarietyId};

use crate::generated::MainWindow;
use crate::{
    i32_to_usize, open_planting_detail, refresh_plantings, render_form_error, validate_iso_date,
    validate_positive_count, validate_positive_decimal, FormError, UiState,
};

/// Register every plantings-screen callback on the window. Called once from
/// `main()`; standard wiring shape — see `wiring/mod.rs`.
pub(crate) fn wire_plantings(window: &MainWindow, state: &Rc<RefCell<UiState>>) {
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_navigate_plantings(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            if let Err(e) = refresh_plantings(&window, &mut state.borrow_mut()) {
                tracing::error!(error = %e, "failed to refresh plantings");
            }
            window.set_current_page(SharedString::from("plantings"));
            window.set_status_text(SharedString::from(""));
            window.set_status_is_error(false);
        });
    }

    // --- Plantings page callback ---
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_create_planting(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let mut s = state.borrow_mut();
            match try_create_planting(&window, &mut s) {
                Ok(notice) => {
                    let i18n = s.app.i18n();
                    // Retro-entering a pre-existing perennial replaces the bare
                    // "created" confirmation with the reassurance line: the
                    // avalanche fear is answered before it forms (story 3.4).
                    let text = notice.unwrap_or_else(|| i18n.t("status-planting-created"));
                    window.set_status_text(SharedString::from(text));
                    window.set_status_is_error(false);
                    if let Err(e) = refresh_plantings(&window, &mut s) {
                        tracing::error!(error = %e, "failed to refresh plantings after create");
                    }
                }
                Err(e) => {
                    let (text, is_err) = render_form_error(s.app.i18n(), e);
                    window.set_status_text(text);
                    window.set_status_is_error(is_err);
                }
            }
        });
    }
    // --- Planting row click → open detail ---
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_planting_row_clicked(move |pid| {
            let Some(window) = weak.upgrade() else {
                return;
            };
            open_planting_detail(&window, &mut state.borrow_mut(), &pid, "plantings");
        });
    }

    // --- Plantings table: click a column header to sort (toggle direction on
    //     the active column, else switch to the new column ascending) ---
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_plantings_sort(move |column| {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let mut s = state.borrow_mut();
            let column = column.to_string();
            if s.plantings_sort_column == column {
                s.plantings_sort_asc = !s.plantings_sort_asc;
            } else {
                s.plantings_sort_column = column;
                s.plantings_sort_asc = true;
            }
            if let Err(e) = refresh_plantings(&window, &mut s) {
                tracing::error!(error = %e, "failed to refresh plantings after sort");
            }
        });
    }
}

/// Read the form fields, validate them, build typed IDs, and call the right
/// service depending on whether the picked variety is annual or perennial.
/// Client-side validation surfaces localized messages; service-side errors
/// pass through unchanged.
///
/// Returns the retro-entry reassurance line when one applies (a perennial
/// established in the past, story 3.4) — `None` means the caller shows the
/// ordinary "planting created" confirmation.
fn try_create_planting(
    window: &MainWindow,
    state: &mut UiState,
) -> Result<Option<String>, FormError> {
    let i18n = state.app.i18n();
    // The clock is read here, at the UI edge, and injected downwards (AR12).
    // It drives the retro-entry cutoff: a perennial established in the past
    // generates no past-dated task (story 3.4).
    let today = Local::now().date_naive();
    let variety_idx = i32_to_usize(window.get_variety_index());
    let location_idx = i32_to_usize(window.get_location_index());
    let strata_idx = i32_to_usize(window.get_strata_index());
    let variety_id_str = state
        .variety_ids
        .get(variety_idx)
        .ok_or_else(|| FormError::Service(AppError::Inconsistent("no variety selected".into())))?;
    let location_id_str = state
        .location_ids
        .get(location_idx)
        .ok_or_else(|| FormError::Service(AppError::Inconsistent("no location selected".into())))?;
    let strata_id_str = state
        .strata_ids
        .get(strata_idx)
        .ok_or_else(|| FormError::Service(AppError::Inconsistent("no strata selected".into())))?;
    let is_annual = state
        .variety_is_annuals_plantings
        .get(variety_idx)
        .copied()
        .unwrap_or(true);

    let variety_id: VarietyId = parse_id(variety_id_str).map_err(FormError::Service)?;
    let location_id: LocationId = parse_id(location_id_str).map_err(FormError::Service)?;
    let strata_id: StrataId = parse_id(strata_id_str).map_err(FormError::Service)?;
    // The form field is in the display unit (issue #29); storage stays m².
    let area_m2 = state
        .app
        .area_unit()
        .to_m2(validate_positive_decimal(&window.get_area_text(), i18n)?);
    let plants_count = validate_positive_count(&window.get_count_text(), i18n)?;

    let notice = if is_annual {
        // The sown-on field holds the sowing date (direct / raised) or the
        // planting date (bought plants), per the chosen establishment method.
        let date = validate_iso_date(&window.get_sown_on_text(), i18n)?;
        let method = establishment_method_from_index(window.get_planting_method_index());
        state.runtime.block_on(async {
            services::create_annual_planting(
                state.app.repo(),
                services::AnnualPlantingRequest::from_sowing(
                    variety_id,
                    location_id,
                    strata_id,
                    date,
                    area_m2,
                    plants_count,
                )
                .with_method(method),
                today,
            )
            .await
            .map(|_| None)
        })?
    } else {
        let established_on = validate_iso_date(&window.get_established_on_text(), i18n)?;
        let removal_text = window.get_removal_on_text();
        let expected_removal_on = if removal_text.trim().is_empty() {
            None
        } else {
            Some(validate_iso_date(&removal_text, i18n)?)
        };
        state.runtime.block_on(async {
            let mut request = services::PerennialPlantingRequest::new(
                variety_id,
                location_id,
                strata_id,
                established_on,
                area_m2,
                plants_count,
            );
            if let Some(removal) = expected_removal_on {
                request = request.with_expected_removal(removal);
            }
            let planting =
                services::create_perennial_planting(state.app.repo(), request, today).await?;
            plantings_view::retro_entry_notice(state.app.repo(), state.app.i18n(), &planting, today)
                .await
        })?
    };
    Ok(notice)
}

/// Map the establishment-method dropdown index to the service enum. Order must
/// match the `planting-method-labels` model built in `refresh_i18n`.
fn establishment_method_from_index(idx: i32) -> services::EstablishmentMethod {
    match idx {
        1 => services::EstablishmentMethod::RaisedTransplant,
        2 => services::EstablishmentMethod::BoughtPlants,
        _ => services::EstablishmentMethod::DirectSow,
    }
}
