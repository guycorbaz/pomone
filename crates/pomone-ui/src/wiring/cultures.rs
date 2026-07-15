//! Cultures screen (crops + varieties master-detail) wiring — extracted
//! from `main.rs` (story 0.2). Shared helpers (`UiState`, refreshes, delete
//! executors, error rendering) stay in the crate root, via `crate::…`.

use std::cell::RefCell;
use std::rc::Rc;

use anyhow::{Context, Result};
use slint::{ComponentHandle, SharedString};

use crate::generated::MainWindow;
use crate::{
    i32_to_usize, optional_text, parse_optional_decimal, parse_optional_u16, parse_u16, parse_u8,
    refresh_cultures, refresh_varieties_of_selected_crop, render_form_error,
    reset_crop_form_to_create, reset_variety_form_to_create, validate_required_name, FormError,
    PendingDelete, UiState,
};
use pomone_app::{
    create_crop, create_variety, get_crop_for_edit, get_variety_for_edit, update_crop,
    update_variety, AppError, CropEditForm, CropInput, LifespanKind, VarietyEditForm, VarietyInput,
    VarietyProfileKind,
};
use pomone_domain::PruningSeason;

/// Register every cultures-screen callback on the window. Called once from
/// `main()`; standard wiring shape — see `wiring/mod.rs`.
#[allow(clippy::too_many_lines)]
pub(crate) fn wire_cultures(window: &MainWindow, state: &Rc<RefCell<UiState>>) {
    // --- Cultures navigation ---
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_navigate_cultures(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            if let Err(e) = refresh_cultures(&window, &mut state.borrow_mut()) {
                tracing::error!(error = %e, "failed to refresh cultures");
            }
            window.set_current_page(SharedString::from("cultures"));
            window.set_status_text(SharedString::from(""));
            window.set_status_is_error(false);
        });
    }

    // --- Crop selection (master-detail) ---
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_select_crop(move |idx| {
            let Some(window) = weak.upgrade() else {
                return;
            };
            window.set_selected_crop_index(idx);
            let mut s = state.borrow_mut();
            // Update the bool that drives the variety form's conditional
            // rendering. Default to true (annual) if the index is out of
            // range — that matches the default form panel.
            let is_annual = s
                .crop_is_annuals
                .get(i32_to_usize(idx))
                .copied()
                .unwrap_or(true);
            window.set_selected_crop_is_annual(is_annual);
            if let Err(e) = refresh_varieties_of_selected_crop(&window, &mut s) {
                tracing::error!(error = %e, "failed to refresh varieties");
            }
            // The ITK editor follows the selected crop (story 2.5).
            if let Err(e) = crate::wiring::itk::refresh_itk(&window, &mut s) {
                tracing::error!(error = %e, "failed to refresh ITK editor");
            }
        });
    }

    // --- Create crop ---
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_create_crop(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let mut s = state.borrow_mut();
            let was_edit = window.get_crop_is_edit_mode();
            match try_save_crop(&window, &mut s) {
                Ok(()) => {
                    let key = if was_edit {
                        "status-crop-updated"
                    } else {
                        "status-crop-created"
                    };
                    window.set_status_text(SharedString::from(s.app.i18n().t(key)));
                    window.set_status_is_error(false);
                    // Back to a clean create form (also clears edit mode).
                    reset_crop_form_to_create(&window, &mut s);
                    if let Err(e) = refresh_cultures(&window, &mut s) {
                        tracing::error!(error = %e, "failed to refresh cultures after save");
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

    // --- Edit / delete / cancel-edit a crop (Cultures screen) ---
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_edit_crop(move |id| {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let mut s = state.borrow_mut();
            if let Err(e) = open_crop_form_for_edit(&window, &mut s, &id) {
                tracing::error!(error = %e, "failed to open crop edit form");
            }
        });
    }
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_delete_crop(move |id| {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let mut s = state.borrow_mut();
            s.pending_delete = Some(PendingDelete::Crop(id.to_string()));
            window.set_confirm_message(SharedString::from(s.app.i18n().t("confirm-delete-crop")));
            window.set_confirm_visible(true);
        });
    }
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_cancel_crop_edit(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            reset_crop_form_to_create(&window, &mut state.borrow_mut());
        });
    }
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_delete_variety(move |id| {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let mut s = state.borrow_mut();
            s.pending_delete = Some(PendingDelete::Variety(id.to_string()));
            window
                .set_confirm_message(SharedString::from(s.app.i18n().t("confirm-delete-variety")));
            window.set_confirm_visible(true);
        });
    }
    // --- Create variety ---
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_create_variety(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let mut s = state.borrow_mut();
            let was_edit = window.get_variety_is_edit_mode();
            match try_save_variety(&window, &mut s) {
                Ok(()) => {
                    let key = if was_edit {
                        "status-variety-updated"
                    } else {
                        "status-variety-created"
                    };
                    window.set_status_text(SharedString::from(s.app.i18n().t(key)));
                    window.set_status_is_error(false);
                    if was_edit {
                        // Back to a clean create form (also clears edit mode).
                        reset_variety_form_to_create(&window, &mut s);
                    } else {
                        // Keep the profile fields for rapid entry; only clear
                        // the name + description of the just-created variety.
                        window.set_new_variety_name(SharedString::from(""));
                        window.set_new_variety_description(SharedString::from(""));
                    }
                    // Refreshes the catalog counts and the crop's variety list.
                    if let Err(e) = refresh_cultures(&window, &mut s) {
                        tracing::error!(error = %e, "failed to refresh cultures after save");
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

    // --- Edit / cancel-edit a variety (Cultures screen) ---
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_edit_variety(move |id| {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let mut s = state.borrow_mut();
            if let Err(e) = open_variety_form_for_edit(&window, &mut s, &id) {
                tracing::error!(error = %e, "failed to open variety edit form");
            }
        });
    }
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_cancel_variety_edit(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            reset_variety_form_to_create(&window, &mut state.borrow_mut());
        });
    }
}

