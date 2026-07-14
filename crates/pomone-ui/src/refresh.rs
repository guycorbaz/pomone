//! Shared per-screen refresh helpers, snapshots and Slint row converters.
//! Extracted from `main.rs` (story 0.4); re-exported from the crate root so
//! `crate::…` paths keep working everywhere.

use crate::{i32_to_usize, parse_hex_color, usize_to_i32, UiState};
use anyhow::{Context, Result};
use chrono::{Datelike, Days, Local, NaiveDate, Weekday};
use pomone_app::{
    bed_usage_series, get_planting_detail, list_agenda, list_calendar_entries, list_crops,
    list_families_admin, list_family_options, list_location_kind_options, list_location_options,
    list_locations_tree, list_parent_options, list_planting_tasks, list_plantings,
    list_strata_options, list_strata_rows, list_task_category_options,
    list_treatments_for_planting, list_varieties_for_crop, list_variety_options,
    list_yearly_harvests_for_planting, planting_status_key, AgendaRow as AppAgendaRow, App,
    AppError, BedUsagePoint as AppBedUsagePoint, CalendarEntry as AppCalendarEntry,
    CalendarEntryKind, CalendarEventKind, CropRow as AppCropRow, CycleDates, FamilyAdminRow,
    FamilyOption, LocationKindOption, LocationListItem, LocationOption, ParentLocationOption,
    PlantingDetail as AppPlantingDetail, PlantingRow as AppPlantingRow,
    PlantingTaskRow as AppPlantingTaskRow, StrataOption, StrataRow as AppStrataRow,
    TreatmentRow as AppTreatmentRow, VarietyOption, VarietyRow as AppVarietyRow,
    YearlyHarvestRow as AppYearlyHarvestRow,
};
use pomone_domain::{holidays_in_year, HolidayRegion, PlantingStatus, DEFAULT_FAMILY_COLOR};
use slint::{ModelRc, SharedString, VecModel};

use crate::generated::{
    AgendaRow as SlintAgendaRow, CropRow as SlintCropRow, DetailLine as SlintDetailLine,
    FamilyAdminItem as SlintFamilyAdminItem, GanttBar as SlintGanttBar,
    LocationItem as SlintLocationItem, MainWindow, PaletteColor as SlintPaletteColor,
    PlantingRow as SlintPlantingRow, PlantingTaskRow as SlintPlantingTaskRow,
    StrataItem as SlintStrataItem, TaskCalendarDay as SlintTaskCalendarDay,
    TaskCategoryChip as SlintTaskCategoryChip, TaskRow as SlintTaskRow,
    TreatmentRow as SlintTreatmentRow, VarietyRow as SlintVarietyRow,
    YearlyHarvestRow as SlintYearlyHarvestRow,
};

/// Rebuild the home page's bed-usage curve: a 12-month series turned into two
/// SVG polyline strings (open-field + sheltered) in a 12 × 100 viewbox, with
/// y flipped so 100% sits at the top. `has-data` drives the empty state;
/// `has-sheltered` hides the sheltered curve on farms without any.
pub(crate) fn refresh_bed_usage(window: &MainWindow, app: &App, runtime: &tokio::runtime::Runtime) {
    // Same season the home Gantt shows: the current calendar year.
    let season_year = Local::now().date_naive().year();
    let usage = match runtime.block_on(async { bed_usage_series(app.repo(), season_year).await }) {
        Ok(u) => u,
        Err(e) => {
            tracing::error!(error = %e, "failed to compute bed usage");
            return;
        }
    };

    window.set_bed_usage_open_path(SharedString::from(polyline_path(&usage.points, |p| {
        p.open_pct
    })));
    window.set_bed_usage_sheltered_path(SharedString::from(polyline_path(&usage.points, |p| {
        p.sheltered_pct
    })));
    window.set_bed_usage_has_data(usage.has_open_beds || usage.has_sheltered_beds);
    window.set_bed_usage_has_open(usage.has_open_beds);
    window.set_bed_usage_has_sheltered(usage.has_sheltered_beds);
}

/// Build an SVG polyline ("M x y L x y …") for one weekly series, in the plot's
/// pixel coordinate system. Each week is placed at its mid-point day-of-year
/// (so it lines up with the Gantt's day-of-year bars), and y is flipped so
/// 100% draws at the top.
pub(crate) fn polyline_path(
    series: &[AppBedUsagePoint],
    value: impl Fn(&AppBedUsagePoint) -> f64,
) -> String {
    use std::fmt::Write as _;
    let total_width = 12.0 * PLOT_MONTH_WIDTH_PX;
    let mut cmds = String::new();
    for (i, point) in series.iter().enumerate() {
        let midpoint_doy = (f64::from(point.week) - 0.5) * 7.0;
        let x = (midpoint_doy.min(PLOT_TOTAL_DAYS) / PLOT_TOTAL_DAYS) * total_width;
        let y = (1.0 - value(point).clamp(0.0, 100.0) / 100.0) * PLOT_HEIGHT_PX;
        let cmd = if i == 0 { 'M' } else { 'L' };
        let _ = write!(cmds, " {cmd} {x:.1} {y:.1}");
    }
    cmds
}

/// Snapshot of everything the Plantings screen needs on every refresh.
pub(crate) struct PlantingsSnapshot {
    pub(crate) varieties: Vec<VarietyOption>,
    pub(crate) locations: Vec<LocationOption>,
    pub(crate) strata: Vec<StrataOption>,
    pub(crate) plantings: Vec<AppPlantingRow>,
}

