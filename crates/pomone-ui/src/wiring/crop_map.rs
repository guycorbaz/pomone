//! Crop-map screen (lanes, move picker, split dialog) wiring — extracted
//! from `main.rs` (story 0.3). This screen's refresh lives here
//! (`refresh_crop_map` has no cross-screen callers); other shared helpers
//! (`UiState`, error rendering) stay in the crate root, via `crate::…`.

use std::cell::RefCell;
use std::rc::Rc;
use std::str::FromStr;

use anyhow::{Context, Result};
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

use fluent::FluentArgs;
use pomone_app::{
    list_crop_map_lanes, move_planting_to_location, parse_id, split_planting,
    CropMapBar as AppCropMapBar, CropMapLane as AppCropMapLane, SplitPart,
};
use pomone_domain::PlantingId;
use rust_decimal::Decimal;

use crate::generated::{
    CropMapBarItem as SlintCropMapBar, CropMapLaneItem as SlintCropMapLane,
    CropMapLocationOption as SlintCropMapLocationOption, MainWindow,
};
use crate::{localize_app_error, parse_hex_color, render_form_error, FormError, UiState};

/// Register every crop-map-screen callback on the window. Called once from
/// `main()`; standard wiring shape — see `wiring/mod.rs`.
#[allow(clippy::too_many_lines)]
pub(crate) fn wire_crop_map(window: &MainWindow, state: &Rc<RefCell<UiState>>) {
    // --- Crop Map navigation + selection / move / split callbacks ---
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_navigate_crop_map(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let mut s = state.borrow_mut();
            if let Err(e) = refresh_crop_map(&window, &mut s) {
                tracing::error!(error = %e, "failed to refresh crop map");
            }
            window.set_current_page(SharedString::from("crop-map"));
            window.set_crop_map_selected_planting_id(SharedString::from(""));
            window.set_crop_map_move_picker_visible(false);
            window.set_crop_map_split_form_visible(false);
            window.set_crop_map_split_status_text(SharedString::from(""));
        });
    }
    {
        let weak = window.as_weak();
        window.on_crop_map_bar_clicked(move |pid| {
            let Some(window) = weak.upgrade() else {
                return;
            };
            // Toggle: clicking the same bar deselects.
            let current = window.get_crop_map_selected_planting_id();
            if current.as_str() == pid.as_str() {
                window.set_crop_map_selected_planting_id(SharedString::from(""));
            } else {
                window.set_crop_map_selected_planting_id(pid);
            }
        });
    }
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_crop_map_move_to(move |pid, lid| {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let mut s = state.borrow_mut();
            let result = s
                .runtime
                .block_on(async { move_planting_to_location(s.app.repo(), &pid, &lid).await });
            if let Err(e) = result {
                tracing::error!(error = %e, "failed to move planting");
                let mut args = FluentArgs::new();
                args.set("message", localize_app_error(s.app.i18n(), &e));
                window.set_status_text(SharedString::from(
                    s.app.i18n().t_args("status-planting-failed", &args),
                ));
                window.set_status_is_error(true);
                return;
            }
            if let Err(e) = refresh_crop_map(&window, &mut s) {
                tracing::error!(error = %e, "failed to refresh crop map after move");
            }
            window.set_crop_map_selected_planting_id(SharedString::from(""));
        });
    }
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_crop_map_split_clicked(move |pid| {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let s = state.borrow();
            // Pre-fill the split form with a 50/50 default + the source's
            // current location in part A, the next location in the list
            // for part B (so the user only needs to confirm in the
            // happy case).
            if let Err(e) = prefill_split_form(&window, &s, &pid) {
                tracing::warn!(error = %e, "failed to prefill split form");
            }
        });
    }
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_crop_map_split_confirm(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let mut s = state.borrow_mut();
            match try_confirm_split(&window, &mut s) {
                Ok(()) => {
                    window.set_crop_map_split_form_visible(false);
                    window.set_crop_map_selected_planting_id(SharedString::from(""));
                    if let Err(e) = refresh_crop_map(&window, &mut s) {
                        tracing::error!(error = %e, "failed to refresh crop map after split");
                    }
                }
                Err(e) => {
                    let (text, is_err) = render_form_error(s.app.i18n(), e);
                    window.set_crop_map_split_status_text(text);
                    window.set_crop_map_split_status_is_error(is_err);
                }
            }
        });
    }
    {
        let weak = window.as_weak();
        window.on_crop_map_split_cancel(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            window.set_crop_map_split_form_visible(false);
            window.set_crop_map_split_status_text(SharedString::from(""));
        });
    }
}