fn lifespan_kind_from_index(idx: i32) -> Result<LifespanKind, AppError> {
    match idx {
        0 => Ok(LifespanKind::Annual),
        1 => Ok(LifespanKind::PluriannualSingleCycle),
        2 => Ok(LifespanKind::PluriannualRecurring),
        other => Err(AppError::Inconsistent(format!(
            "unexpected lifespan dropdown index {other}"
        ))),
    }
}

fn pruning_from_index(idx: i32) -> Result<PruningSeason, AppError> {
    match idx {
        0 => Ok(PruningSeason::None),
        1 => Ok(PruningSeason::Winter),
        2 => Ok(PruningSeason::Summer),
        3 => Ok(PruningSeason::Both),
        other => Err(AppError::Inconsistent(format!(
            "unexpected pruning dropdown index {other}"
        ))),
    }
}

/// Build the `CropInput` from the crop form, then create or update depending
/// on the form's edit mode (the crop being edited is `state.editing_crop_id`).
fn try_save_crop(window: &MainWindow, state: &mut UiState) -> Result<(), FormError> {
    let i18n = state.app.i18n();
    let family_idx = i32_to_usize(window.get_family_index());
    let family_id_str = state
        .family_ids
        .get(family_idx)
        .ok_or_else(|| FormError::Service(AppError::Inconsistent("no family selected".into())))?
        .clone();
    let name = validate_required_name(&window.get_new_crop_name(), i18n)?;
    let latin_name = optional_text(&window.get_new_crop_latin());
    let lifespan_kind = lifespan_kind_from_index(window.get_new_crop_lifespan_index())
        .map_err(FormError::Service)?;
    let pruning_season =
        pruning_from_index(window.get_new_crop_pruning_index()).map_err(FormError::Service)?;
    // Only parse the pluriannual fields when they're actually needed — leaves
    // pristine defaults for the Annual case and gives clearer errors for the
    // other two.
    let (lifespan_years, years_to_first_yield) = match lifespan_kind {
        LifespanKind::Annual => (0, 0),
        LifespanKind::PluriannualSingleCycle => (
            parse_u8(&window.get_new_crop_lifespan_years(), "lifespan years")
                .map_err(FormError::Service)?,
            0,
        ),
        LifespanKind::PluriannualRecurring => (
            parse_u8(&window.get_new_crop_lifespan_years(), "lifespan years")
                .map_err(FormError::Service)?,
            parse_u8(
                &window.get_new_crop_years_to_first_yield(),
                "years to first yield",
            )
            .map_err(FormError::Service)?,
        ),
    };

    let input = CropInput {
        family_id_str,
        name,
        latin_name,
        lifespan_kind,
        lifespan_years,
        years_to_first_yield,
        pruning_season,
    };
    let editing_id = state.editing_crop_id.clone();
    state
        .runtime
        .block_on(async {
            if editing_id.is_empty() {
                create_crop(state.app.repo(), input).await.map(|_| ())
            } else {
                update_crop(state.app.repo(), &editing_id, input).await
            }
        })
        .map_err(FormError::Service)
}