/// Reload the plantings list AND the dropdown options for the form. Stores
/// the option IDs in `state` so the create callback can look them up by index.
pub(crate) fn refresh_plantings(window: &MainWindow, state: &mut UiState) -> Result<()> {
    let snapshot: Result<PlantingsSnapshot, AppError> = state.runtime.block_on(async {
        let varieties = list_variety_options(state.app.repo()).await?;
        let locations = list_location_options(state.app.repo()).await?;
        let strata = list_strata_options(state.app.repo()).await?;
        let plantings = list_plantings(state.app.repo(), state.app.area_unit()).await?;
        Ok(PlantingsSnapshot {
            varieties,
            locations,
            strata,
            plantings,
        })
    });
    let snapshot = snapshot.context("failed to load plantings data")?;

    state.variety_ids = snapshot.varieties.iter().map(|v| v.id.clone()).collect();
    state.variety_is_annuals_plantings = snapshot.varieties.iter().map(|v| v.is_annual).collect();
    state.location_ids = snapshot.locations.iter().map(|l| l.id.clone()).collect();
    state.strata_ids = snapshot.strata.iter().map(|s| s.id.clone()).collect();

    let variety_labels: Vec<SharedString> = snapshot
        .varieties
        .into_iter()
        .map(|v| SharedString::from(v.label))
        .collect();
    window.set_variety_labels(ModelRc::new(VecModel::from(variety_labels)));
    let variety_is_annuals: Vec<bool> = state.variety_is_annuals_plantings.clone();
    window.set_variety_is_annuals(ModelRc::new(VecModel::from(variety_is_annuals)));

    let location_labels: Vec<SharedString> = snapshot
        .locations
        .into_iter()
        .map(|l| SharedString::from(l.label))
        .collect();
    window.set_location_labels(ModelRc::new(VecModel::from(location_labels)));

    let strata_labels: Vec<SharedString> = snapshot
        .strata
        .into_iter()
        .map(|s| SharedString::from(s.label))
        .collect();
    window.set_strata_labels(ModelRc::new(VecModel::from(strata_labels)));

    // Build the Gantt model alongside the list. Only annuals (Cycle schedule)
    // and only those whose first-harvest year matches today's year — winter-sow
    // plantings from another season are intentionally hidden so the today-line
    // stays meaningful and the axis doesn't need to span multiple years.
    let today_year = Local::now().date_naive().year();
    let gantt_bars: Vec<SlintGanttBar> = snapshot
        .plantings
        .iter()
        .filter_map(|row| to_gantt_bar(row, today_year))
        .collect();
    window.set_gantt_bars(ModelRc::new(VecModel::from(gantt_bars)));

    // Apply the active table sort before converting to Slint rows.
    let mut app_rows = snapshot.plantings;
    sort_planting_rows(
        &mut app_rows,
        &state.plantings_sort_column,
        state.plantings_sort_asc,
    );
    window.set_plantings_sort_column(SharedString::from(state.plantings_sort_column.clone()));
    window.set_plantings_sort_asc(state.plantings_sort_asc);

    let i18n = state.app.i18n();
    let rows: Vec<SlintPlantingRow> = app_rows
        .into_iter()
        .map(|r| to_slint_row(r, i18n))
        .collect();
    window.set_plantings(ModelRc::new(VecModel::from(rows)));

    // Clamp selected indices to stay valid after a refresh.
    let varieties_len = state.variety_ids.len();
    let locations_len = state.location_ids.len();
    if i32_to_usize(window.get_variety_index()) >= varieties_len {
        window.set_variety_index(0);
    }
    if i32_to_usize(window.get_location_index()) >= locations_len {
        window.set_location_index(0);
    }
    if i32_to_usize(window.get_strata_index()) >= state.strata_ids.len() {
        window.set_strata_index(0);
    }

    Ok(())
}

/// Sort the plantings table in place by the active column + direction. Unknown
/// columns fall back to the variety label so the list is always deterministic.
pub(crate) fn sort_planting_rows(rows: &mut [AppPlantingRow], column: &str, ascending: bool) {
    match column {
        "location" => rows.sort_by(|a, b| {
            a.location_label
                .to_lowercase()
                .cmp(&b.location_label.to_lowercase())
        }),
        "area" => rows.sort_by_key(|r| r.area_m2),
        "plants" => rows.sort_by_key(|r| r.plants_count),
        "status" => {
            rows.sort_by(|a, b| planting_status_key(a.status).cmp(planting_status_key(b.status)));
        }
        _ => rows.sort_by(|a, b| {
            a.variety_label
                .to_lowercase()
                .cmp(&b.variety_label.to_lowercase())
        }),
    }
    if !ascending {
        rows.reverse();
    }
}

pub(crate) fn to_slint_row(row: AppPlantingRow, i18n: &pomone_app::I18n) -> SlintPlantingRow {
    SlintPlantingRow {
        id: SharedString::from(row.id),
        variety_label: SharedString::from(row.variety_label),
        crop_initials: SharedString::from(row.crop_initials),
        family_color: parse_hex_color(&row.family_color),
        location_label: SharedString::from(row.location_label),
        strata_label: SharedString::from(row.strata_label),
        schedule_summary: SharedString::from(row.schedule_summary),
        area_label: SharedString::from(row.area_label),
        plants_count: usize_to_i32(row.plants_count as usize),
        status_label: SharedString::from(i18n.t(planting_status_key(row.status))),
        status_active: row.status == PlantingStatus::Active,
    }
}