/// Push the Crop Map data to Slint: lanes + month labels + parallel
/// `(label, id)` table for the move-picker and split-form ComboBoxes.
fn refresh_crop_map(window: &MainWindow, state: &mut UiState) -> Result<()> {
    let lanes: Vec<AppCropMapLane> = state
        .runtime
        .block_on(async { list_crop_map_lanes(state.app.repo()).await })
        .context("failed to load crop map")?;

    // Parallel id table — same ordering as the lanes so move-picker /
    // split ComboBoxes are interchangeable. We also derive the label
    // model from the same list.
    state.crop_map_location_ids = lanes.iter().map(|l| l.location_id.clone()).collect();
    let move_targets: Vec<SlintCropMapLocationOption> = lanes
        .iter()
        .map(|l| SlintCropMapLocationOption {
            location_id: SharedString::from(l.location_id.clone()),
            label: SharedString::from(l.label.clone()),
        })
        .collect();
    window.set_crop_map_move_target_options(ModelRc::new(VecModel::from(move_targets)));
    let split_labels: Vec<SharedString> = lanes
        .iter()
        .map(|l| SharedString::from(l.label.clone()))
        .collect();
    window.set_crop_map_split_target_labels(ModelRc::new(VecModel::from(split_labels)));

    let slint_lanes: Vec<SlintCropMapLane> = lanes
        .into_iter()
        .map(|l| SlintCropMapLane {
            location_id: SharedString::from(l.location_id),
            label: SharedString::from(l.label),
            dimensions_label: SharedString::from(l.dimensions_label),
            bars: ModelRc::new(VecModel::from(
                l.bars.into_iter().map(bar_to_slint).collect::<Vec<_>>(),
            )),
        })
        .collect();
    window.set_crop_map_lanes(ModelRc::new(VecModel::from(slint_lanes)));

    // Month labels — re-use the Gantt translations so the season axis
    // stays consistent across screens.
    let i18n = state.app.i18n();
    let months: Vec<SharedString> = (1..=12)
        .map(|m| SharedString::from(i18n.t(&format!("gantt-month-{m}"))))
        .collect();
    window.set_crop_map_month_labels(ModelRc::new(VecModel::from(months)));
    Ok(())
}

fn bar_to_slint(b: AppCropMapBar) -> SlintCropMapBar {
    SlintCropMapBar {
        planting_id: SharedString::from(b.planting_id),
        label: SharedString::from(b.label),
        color: parse_hex_color(&b.color_hex),
        start_doy: b.start_doy,
        end_doy: b.end_doy,
    }
}

/// Pre-fill the split form with sensible defaults so the happy path is
/// a single Confirm click: part A = source's current location with half
/// the area+count; part B = next location in the list (cycles back to
/// the first if the source is the last one) with the other half.
fn prefill_split_form(window: &MainWindow, state: &UiState, planting_id: &str) -> Result<()> {
    let p_id: PlantingId = parse_id(planting_id)?;
    let planting = state
        .runtime
        .block_on(async { state.app.repo().planting_get(p_id).await })?
        .context("planting referenced by the split form vanished")?;
    let source_location_str = planting.location_id.to_string();
    let source_idx = state
        .crop_map_location_ids
        .iter()
        .position(|id| id == &source_location_str)
        .map_or(0, |i| i32::try_from(i).unwrap_or(0));
    // Pick a *different* location for part B when possible.
    let next_idx = if state.crop_map_location_ids.len() > 1 {
        let n = state.crop_map_location_ids.len();
        let i = usize::try_from(source_idx).unwrap_or(0);
        i32::try_from((i + 1) % n).unwrap_or(0)
    } else {
        source_idx
    };
    // Prefill in the display unit (issue #29), mirroring the parse side in
    // `try_confirm_split`.
    let half_area = state
        .app
        .area_unit()
        .to_display(planting.area_m2 / Decimal::from(2));
    let half_count = planting.plants_count / 2;
    let remainder_count = planting.plants_count - half_count;

    window.set_crop_map_split_part_a_location_index(source_idx);
    window.set_crop_map_split_part_b_location_index(next_idx);
    window.set_crop_map_split_part_a_area(SharedString::from(half_area.normalize().to_string()));
    window.set_crop_map_split_part_b_area(SharedString::from(half_area.normalize().to_string()));
    window.set_crop_map_split_part_a_count(SharedString::from(half_count.to_string()));
    window.set_crop_map_split_part_b_count(SharedString::from(remainder_count.to_string()));
    window.set_crop_map_split_status_text(SharedString::from(""));
    window.set_crop_map_split_status_is_error(false);
    Ok(())
}

/// Validate the split form fields and call `split_planting`. Validation
/// errors are surfaced as `FormError::Validation` so the existing
/// `render_form_error` template picks them up.
fn try_confirm_split(window: &MainWindow, state: &mut UiState) -> Result<(), FormError> {
    let i18n = state.app.i18n();
    let pid = window.get_crop_map_selected_planting_id().to_string();
    if pid.is_empty() {
        return Err(FormError::Validation(i18n.t("error-no-planting-selected")));
    }
    let area_unit = state.app.area_unit();
    let part = |loc_idx: i32,
                area_text: SharedString,
                count_text: SharedString|
     -> Result<SplitPart, FormError> {
        let usize_idx = usize::try_from(loc_idx.max(0)).unwrap_or(0);
        let location_id = state
            .crop_map_location_ids
            .get(usize_idx)
            .cloned()
            .ok_or_else(|| FormError::Validation(i18n.t("error-location-required")))?;
        // The area field is in the display unit (issue #29); storage stays m².
        let area: Decimal = Decimal::from_str(area_text.trim())
            .map_err(|_| FormError::Validation(i18n.t("error-number-invalid")))?;
        let count: u32 = count_text
            .trim()
            .parse()
            .map_err(|_| FormError::Validation(i18n.t("error-number-invalid")))?;
        Ok(SplitPart {
            location_id,
            area_m2: area_unit.to_m2(area),
            plants_count: count,
        })
    };
    let part_a = part(
        window.get_crop_map_split_part_a_location_index(),
        window.get_crop_map_split_part_a_area(),
        window.get_crop_map_split_part_a_count(),
    )?;
    let part_b = part(
        window.get_crop_map_split_part_b_location_index(),
        window.get_crop_map_split_part_b_area(),
        window.get_crop_map_split_part_b_count(),
    )?;
    state
        .runtime
        .block_on(async { split_planting(state.app.repo(), &pid, &[part_a, part_b]).await })
        .map_err(FormError::Service)?;
    Ok(())
}
