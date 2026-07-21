//! Wiring for the placement screen + live capacity curve (story 3.2).
//!
//! Left: the unplaced planned successions. Middle: the bed tree + a small form
//! (strata, plant count) and the Place / Undo buttons. Right: the occupancy
//! curve (sheltered vs open-field), its peak, and — when a peak is clicked —
//! the series composing it. Every DB call goes through `state.runtime.block_on`
//! and `state.app.repo()`; all view logic lives in `pomone_app::capacity_view`.

use std::cell::RefCell;
use std::rc::Rc;

use chrono::{Datelike, Local};
use fluent::FluentArgs;
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

use crate::generated::{
    MainWindow, PlacementBedRow as SlintBed, PlacementCompositionRow as SlintComp,
    PlacementUnplacedRow as SlintUnplaced,
};
use crate::{localize_app_error, UiState};
use pomone_app::capacity_view::{occupancy_curve, peak_composition, unplaced_list, OccupancyCurve};
use pomone_app::locations_view::list_locations_tree;
use pomone_app::plantings_view::parse_id;
use pomone_app::services::{place_planned_planting, unplace_planned_planting, PlacementRequest};
use pomone_app::{list_strata_options, AppError};

/// Plot dimensions — must match the `viewbox-*` of the `Path` in `placement.slint`.
const PLOT_W: f32 = 320.0;
const PLOT_H: f32 = 160.0;

fn season_year() -> i32 {
    Local::now().date_naive().year()
}

pub(crate) fn wire_placement(window: &MainWindow, state: &Rc<RefCell<UiState>>) {
    // Navigate → refresh + show.
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_navigate_placement(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            refresh_placement(&window, &mut state.borrow_mut());
            window.set_current_page(SharedString::from("placement"));
        });
    }

    // Select an unplaced succession (highlight).
    {
        let weak = window.as_weak();
        window.on_placement_select_unplaced(move |id| {
            if let Some(window) = weak.upgrade() {
                window.set_placement_selected_unplaced_id(id);
            }
        });
    }

    // Select a bed (highlight).
    {
        let weak = window.as_weak();
        window.on_placement_select_bed(move |id| {
            if let Some(window) = weak.upgrade() {
                window.set_placement_selected_bed_id(id);
            }
        });
    }

    // Place the selected succession onto the selected bed.
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_placement_place(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            place(&window, &mut state.borrow_mut());
        });
    }

    // Undo the last placement of this session.
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_placement_unplace_last(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            unplace(&window, &mut state.borrow_mut());
        });
    }

    // Explain the peak: list its composing series.
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_placement_peak_clicked(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            show_peak_composition(&window, &mut state.borrow_mut());
        });
    }
}

/// Reload the unplaced list, the bed tree, the strata options and the curve.
pub(crate) fn refresh_placement(window: &MainWindow, state: &mut UiState) {
    let season = season_year();
    let area_unit = state.app.area_unit();
    let loaded: Result<_, AppError> = state.runtime.block_on(async {
        let unplaced = unplaced_list(state.app.repo()).await?;
        let beds = list_locations_tree(state.app.repo(), area_unit).await?;
        let strata = list_strata_options(state.app.repo()).await?;
        let curve = occupancy_curve(state.app.repo(), season).await?;
        Ok((unplaced, beds, strata, curve))
    });
    let (unplaced, beds, strata, curve) = match loaded {
        Ok(v) => v,
        Err(e) => {
            tracing::error!(error = %e, "failed to load placement data");
            return;
        }
    };

    // Unplaced list.
    let rows: Vec<SlintUnplaced> = unplaced
        .into_iter()
        .map(|u| SlintUnplaced {
            id: SharedString::from(u.id),
            variety: SharedString::from(u.variety),
            date: SharedString::from(u.planned_on),
            bed_meters: SharedString::from(u.bed_meters),
        })
        .collect();
    window.set_placement_unplaced_rows(ModelRc::new(VecModel::from(rows)));

    // Bed tree.
    let bed_rows: Vec<SlintBed> = beds
        .into_iter()
        .map(|b| SlintBed {
            id: SharedString::from(b.id),
            name: SharedString::from(b.name),
            depth: i32::try_from(b.depth).unwrap_or(0),
            dimensions: SharedString::from(b.dimensions_label),
        })
        .collect();
    window.set_placement_bed_rows(ModelRc::new(VecModel::from(bed_rows)));

    // Strata options (labels for the dropdown; ids parallel in UiState).
    state.strata_ids = strata.iter().map(|s| s.id.clone()).collect();
    let strata_labels: Vec<SharedString> = strata
        .into_iter()
        .map(|s| SharedString::from(s.label))
        .collect();
    window.set_placement_strata_options(ModelRc::new(VecModel::from(strata_labels)));

    apply_curve(window, state, &curve);

    // Composition resets until the grower clicks a peak again.
    window.set_placement_composition_rows(ModelRc::new(VecModel::from(Vec::<SlintComp>::new())));
}