/// Convert one planting row into a Gantt bar.
///
/// Returns `None` if the row is perennial (no cycle dates) or if the
/// planting's first-harvest year doesn't match `today_year` — keeping the
/// timeline single-year keeps the today-line meaningful and avoids the
/// multi-axis complexity that comes with cross-year cycles.
///
/// Dates that fall in a different year than the harvest year (e.g. a
/// winter sow that crosses Jan 1) are clamped to day 1 so the greenhouse
/// segment still appears at the very start of the axis rather than
/// disappearing.
pub(crate) fn to_gantt_bar(row: &AppPlantingRow, today_year: i32) -> Option<SlintGanttBar> {
    let CycleDates {
        sown_on,
        transplanted_on,
        first_harvest_on,
        last_harvest_on,
    } = row.cycle_dates?;
    if first_harvest_on.year() != today_year {
        return None;
    }
    let doy_in_year = |d: chrono::NaiveDate| -> i32 {
        // Harvest-end can technically spill into next year (winter crops);
        // clamp to 365 so the bar reaches the right edge of the axis.
        // Winter-sow plantings start in the previous year — clamp to day 1
        // so the greenhouse segment is visible at the axis start.
        use std::cmp::Ordering;
        match d.year().cmp(&today_year) {
            Ordering::Less => 1,
            Ordering::Greater => AXIS_DAYS,
            // Leap years run to ordinal 366; the axis is 365 wide, so fold the
            // extra day onto the last column for a consistent mapping (#67).
            Ordering::Equal => usize_to_i32(d.ordinal() as usize).min(AXIS_DAYS),
        }
    };
    Some(SlintGanttBar {
        id: SharedString::from(row.id.clone()),
        name: SharedString::from(row.variety_label.clone()),
        sow_day: sown_on.map_or(0, doy_in_year),
        transplant_day: transplanted_on.map_or(0, doy_in_year),
        harvest_start_day: doy_in_year(first_harvest_on),
        harvest_end_day: doy_in_year(last_harvest_on),
    })
}

/// Snapshot of everything the Cultures screen needs on every refresh.
pub(crate) struct CulturesSnapshot {
    pub(crate) crops: Vec<AppCropRow>,
    pub(crate) families: Vec<FamilyOption>,
}

/// Reload crops + dropdown options. Also refreshes the right-side varieties
/// for whichever crop is currently selected (or empties them if none).
pub(crate) fn refresh_cultures(window: &MainWindow, state: &mut UiState) -> Result<()> {
    let snapshot: Result<CulturesSnapshot, AppError> = state.runtime.block_on(async {
        let crops = list_crops(state.app.repo()).await?;
        let families = list_family_options(state.app.repo()).await?;
        Ok(CulturesSnapshot { crops, families })
    });
    let snapshot = snapshot.context("failed to load cultures data")?;

    state.crop_ids = snapshot.crops.iter().map(|c| c.id.clone()).collect();
    state.crop_is_annuals = snapshot.crops.iter().map(|c| c.is_annual).collect();
    state.family_ids = snapshot.families.iter().map(|f| f.id.clone()).collect();

    let crop_rows: Vec<SlintCropRow> = snapshot.crops.into_iter().map(crop_to_slint).collect();
    window.set_crops(ModelRc::new(VecModel::from(crop_rows)));

    let family_labels: Vec<SharedString> = snapshot
        .families
        .into_iter()
        .map(|f| SharedString::from(f.label))
        .collect();
    window.set_family_labels(ModelRc::new(VecModel::from(family_labels)));

    // Clamp form dropdowns; keep selected-crop-index if still valid.
    if i32_to_usize(window.get_family_index()) >= state.family_ids.len() {
        window.set_family_index(0);
    }
    let selected_idx = window.get_selected_crop_index();
    if selected_idx < 0 || i32_to_usize(selected_idx) >= state.crop_ids.len() {
        window.set_selected_crop_index(-1);
    }
    refresh_varieties_of_selected_crop(window, state)
}

/// Re-read the variety list for the currently selected crop. If no crop is
/// selected (`selected-crop-index < 0`), the list is cleared.
pub(crate) fn refresh_varieties_of_selected_crop(
    window: &MainWindow,
    state: &mut UiState,
) -> Result<()> {
    let idx = window.get_selected_crop_index();
    if idx < 0 {
        window.set_varieties(ModelRc::new(VecModel::from(Vec::<SlintVarietyRow>::new())));
        return Ok(());
    }
    let Some(crop_id_str) = state.crop_ids.get(i32_to_usize(idx)).cloned() else {
        window.set_varieties(ModelRc::new(VecModel::from(Vec::<SlintVarietyRow>::new())));
        return Ok(());
    };
    let varieties: Result<Vec<AppVarietyRow>, AppError> = state
        .runtime
        .block_on(async { list_varieties_for_crop(state.app.repo(), &crop_id_str).await });
    let varieties = varieties.context("failed to load varieties")?;
    let rows: Vec<SlintVarietyRow> = varieties.into_iter().map(variety_to_slint).collect();
    window.set_varieties(ModelRc::new(VecModel::from(rows)));
    Ok(())
}

pub(crate) fn crop_to_slint(row: AppCropRow) -> SlintCropRow {
    SlintCropRow {
        id: SharedString::from(row.id),
        name: SharedString::from(row.name),
        family_label: SharedString::from(row.family_label),
        lifespan_label: SharedString::from(row.lifespan_label),
        pruning_label: SharedString::from(row.pruning_label),
        variety_count: usize_to_i32(row.variety_count as usize),
        is_annual: row.is_annual,
        in_use: row.in_use,
    }
}

pub(crate) fn variety_to_slint(row: AppVarietyRow) -> SlintVarietyRow {
    SlintVarietyRow {
        id: SharedString::from(row.id),
        name: SharedString::from(row.name),
        description: SharedString::from(row.description),
        profile_label: SharedString::from(row.profile_label),
        in_use: row.in_use,
    }
}