/// Load one crop into the crop form and switch it to edit mode.
fn open_crop_form_for_edit(window: &MainWindow, state: &mut UiState, id: &str) -> Result<()> {
    let form: CropEditForm = state
        .runtime
        .block_on(async { get_crop_for_edit(state.app.repo(), id).await })
        .context("failed to load crop for edit")?;

    let family_idx = state
        .family_ids
        .iter()
        .position(|f| f == &form.family_id_str)
        .map_or(0, |i| i32::try_from(i).unwrap_or(0));
    let lifespan_idx = match form.lifespan_kind {
        LifespanKind::Annual => 0,
        LifespanKind::PluriannualSingleCycle => 1,
        LifespanKind::PluriannualRecurring => 2,
    };
    let pruning_idx = match form.pruning_season {
        PruningSeason::None => 0,
        PruningSeason::Winter => 1,
        PruningSeason::Summer => 2,
        PruningSeason::Both => 3,
    };

    state.editing_crop_id.clone_from(&form.id);
    window.set_crop_is_edit_mode(true);
    window.set_family_index(family_idx);
    window.set_new_crop_name(SharedString::from(form.name));
    window.set_new_crop_latin(SharedString::from(form.latin_name));
    window.set_new_crop_lifespan_index(lifespan_idx);
    window.set_new_crop_pruning_index(pruning_idx);
    window.set_new_crop_lifespan_years(SharedString::from(form.lifespan_years.to_string()));
    window.set_new_crop_years_to_first_yield(SharedString::from(
        form.years_to_first_yield.to_string(),
    ));
    window.set_status_text(SharedString::from(""));
    window.set_status_is_error(false);
    Ok(())
}