/// Push a computed curve into the window: the two series paths, presence flags,
/// overflow flag and the localized peak line.
fn apply_curve(window: &MainWindow, state: &UiState, curve: &OccupancyCurve) {
    window.set_placement_open_path(SharedString::from(series_path(
        curve,
        |p| p.open,
        curve.open_capacity,
    )));
    window.set_placement_covered_path(SharedString::from(series_path(
        curve,
        |p| p.covered,
        curve.covered_capacity,
    )));
    window.set_placement_has_open(curve.has_open);
    window.set_placement_has_covered(curve.has_covered);
    window.set_placement_over_capacity(curve.over_capacity);
    window.set_placement_peak_text(SharedString::from(peak_line(state, curve)));
}

/// Build an SVG polyline over the plot box from a per-week value normalized to
/// its group capacity (0..=100%). Clamps so an overflow still draws inside the box.
fn series_path(
    curve: &OccupancyCurve,
    value: impl Fn(&pomone_app::OccupancyPoint) -> f32,
    cap: f32,
) -> String {
    use std::fmt::Write as _;
    let n = curve.points.len().max(2);
    let mut cmds = String::new();
    for (i, p) in curve.points.iter().enumerate() {
        #[allow(clippy::cast_precision_loss)]
        let x = (i as f32) / ((n - 1) as f32) * PLOT_W;
        let frac = if cap > f32::EPSILON {
            (value(p) / cap).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let y = (1.0 - frac) * PLOT_H;
        let cmd = if i == 0 { 'M' } else { 'L' };
        let _ = write!(cmds, " {cmd} {x:.1} {y:.1}");
    }
    cmds
}

/// Which cover group is the more constrained (higher peak/capacity ratio) —
/// the one the amber flag and the peak line are about. `true` = sheltered.
/// The peak text and the peak composition MUST agree on this, or clicking the
/// amber peak would explain a different group/week (FR11).
fn binding_group(curve: &OccupancyCurve) -> bool {
    let ratio = |peak: f32, cap: f32| if cap > f32::EPSILON { peak / cap } else { 0.0 };
    curve.has_covered
        && ratio(curve.peak_covered, curve.covered_capacity)
            >= ratio(curve.peak_open, curve.open_capacity)
}

/// The localized peak line, for the binding cover group (see [`binding_group`]).
fn peak_line(state: &UiState, curve: &OccupancyCurve) -> String {
    let covered_binds = binding_group(curve);
    let (peak, cap, key) = if covered_binds {
        (
            curve.peak_covered,
            curve.covered_capacity,
            "placement-peak-covered",
        )
    } else {
        (curve.peak_open, curve.open_capacity, "placement-peak-open")
    };
    let mut args = FluentArgs::new();
    args.set("peak", format!("{peak:.1}"));
    args.set("cap", format!("{cap:.1}"));
    state.app.i18n().t_args(key, &args)
}

/// Place the selected succession onto the selected bed.
fn place(window: &MainWindow, state: &mut UiState) {
    let unplaced_id = window.get_placement_selected_unplaced_id().to_string();
    let bed_id = window.get_placement_selected_bed_id().to_string();
    if unplaced_id.is_empty() || bed_id.is_empty() {
        return;
    }
    let plants_text = window.get_placement_plants_count_text().to_string();
    let Ok(plants_count) = plants_text.trim().parse::<u32>() else {
        set_status(window, state, "placement-error-plants", true);
        return;
    };
    if plants_count == 0 {
        set_status(window, state, "placement-error-plants", true);
        return;
    }
    let strata_index = usize::try_from(window.get_placement_strata_index()).unwrap_or(0);
    let Some(strata_id_str) = state.strata_ids.get(strata_index).cloned() else {
        set_status(window, state, "placement-error-strata", true);
        return;
    };

    let (Ok(pp_id), Ok(loc_id), Ok(strata_id)) = (
        parse_id(&unplaced_id),
        parse_id(&bed_id),
        parse_id(&strata_id_str),
    ) else {
        set_status(window, state, "placement-error-generic", true);
        return;
    };

    let result = state.runtime.block_on(async {
        place_planned_planting(
            state.app.repo(),
            PlacementRequest::new(pp_id, loc_id, strata_id, plants_count),
            Local::now().date_naive(),
        )
        .await
    });
    match result {
        Ok(_) => {
            state.placement_last_placed = unplaced_id;
            window.set_placement_selected_unplaced_id(SharedString::from(""));
            refresh_placement(window, state);
            set_status(window, state, "placement-placed", false);
        }
        Err(e) => {
            let msg = localize_app_error(state.app.i18n(), &e);
            window.set_placement_status_text(SharedString::from(msg));
            window.set_placement_status_is_error(true);
        }
    }
}

/// Undo the last placement of this session.
fn unplace(window: &MainWindow, state: &mut UiState) {
    if state.placement_last_placed.is_empty() {
        set_status(window, state, "placement-nothing-to-undo", true);
        return;
    }
    let Ok(pp_id) = parse_id(&state.placement_last_placed) else {
        set_status(window, state, "placement-error-generic", true);
        return;
    };
    let result = state
        .runtime
        .block_on(async { unplace_planned_planting(state.app.repo(), pp_id).await });
    match result {
        Ok(()) => {
            state.placement_last_placed.clear();
            refresh_placement(window, state);
            set_status(window, state, "placement-unplaced", false);
        }
        Err(e) => {
            let msg = localize_app_error(state.app.i18n(), &e);
            window.set_placement_status_text(SharedString::from(msg));
            window.set_placement_status_is_error(true);
        }
    }
}

/// Compute the composition of the binding peak and list it.
fn show_peak_composition(window: &MainWindow, state: &mut UiState) {
    let season = season_year();
    let loaded: Result<_, AppError> = state.runtime.block_on(async {
        let curve = occupancy_curve(state.app.repo(), season).await?;
        // Same binding group the amber peak names, so the panel explains THAT
        // peak — its week and its cover side (FR11).
        let covered_binds = binding_group(&curve);
        let peak_week = curve
            .points
            .iter()
            .max_by(|a, b| {
                let va = if covered_binds { a.covered } else { a.open };
                let vb = if covered_binds { b.covered } else { b.open };
                va.partial_cmp(&vb).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map_or(1, |p| p.week);
        let comp =
            peak_composition(state.app.repo(), season, peak_week, Some(covered_binds)).await?;
        Ok(comp)
    });
    let comp = match loaded {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(error = %e, "failed to compute peak composition");
            return;
        }
    };
    let rows: Vec<SlintComp> = comp
        .into_iter()
        .map(|c| SlintComp {
            variety: SharedString::from(c.variety),
            date: SharedString::from(c.date),
            bed_meters: SharedString::from(c.bed_meters),
        })
        .collect();
    window.set_placement_composition_rows(ModelRc::new(VecModel::from(rows)));
}

/// Set the localized status line.
fn set_status(window: &MainWindow, state: &UiState, key: &str, is_error: bool) {
    window.set_placement_status_text(SharedString::from(state.app.i18n().t(key)));
    window.set_placement_status_is_error(is_error);
}