/// Clear the crop form and drop back to create mode.
pub(crate) fn reset_crop_form_to_create(window: &MainWindow, state: &mut UiState) {
    state.editing_crop_id.clear();
    window.set_crop_is_edit_mode(false);
    window.set_new_crop_name(SharedString::from(""));
    window.set_new_crop_latin(SharedString::from(""));
    window.set_new_crop_lifespan_index(0);
    window.set_new_crop_pruning_index(0);
    window.set_new_crop_lifespan_years(SharedString::from("30"));
    window.set_new_crop_years_to_first_yield(SharedString::from("3"));
}

/// Clear the variety form and drop back to create mode. Numeric fields go back
/// to the same defaults the Slint form ships with.
pub(crate) fn reset_variety_form_to_create(window: &MainWindow, state: &mut UiState) {
    state.editing_variety_id.clear();
    window.set_variety_is_edit_mode(false);
    window.set_new_variety_name(SharedString::from(""));
    window.set_new_variety_description(SharedString::from(""));
    window.set_new_variety_dtt(SharedString::from("35"));
    window.set_new_variety_dtm(SharedString::from("70"));
    window.set_new_variety_window(SharedString::from("60"));
    window.set_new_variety_bud_break_doy(SharedString::from(""));
    window.set_new_variety_flowering_doy(SharedString::from(""));
    window.set_new_variety_harvest_start_doy(SharedString::from("220"));
    window.set_new_variety_harvest_end_doy(SharedString::from("280"));
    window.set_new_variety_yield_kg(SharedString::from(""));
}

/// Snapshot of everything the Locations screen needs on every refresh.
pub(crate) struct LocationsSnapshot {
    pub(crate) items: Vec<LocationListItem>,
    pub(crate) kinds: Vec<LocationKindOption>,
    pub(crate) parents: Vec<ParentLocationOption>,
}

/// Reload the location tree + dropdown options (kinds, parents). Indices are
/// clamped to stay valid after a refresh.
pub(crate) fn refresh_locations(window: &MainWindow, state: &mut UiState) -> Result<()> {
    // "(aucun) / (none)" label for the synthetic root-parent option.
    let none_label = state.app.i18n().t("parent-none");
    let snapshot: Result<LocationsSnapshot, AppError> = state.runtime.block_on(async {
        let items = list_locations_tree(state.app.repo(), state.app.area_unit()).await?;
        let kinds = list_location_kind_options(state.app.repo()).await?;
        let parents = list_parent_options(state.app.repo(), &none_label).await?;
        Ok(LocationsSnapshot {
            items,
            kinds,
            parents,
        })
    });
    let snapshot = snapshot.context("failed to load locations data")?;

    state.location_kind_ids = snapshot.kinds.iter().map(|k| k.id.clone()).collect();
    state.parent_location_ids = snapshot.parents.iter().map(|p| p.id.clone()).collect();

    let items: Vec<SlintLocationItem> = snapshot.items.into_iter().map(location_to_slint).collect();
    window.set_locations(ModelRc::new(VecModel::from(items)));

    let kind_labels: Vec<SharedString> = snapshot
        .kinds
        .into_iter()
        .map(|k| SharedString::from(k.label))
        .collect();
    window.set_loc_kind_labels(ModelRc::new(VecModel::from(kind_labels)));

    let parent_labels: Vec<SharedString> = snapshot
        .parents
        .into_iter()
        .map(|p| SharedString::from(p.label))
        .collect();
    window.set_loc_parent_labels(ModelRc::new(VecModel::from(parent_labels)));

    if i32_to_usize(window.get_loc_kind_index()) >= state.location_kind_ids.len() {
        window.set_loc_kind_index(0);
    }
    if i32_to_usize(window.get_loc_parent_index()) >= state.parent_location_ids.len() {
        window.set_loc_parent_index(0);
    }
    Ok(())
}

pub(crate) fn location_to_slint(item: LocationListItem) -> SlintLocationItem {
    SlintLocationItem {
        id: SharedString::from(item.id),
        name: SharedString::from(item.name),
        kind_label: SharedString::from(item.kind_label),
        area_label: SharedString::from(item.area_label),
        dimensions_label: SharedString::from(item.dimensions_label),
        parent_label: SharedString::from(item.parent_label),
        full_path: SharedString::from(item.full_path),
        depth: usize_to_i32(item.depth as usize),
        in_use: item.in_use,
    }
}

pub(crate) fn refresh_strata(window: &MainWindow, state: &mut UiState) -> Result<()> {
    let rows: Result<Vec<AppStrataRow>, AppError> = state
        .runtime
        .block_on(async { list_strata_rows(state.app.repo()).await });
    let rows = rows.context("failed to load strata")?;
    let items: Vec<SlintStrataItem> = rows
        .into_iter()
        .map(|r| SlintStrataItem {
            id: SharedString::from(r.id),
            name: SharedString::from(r.name),
            description: SharedString::from(r.description),
            height_label: SharedString::from(r.height_label),
            sort_order: r.sort_order,
            in_use: r.in_use,
        })
        .collect();
    window.set_strata_items(ModelRc::new(VecModel::from(items)));
    Ok(())
}

/// Reload the Families admin list into the Slint model (colour parsed for the
/// swatch, raw hex kept for form pre-fill).
pub(crate) fn refresh_families(window: &MainWindow, state: &mut UiState) -> Result<()> {
    let rows: Vec<FamilyAdminRow> = state
        .runtime
        .block_on(async { list_families_admin(state.app.repo()).await })
        .context("failed to load families")?;
    let items: Vec<SlintFamilyAdminItem> = rows
        .into_iter()
        .map(|r| SlintFamilyAdminItem {
            id: SharedString::from(r.id),
            name: SharedString::from(r.name),
            latin_name: SharedString::from(r.latin_name),
            color: parse_hex_color(&r.color),
            color_hex: SharedString::from(r.color),
            in_use: r.in_use,
        })
        .collect();
    window.set_families_items(ModelRc::new(VecModel::from(items)));
    Ok(())
}