/// Build the `VarietyInput` from the variety form, then create or update
/// depending on the form's edit mode (the variety being edited is
/// `state.editing_variety_id`).
fn try_save_variety(window: &MainWindow, state: &mut UiState) -> Result<(), FormError> {
    let i18n = state.app.i18n();
    let idx = window.get_selected_crop_index();
    if idx < 0 {
        return Err(FormError::Service(AppError::Inconsistent(
            "no crop selected for variety create".into(),
        )));
    }
    let crop_id_str = state
        .crop_ids
        .get(i32_to_usize(idx))
        .ok_or_else(|| {
            FormError::Service(AppError::Inconsistent(
                "selected crop index out of range".into(),
            ))
        })?
        .clone();
    let name = validate_required_name(&window.get_new_variety_name(), i18n)?;
    let description = optional_text(&window.get_new_variety_description());
    let is_annual = window.get_selected_crop_is_annual();
    let profile_kind = if is_annual {
        VarietyProfileKind::Annual
    } else {
        VarietyProfileKind::Pluriannual
    };

    // Parse only the fields relevant to the chosen profile kind; the others
    // stay at zero/None and are ignored by the service.
    let mut input = VarietyInput {
        crop_id_str,
        name,
        description,
        profile_kind,
        days_to_transplant: None,
        days_to_maturity: 0,
        harvest_window_days: 0,
        bud_break_doy: None,
        flowering_doy: None,
        harvest_start_doy: 0,
        harvest_end_doy: 0,
        expected_yield_kg_per_plant: None,
    };
    if is_annual {
        input.days_to_transplant =
            parse_optional_u16(&window.get_new_variety_dtt(), "DTT").map_err(FormError::Service)?;
        input.days_to_maturity =
            parse_u16(&window.get_new_variety_dtm(), "DTM").map_err(FormError::Service)?;
        input.harvest_window_days = parse_u16(&window.get_new_variety_window(), "harvest window")
            .map_err(FormError::Service)?;
    } else {
        input.bud_break_doy =
            parse_optional_u16(&window.get_new_variety_bud_break_doy(), "bud break DOY")
                .map_err(FormError::Service)?;
        input.flowering_doy =
            parse_optional_u16(&window.get_new_variety_flowering_doy(), "flowering DOY")
                .map_err(FormError::Service)?;
        input.harvest_start_doy = parse_u16(
            &window.get_new_variety_harvest_start_doy(),
            "harvest start DOY",
        )
        .map_err(FormError::Service)?;
        input.harvest_end_doy =
            parse_u16(&window.get_new_variety_harvest_end_doy(), "harvest end DOY")
                .map_err(FormError::Service)?;
        input.expected_yield_kg_per_plant =
            parse_optional_decimal(&window.get_new_variety_yield_kg(), "yield")
                .map_err(FormError::Service)?;
    }

    let editing_id = state.editing_variety_id.clone();
    state
        .runtime
        .block_on(async {
            if editing_id.is_empty() {
                create_variety(state.app.repo(), input).await.map(|_| ())
            } else {
                update_variety(state.app.repo(), &editing_id, input).await
            }
        })
        .map_err(FormError::Service)
}

/// Load one variety into the variety form and switch it to edit mode. The form
/// panel shown (annual vs pluriannual) is driven by the selected crop, which
/// already owns this variety, so only the field values need prefilling.
fn open_variety_form_for_edit(window: &MainWindow, state: &mut UiState, id: &str) -> Result<()> {
    let form: VarietyEditForm = state
        .runtime
        .block_on(async { get_variety_for_edit(state.app.repo(), id).await })
        .context("failed to load variety for edit")?;

    state.editing_variety_id.clone_from(&form.id);
    window.set_variety_is_edit_mode(true);
    window.set_new_variety_name(SharedString::from(form.name));
    window.set_new_variety_description(SharedString::from(form.description));
    if form.is_annual {
        window.set_new_variety_dtt(SharedString::from(form.days_to_transplant));
        window.set_new_variety_dtm(SharedString::from(form.days_to_maturity));
        window.set_new_variety_window(SharedString::from(form.harvest_window_days));
    } else {
        window.set_new_variety_bud_break_doy(SharedString::from(form.bud_break_doy));
        window.set_new_variety_flowering_doy(SharedString::from(form.flowering_doy));
        window.set_new_variety_harvest_start_doy(SharedString::from(form.harvest_start_doy));
        window.set_new_variety_harvest_end_doy(SharedString::from(form.harvest_end_doy));
        window.set_new_variety_yield_kg(SharedString::from(form.expected_yield_kg_per_plant));
    }
    window.set_status_text(SharedString::from(""));
    window.set_status_is_error(false);
    Ok(())
}