/// Reset the Families form to a blank "create" state.
pub(crate) fn reset_families_form_to_create(window: &MainWindow, state: &mut UiState) {
    state.editing_family_id.clear();
    window.set_families_is_edit_mode(false);
    window.set_families_form_name(SharedString::from(""));
    window.set_families_form_latin(SharedString::from(""));
    window.set_families_form_description(SharedString::from(""));
    window.set_families_form_color(SharedString::from(DEFAULT_FAMILY_COLOR));
    window.set_families_form_color_preview(parse_hex_color(DEFAULT_FAMILY_COLOR));
    window.set_families_status_text(SharedString::from(""));
    window.set_families_status_is_error(false);
}

/// Curated palette offered by the popup colour chooser (shared by the Families
/// and Task Types forms). A spread of hues that read on both the light and dark
/// surfaces; users can still type any custom hex. Kept at a multiple of the
/// picker's column count for a tidy grid.
pub(crate) fn color_chooser_palette() -> Vec<SlintPaletteColor> {
    const HEXES: &[&str] = &[
        "#B85C38", "#A64238", "#C4622F", "#B07C25", "#C89A3A", "#8A9A2E", "#6FAF7A", "#4E8C5A",
        "#3C6E47", "#244529", "#5F9F8B", "#4F7F8F", "#3E6B87", "#6B4E8C", "#9A6E5C", "#7A6A5C",
        "#6B5D4D", "#A0518A",
    ];
    HEXES
        .iter()
        .map(|hex| SlintPaletteColor {
            hex: SharedString::from(*hex),
            color: parse_hex_color(hex),
        })
        .collect()
}

/// Inverse of `status_from_index` (in `wiring::planting_detail`).
pub(crate) fn status_to_index(status: PlantingStatus) -> i32 {
    match status {
        PlantingStatus::Active => 0,
        PlantingStatus::Completed => 1,
        PlantingStatus::Failed => 2,
        PlantingStatus::Abandoned => 3,
    }
}

/// Load one planting's detail, push it to the UI and switch to the detail
/// page. `previous_page` is stored on the state so the Back button knows
/// where to return.
pub(crate) fn open_planting_detail(
    window: &MainWindow,
    state: &mut UiState,
    planting_id: &str,
    previous_page: &str,
) {
    previous_page.clone_into(&mut state.detail_previous_page);
    match refresh_planting_detail(window, state, planting_id) {
        Ok(()) => {
            window.set_current_page(SharedString::from("planting-detail"));
            window.set_status_text(SharedString::from(""));
            window.set_status_is_error(false);
        }
        Err(e) => {
            tracing::error!(error = %e, planting_id, "failed to load planting detail");
            // Push the empty-state shape so the page renders something
            // useful instead of stale data from a previous open.
            window.set_detail_has_detail(false);
            window.set_current_page(SharedString::from("planting-detail"));
        }
    }
}

pub(crate) fn refresh_planting_detail(
    window: &MainWindow,
    state: &mut UiState,
    planting_id: &str,
) -> Result<()> {
    type DetailSnapshot = (
        AppPlantingDetail,
        Vec<AppYearlyHarvestRow>,
        Vec<AppPlantingTaskRow>,
        Vec<AppTreatmentRow>,
    );
    let today = Local::now().date_naive();
    let snapshot: Result<DetailSnapshot, AppError> = state.runtime.block_on(async {
        let detail =
            get_planting_detail(state.app.repo(), planting_id, state.app.area_unit()).await?;
        // The yearly-harvest table is empty for annuals; querying it
        // anyway keeps the code path uniform and the SQL is a no-op.
        let harvests =
            list_yearly_harvests_for_planting(state.app.repo(), planting_id, state.app.mass_unit())
                .await?;
        let tasks =
            list_planting_tasks(state.app.repo(), planting_id, today, state.app.i18n()).await?;
        let treatments = list_treatments_for_planting(state.app.repo(), planting_id).await?;
        Ok((detail, harvests, tasks, treatments))
    });
    let (detail, harvests, tasks, treatments) =
        snapshot.context("failed to load planting detail")?;

    planting_id.clone_into(&mut state.detail_planting_id);

    let i18n = state.app.i18n();
    let lines: Vec<SlintDetailLine> = detail
        .schedule_lines
        .into_iter()
        .map(|l| SlintDetailLine {
            label: SharedString::from(i18n.t(l.label_key)),
            value: SharedString::from(l.value),
        })
        .collect();
    let harvest_rows: Vec<SlintYearlyHarvestRow> = harvests
        .into_iter()
        .map(|h| SlintYearlyHarvestRow {
            year: h.year,
            expected_label: SharedString::from(h.expected_label),
            actual_label: SharedString::from(h.actual_label),
            variance_label: SharedString::from(h.variance_label),
            notes: SharedString::from(h.notes),
        })
        .collect();

    window.set_detail_variety_label(SharedString::from(detail.variety_label));
    window.set_detail_location_label(SharedString::from(detail.location_label));
    window.set_detail_strata_label(SharedString::from(detail.strata_label));
    window.set_detail_area_label(SharedString::from(detail.area_label));
    window.set_detail_plants_count(usize_to_i32(detail.plants_count as usize));
    window.set_detail_name_value(SharedString::from(detail.name.unwrap_or_default()));
    window.set_detail_notes_value(SharedString::from(detail.notes.unwrap_or_default()));
    // Life-cycle status (issue #63): the header badge + the selector's current
    // value. The action message is cleared on each (re)load.
    window.set_detail_status_label(SharedString::from(
        i18n.t(planting_status_key(detail.status)),
    ));
    window.set_detail_status_index(status_to_index(detail.status));
    window.set_detail_lifecycle_status_text(SharedString::from(""));
    window.set_detail_lifecycle_status_is_error(false);
    let task_rows: Vec<SlintPlantingTaskRow> = tasks
        .into_iter()
        .map(|t| SlintPlantingTaskRow {
            task_id: SharedString::from(t.task_id),
            planned_on: SharedString::from(t.planned_on),
            type_name: SharedString::from(t.type_name),
            color: parse_hex_color(&t.color),
            completed: t.completed,
            skipped: t.skipped,
            skip_reason: SharedString::from(t.skip_reason),
            overdue: t.overdue,
            notes: SharedString::from(t.notes),
        })
        .collect();

    let treatment_rows: Vec<SlintTreatmentRow> = treatments
        .into_iter()
        .map(|t| SlintTreatmentRow {
            treatment_id: SharedString::from(t.treatment_id),
            applied_on: SharedString::from(t.applied_on),
            active_substance: SharedString::from(t.active_substance),
            product_name: SharedString::from(t.product_name),
            dose_label: SharedString::from(t.dose_label),
            notes: SharedString::from(t.notes),
        })
        .collect();

    window.set_detail_schedule_lines(ModelRc::new(VecModel::from(lines)));
    window.set_detail_task_rows(ModelRc::new(VecModel::from(task_rows)));
    window.set_detail_has_detail(true);
    window.set_detail_is_perennial(detail.is_perennial);
    window.set_harvest_rows(ModelRc::new(VecModel::from(harvest_rows)));
    window.set_treatment_rows(ModelRc::new(VecModel::from(treatment_rows)));
    // Clear stale form/status from the previous detail open.
    window.set_harvest_status_text(SharedString::from(""));
    window.set_harvest_status_is_error(false);
    window.set_treatment_status_text(SharedString::from(""));
    window.set_treatment_status_is_error(false);
    Ok(())
}

/// Rebuild the unified calendar's 42-cell day grid for `state.task_calendar_*`.
/// Each cell groups [`AppCalendarEntry`]s by date: editable tasks (TaskType
/// color) alongside read-only crop-cycle milestones (outline, by kind).
#[allow(clippy::too_many_lines)]
pub(crate) fn refresh_task_calendar(window: &MainWindow, state: &mut UiState) -> Result<()> {
    let year = state.task_calendar_year;
    let month = state.task_calendar_month;

    let first = first_of_month(year, month);
    let lead = weekday_offset_mon(first);
    let grid_start = first
        .checked_sub_days(Days::new(u64::from(lead)))
        .context("task calendar grid underflow")?;
    let grid_end = grid_start
        .checked_add_days(Days::new(41))
        .context("task calendar grid overflow")?;

    // Build the typed filter set from `state.task_filter_categories` (UI
    // holds them as stable string keys to stay decoupled from the enum).
    // Empty state = "all on" by convention: pushing `None` to the view
    // helper means no filtering.
    let filter_set: std::collections::HashSet<pomone_domain::TaskCategory> = state
        .task_filter_categories
        .iter()
        .filter_map(|k| category_from_key(k))
        .collect();
    let filter_arg = if filter_set.len() == category_count_total() {
        None
    } else {
        Some(&filter_set)
    };

    // Unified entries = operational tasks + curated crop-cycle milestones,
    // de-duplicated at the source (#47). The category filter applies to tasks
    // only; the milestone family is governed by its own master toggle.
    let show_milestones = state.show_milestones;
    let entries: Vec<AppCalendarEntry> = state
        .runtime
        .block_on(async {
            list_calendar_entries(
                state.app.repo(),
                grid_start,
                grid_end,
                filter_arg,
                show_milestones,
                Local::now().date_naive(),
            )
            .await
        })
        .context("failed to load unified calendar entries")?;

    let i18n_glyphs = state.app.i18n();
    let mut by_date: std::collections::HashMap<NaiveDate, Vec<SlintTaskRow>> =
        std::collections::HashMap::new();
    for e in &entries {
        let row = match e.kind {
            CalendarEntryKind::Task => SlintTaskRow {
                task_id: SharedString::from(e.task_id.map(|id| id.to_string()).unwrap_or_default()),
                planting_id: SharedString::from(""),
                is_milestone: false,
                milestone_kind: 0,
                glyph: SharedString::from(""),
                label: SharedString::from(e.label.clone()),
                color: e
                    .task_color
                    .as_deref()
                    .map_or_else(|| parse_hex_color("#3C6E47"), parse_hex_color),
                completed: e.completed,
                skipped: e.skipped,
            },
            CalendarEntryKind::Milestone => {
                let kind = e.milestone_kind.unwrap_or(CalendarEventKind::Sowing);
                SlintTaskRow {
                    task_id: SharedString::from(""),
                    planting_id: SharedString::from(e.planting_id.clone().unwrap_or_default()),
                    is_milestone: true,
                    milestone_kind: kind_to_int(kind),
                    glyph: SharedString::from(i18n_glyphs.t(kind_glyph_key(kind))),
                    label: SharedString::from(e.label.clone()),
                    color: parse_hex_color("#3C6E47"), // unused for milestones
                    completed: false,
                    skipped: false,
                }
            }
        };
        by_date.entry(e.date).or_default().push(row);
    }

    let today = Local::now().date_naive();

    // Public holidays of the configured region (#35), for every year the
    // 42-cell grid touches (a January/December grid spans two years).
    let mut holiday_by_date: std::collections::HashMap<NaiveDate, pomone_domain::Holiday> =
        std::collections::HashMap::new();
    if let Some(region) = HolidayRegion::parse(&state.app.config().holiday_region) {
        let mut years = vec![grid_start.year()];
        if grid_end.year() != grid_start.year() {
            years.push(grid_end.year());
        }
        for y in years {
            holiday_by_date.extend(holidays_in_year(region, y));
        }
    }

    let mut days: Vec<SlintTaskCalendarDay> = Vec::with_capacity(42);
    for offset in 0..42 {
        let date = grid_start
            .checked_add_days(Days::new(offset))
            .context("task calendar cell overflow")?;
        let in_current_month = date.year() == year && date.month() == month;
        let day_number = if in_current_month {
            i32::try_from(date.day()).unwrap_or(0)
        } else {
            0
        };
        let holiday = holiday_by_date.get(&date);
        let holiday_name = holiday.map_or_else(String::new, |h| {
            i18n_glyphs.t(&format!("holiday-{}", h.key()))
        });
        let cell_tasks: Vec<SlintTaskRow> = by_date.remove(&date).unwrap_or_default();
        days.push(SlintTaskCalendarDay {
            day_number,
            in_current_month,
            is_today: date == today,
            is_holiday: holiday.is_some(),
            holiday_name: SharedString::from(holiday_name),
            tasks: ModelRc::new(VecModel::from(cell_tasks)),
        });
    }
    window.set_task_calendar_days(ModelRc::new(VecModel::from(days)));
    window.set_task_calendar_any_tasks(!entries.is_empty());
    window.set_task_calendar_show_milestones(state.show_milestones);

    // Counts for the month-bar summary ("N tâches · M jalons").
    let n_tasks = entries
        .iter()
        .filter(|e| e.kind == CalendarEntryKind::Task)
        .count();
    let n_milestones = entries.len() - n_tasks;

    let i18n = state.app.i18n();
    let month_key = format!("month-{month}");
    let month_name = i18n.t(&month_key);
    window.set_task_calendar_month_label(SharedString::from(format!("{month_name} {year}")));

    let mut summary_args = fluent::FluentArgs::new();
    summary_args.set("tasks", i64::try_from(n_tasks).unwrap_or(0));
    summary_args.set("milestones", i64::try_from(n_milestones).unwrap_or(0));
    window.set_task_calendar_summary_text(SharedString::from(
        i18n.t_args("task-calendar-summary", &summary_args),
    ));

    // Keep the chip row in sync (selected state mirrors `state.task_filter_categories`,
    // colors mirror whatever the user has set in the types catalog).
    refresh_task_filter_chips(window, state)?;

    Ok(())
}

/// Push the filter-chip row to Slint. For each canonical category, the
/// chip carries the first matching `TaskType`'s color (so the chip
/// matches the pills it filters); types whose category has no seed fall
/// back to a neutral grey.
pub(crate) fn refresh_task_filter_chips(window: &MainWindow, state: &mut UiState) -> Result<()> {
    let types = state
        .runtime
        .block_on(async { state.app.repo().task_type_list().await })
        .context("failed to load task types for filter chips")?;
    let mut color_by_cat: std::collections::HashMap<&'static str, String> =
        std::collections::HashMap::new();
    for t in types {
        color_by_cat
            .entry(category_str_for(t.category))
            .or_insert(t.color);
    }

    let i18n = state.app.i18n();
    let chips: Vec<SlintTaskCategoryChip> = list_task_category_options()
        .into_iter()
        .map(|opt| {
            let color_str = color_by_cat
                .get(opt.key.as_str())
                .cloned()
                .unwrap_or_else(|| "#808080".to_owned());
            SlintTaskCategoryChip {
                key: SharedString::from(opt.key.clone()),
                label: SharedString::from(i18n.t(&opt.label_key)),
                color: parse_hex_color(&color_str),
                selected: state.task_filter_categories.contains(&opt.key),
            }
        })
        .collect();
    window.set_task_calendar_filter_chips(ModelRc::new(VecModel::from(chips)));
    Ok(())
}

/// Load the flat task list (newest first) and push it to the window.
pub(crate) fn refresh_agenda(window: &MainWindow, state: &mut UiState) -> Result<()> {
    let today = Local::now().date_naive();
    let rows: Vec<AppAgendaRow> = state
        .runtime
        .block_on(async { list_agenda(state.app.repo(), state.app.i18n(), today).await })
        .context("failed to load tasks list")?;

    let mapped: Vec<SlintAgendaRow> = rows
        .into_iter()
        .map(|r| SlintAgendaRow {
            task_id: SharedString::from(r.task_id),
            planned_on: SharedString::from(r.planned_on),
            label: SharedString::from(r.label),
            color: parse_hex_color(&r.color),
            completed: r.completed,
            skipped: r.skipped,
            skip_reason: SharedString::from(r.skip_reason),
            overdue: r.overdue,
            today: r.today,
        })
        .collect();
    window.set_agenda_rows(ModelRc::new(VecModel::from(mapped)));
    Ok(())
}

/// After the task form routes back, refresh whichever page we returned to so
/// the change (create / edit / delete) shows up without a manual reload. The
/// form is reachable both from the Task Calendar and from a planting's task
/// list; `prev` says which one to repaint.
pub(crate) fn refresh_after_task_form(window: &MainWindow, state: &mut UiState, prev: &str) {
    let result = match prev {
        "planting-detail" => {
            let pid = state.detail_planting_id.clone();
            refresh_planting_detail(window, state, &pid)
        }
        "agenda" => refresh_agenda(window, state),
        _ => refresh_task_calendar(window, state),
    };
    if let Err(e) = result {
        tracing::error!(error = %e, prev, "failed to refresh after task form");
    }
}

/// The eight stable category keys, in the same order as the
/// `TaskCategory` enum declaration. Kept in sync with
/// `pomone_app::list_task_category_options` (and ultimately the codec).
pub(crate) fn all_category_keys() -> Vec<String> {
    list_task_category_options()
        .into_iter()
        .map(|o| o.key)
        .collect()
}

/// Number of canonical categories — kept as a function to stay in lockstep
/// with `all_category_keys` if a new variant ever lands.
pub(crate) fn category_count_total() -> usize {
    list_task_category_options().len()
}

/// Stable-string → `TaskCategory` lookup. `None` for unknown keys so the
/// caller can decide whether to error or silently skip (the calendar
/// filter chooses the latter — a stale key just means "this filter does
/// nothing now", which is recoverable).
pub(crate) fn category_from_key(key: &str) -> Option<pomone_domain::TaskCategory> {
    match key {
        "sow" => Some(pomone_domain::TaskCategory::Sow),
        "transplant" => Some(pomone_domain::TaskCategory::Transplant),
        "harvest" => Some(pomone_domain::TaskCategory::Harvest),
        "weeding" => Some(pomone_domain::TaskCategory::Weeding),
        "irrigation" => Some(pomone_domain::TaskCategory::Irrigation),
        "treatment" => Some(pomone_domain::TaskCategory::Treatment),
        "tillage" => Some(pomone_domain::TaskCategory::Tillage),
        "other" => Some(pomone_domain::TaskCategory::Other),
        _ => None,
    }
}

/// Map the codec category string to its codec spelling. Mirrors
/// `pomone_app::tasks_view::category_str` (which is `pub(crate)` and not
/// reachable from here); kept private so a future refactor can replace it
/// with the shared helper without touching call-sites.
pub(crate) fn category_str_for(c: pomone_domain::TaskCategory) -> &'static str {
    match c {
        pomone_domain::TaskCategory::Sow => "sow",
        pomone_domain::TaskCategory::Transplant => "transplant",
        pomone_domain::TaskCategory::Harvest => "harvest",
        pomone_domain::TaskCategory::Weeding => "weeding",
        pomone_domain::TaskCategory::Irrigation => "irrigation",
        pomone_domain::TaskCategory::Treatment => "treatment",
        pomone_domain::TaskCategory::Tillage => "tillage",
        pomone_domain::TaskCategory::Other => "other",
    }
}

/// Pixel size of the bed-usage plot. These MUST match the `BedUsageChart`
/// `month-width` (80px) and `plot-height` (150px) in `home.slint`: the Slint
/// `Path` viewbox is set to these exact pixel extents so the polyline maps 1:1
/// to the element. A viewbox in abstract units would be scaled while
/// *preserving aspect ratio*, squishing a wide-but-short curve into a thin
/// vertical spike — which is why we work in pixels here.
pub(crate) const PLOT_MONTH_WIDTH_PX: f64 = 80.0;
pub(crate) const PLOT_HEIGHT_PX: f64 = 150.0;
/// Width, in day-of-year columns, of the shared season axis used by the Gantt,
/// its today-line, and the bed-usage curve. Fixed at 365 so the mapping is
/// consistent; leap-year day 366 folds onto the last column (#67).
pub(crate) const AXIS_DAYS: i32 = 365;
/// Days mapped across the full 12-month width — matches the Gantt's axis so a
/// week sits under the bars covering it.
pub(crate) const PLOT_TOTAL_DAYS: f64 = AXIS_DAYS as f64;

/// First day of `(year, month)` as a `NaiveDate`. Panics only if the inputs
/// are out of `chrono`'s range, which the UI cannot produce.
pub(crate) fn first_of_month(year: i32, month: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, 1).expect("valid year/month from calendar state")
}

/// Map a `Weekday` to its 0-based offset with Monday as the first day of the
/// week (Mon=0, Sun=6). The calendar grid is rendered Monday-first.
pub(crate) fn weekday_offset_mon(d: NaiveDate) -> u32 {
    match d.weekday() {
        Weekday::Mon => 0,
        Weekday::Tue => 1,
        Weekday::Wed => 2,
        Weekday::Thu => 3,
        Weekday::Fri => 4,
        Weekday::Sat => 5,
        Weekday::Sun => 6,
    }
}

/// Convert `CalendarEventKind` to the numeric `kind` carried by the Slint
/// `CalendarEvent` struct.
pub(crate) fn kind_to_int(k: CalendarEventKind) -> i32 {
    match k {
        CalendarEventKind::Sowing => 0,
        CalendarEventKind::Transplanting => 1,
        CalendarEventKind::HarvestStart => 2,
        CalendarEventKind::HarvestEnd => 3,
        CalendarEventKind::Establishment => 4,
        CalendarEventKind::Removal => 5,
        CalendarEventKind::BudBreak => 6,
        CalendarEventKind::Flowering => 7,
    }
}

/// Fluent key for the single-glyph badge of an event kind.
pub(crate) fn kind_glyph_key(k: CalendarEventKind) -> &'static str {
    match k {
        CalendarEventKind::Sowing => "event-sowing-glyph",
        CalendarEventKind::Transplanting => "event-transplanting-glyph",
        CalendarEventKind::HarvestStart => "event-harvest-start-glyph",
        CalendarEventKind::HarvestEnd => "event-harvest-end-glyph",
        CalendarEventKind::Establishment => "event-establishment-glyph",
        CalendarEventKind::Removal => "event-removal-glyph",
        CalendarEventKind::BudBreak => "event-bud-break-glyph",
        CalendarEventKind::Flowering => "event-flowering-glyph",
    }
}
