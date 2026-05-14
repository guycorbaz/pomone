//! Pomone desktop UI binary entry point (Slint).
//!
//! Three screens routed by `current-page`: Home (counts + language toggle),
//! Plantings (list + add form), Cultures (crops + varieties master-detail).
//! The Rust side owns all data — translations come from Fluent and lists
//! come from the repository through the `pomone_app::*_view` helpers — and
//! feeds it to Slint via plain in-properties.

use std::cell::RefCell;
use std::rc::Rc;

use anyhow::{Context, Result};
use chrono::{Datelike, Days, Local, NaiveDate, Weekday};
use fluent::FluentArgs;
use pomone_app::{
    create_crop, create_location, create_strata, create_variety, delete_strata,
    get_planting_detail, list_crops, list_events_in_range, list_family_options,
    list_location_kind_options, list_location_options, list_locations_tree, list_parent_options,
    list_plantings, list_strata_options, list_strata_rows, list_varieties_for_crop,
    list_variety_options, list_yearly_harvests_for_planting, parse_id, services, test_backend,
    App, AppConfig, AppError, BackendConfig, CalendarEvent as AppCalendarEvent, CalendarEventKind,
    CropInput, CropRow as AppCropRow, FamilyOption, Lang, LifespanKind, LocationInput,
    LocationKindOption, LocationListItem, LocationOption, MigrationReport, ParentLocationOption,
    PlantingDetail as AppPlantingDetail, PlantingRow as AppPlantingRow, StrataInput, StrataOption,
    StrataRow as AppStrataRow, VarietyInput, VarietyOption, VarietyProfileKind,
    VarietyRow as AppVarietyRow, YearlyHarvestRow as AppYearlyHarvestRow,
};
use pomone_domain::{LocationId, PlantingId, PruningSeason, VarietyId};
use rust_decimal::Decimal;
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
use std::path::PathBuf;
use std::str::FromStr;

#[allow(
    unsafe_code,
    missing_debug_implementations,
    unreachable_pub,
    clippy::all,
    clippy::pedantic
)]
mod generated {
    slint::include_modules!();
}
use generated::{
    CalendarDay as SlintCalendarDay, CalendarEvent as SlintCalendarEvent, CropRow as SlintCropRow,
    DetailLine as SlintDetailLine, LocationItem as SlintLocationItem, MainWindow,
    PlantingRow as SlintPlantingRow, StrataItem as SlintStrataItem, VarietyRow as SlintVarietyRow,
    YearlyHarvestRow as SlintYearlyHarvestRow,
};

/// Mutable, single-threaded UI state. Slint runs on the main thread and tokio
/// drives async DB calls via `Runtime::block_on` inside callbacks (SQLite
/// queries finish in microseconds — blocking the UI thread is fine here).
struct UiState {
    app: App,
    runtime: tokio::runtime::Runtime,
    /// Stringified `VarietyId`s, parallel to the Plantings page `variety-labels`.
    variety_ids: Vec<String>,
    /// Parallel to `variety_ids`: tells the planting form which date fields
    /// to show (sowing for annuals, establishment + removal for perennials).
    variety_is_annuals_plantings: Vec<bool>,
    /// Stringified `LocationId`s, parallel to the Plantings page `location-labels`.
    location_ids: Vec<String>,
    /// Stringified `FamilyId`s, parallel to the Cultures page `family-labels`.
    family_ids: Vec<String>,
    /// Stringified `StrataId`s, parallel to the Cultures page `strata-labels`.
    strata_ids: Vec<String>,
    /// Stringified `CropId`s, parallel to the Cultures page `crops` model
    /// (so a row click index resolves to a typed `CropId`).
    crop_ids: Vec<String>,
    /// Parallel to `crop_ids`: tells the UI which variety form to show.
    crop_is_annuals: Vec<bool>,
    /// Stringified `LocationKindId`s, parallel to the Locations page
    /// `loc-kind-labels` model.
    location_kind_ids: Vec<String>,
    /// Stringified `LocationId`s, parallel to the Locations page
    /// `loc-parent-labels` model. The first entry is an empty string for the
    /// synthetic "(no parent)" option.
    parent_location_ids: Vec<String>,
    /// Year currently displayed by the Calendar screen.
    calendar_year: i32,
    /// Month (1..=12) currently displayed by the Calendar screen.
    calendar_month: u32,
    /// Page to return to when the Back button is pressed on the detail
    /// screen. Stored at the moment the user opens a planting so the
    /// detail view can route back to either the list or the calendar.
    detail_previous_page: String,
    /// Stringified `PlantingId` currently shown on the detail screen.
    /// Needed by the yearly-harvest "Record" callback, which doesn't get
    /// the id passed back through Slint.
    detail_planting_id: String,
}

// Setting up four panes' worth of callbacks in one place keeps the flow easy
// to follow; clippy's 100-line limit is too tight for a UI entry point.
#[allow(clippy::too_many_lines)]
fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    tracing::info!("pomone starting");

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("failed to build tokio runtime")?;

    let config = AppConfig::load_or_default().context("failed to load config")?;
    tracing::info!(?config.backend, lang = %config.language, "loaded config");
    let app = runtime
        .block_on(App::new(config))
        .context("failed to initialise App (DB connection / migrations / seed)")?;

    let today_local = Local::now().date_naive();
    let state = Rc::new(RefCell::new(UiState {
        app,
        runtime,
        variety_ids: Vec::new(),
        variety_is_annuals_plantings: Vec::new(),
        location_ids: Vec::new(),
        family_ids: Vec::new(),
        strata_ids: Vec::new(),
        crop_ids: Vec::new(),
        crop_is_annuals: Vec::new(),
        location_kind_ids: Vec::new(),
        parent_location_ids: Vec::new(),
        calendar_year: today_local.year(),
        calendar_month: today_local.month(),
        detail_previous_page: "plantings".to_owned(),
        detail_planting_id: String::new(),
    }));

    let window = MainWindow::new().context("failed to create MainWindow")?;
    let today = today_iso();
    window.set_sown_on_text(SharedString::from(today.clone()));
    window.set_established_on_text(SharedString::from(today));

    apply_translations(&window, &state.borrow().app);
    refresh_counts(&window, &state.borrow().app, &state.borrow().runtime);
    refresh_plantings(&window, &mut state.borrow_mut())?;
    refresh_cultures(&window, &mut state.borrow_mut())?;
    refresh_locations(&window, &mut state.borrow_mut())?;
    refresh_calendar(&window, &mut state.borrow_mut())?;
    refresh_strata(&window, &mut state.borrow_mut())?;
    refresh_settings(&window, &state.borrow());

    // --- Home navigation (sidebar) — refresh counts on entry ---
    {
        let state = Rc::clone(&state);
        let weak = window.as_weak();
        window.on_navigate_home(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let s = state.borrow();
            refresh_counts(&window, &s.app, &s.runtime);
        });
    }
    {
        let state = Rc::clone(&state);
        let weak = window.as_weak();
        window.on_toggle_language(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let mut s = state.borrow_mut();
            let next = match s.app.i18n().lang() {
                Lang::Fr => Lang::En,
                Lang::En => Lang::Fr,
            };
            s.app.set_lang(next);
            apply_translations(&window, &s.app);
        });
    }
    {
        let state = Rc::clone(&state);
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
        let state = Rc::clone(&state);
        let weak = window.as_weak();
        window.on_create_planting(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let mut s = state.borrow_mut();
            match try_create_planting(&window, &mut s) {
                Ok(()) => {
                    let i18n = s.app.i18n();
                    window.set_status_text(SharedString::from(i18n.t("status-planting-created")));
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

    // --- Cultures navigation ---
    {
        let state = Rc::clone(&state);
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
        let state = Rc::clone(&state);
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
        });
    }

    // --- Create crop ---
    {
        let state = Rc::clone(&state);
        let weak = window.as_weak();
        window.on_create_crop(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let mut s = state.borrow_mut();
            match try_create_crop(&window, &mut s) {
                Ok(()) => {
                    let i18n = s.app.i18n();
                    window.set_status_text(SharedString::from(i18n.t("status-crop-created")));
                    window.set_status_is_error(false);
                    window.set_new_crop_name(SharedString::from(""));
                    window.set_new_crop_latin(SharedString::from(""));
                    if let Err(e) = refresh_cultures(&window, &mut s) {
                        tracing::error!(error = %e, "failed to refresh cultures after create");
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

    // --- Locations navigation ---
    {
        let state = Rc::clone(&state);
        let weak = window.as_weak();
        window.on_navigate_locations(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            if let Err(e) = refresh_locations(&window, &mut state.borrow_mut()) {
                tracing::error!(error = %e, "failed to refresh locations");
            }
            window.set_current_page(SharedString::from("locations"));
            window.set_status_text(SharedString::from(""));
            window.set_status_is_error(false);
        });
    }

    // --- Create location ---
    {
        let state = Rc::clone(&state);
        let weak = window.as_weak();
        window.on_create_location(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let mut s = state.borrow_mut();
            match try_create_location(&window, &mut s) {
                Ok(()) => {
                    let i18n = s.app.i18n();
                    window.set_status_text(SharedString::from(i18n.t("status-location-created")));
                    window.set_status_is_error(false);
                    window.set_new_loc_name(SharedString::from(""));
                    window.set_new_loc_notes(SharedString::from(""));
                    if let Err(e) = refresh_locations(&window, &mut s) {
                        tracing::error!(error = %e, "failed to refresh locations after create");
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

    // --- Strata navigation + create + delete ---
    {
        let state = Rc::clone(&state);
        let weak = window.as_weak();
        window.on_navigate_strata(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            if let Err(e) = refresh_strata(&window, &mut state.borrow_mut()) {
                tracing::error!(error = %e, "failed to refresh strata");
            }
            window.set_current_page(SharedString::from("strata"));
            window.set_strata_status_text(SharedString::from(""));
            window.set_strata_status_is_error(false);
        });
    }
    {
        let state = Rc::clone(&state);
        let weak = window.as_weak();
        window.on_create_strata(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let mut s = state.borrow_mut();
            match try_create_strata(&window, &mut s) {
                Ok(()) => {
                    let i18n = s.app.i18n();
                    window.set_strata_status_text(SharedString::from(
                        i18n.t("status-strata-created"),
                    ));
                    window.set_strata_status_is_error(false);
                    window.set_new_strata_name(SharedString::from(""));
                    window.set_new_strata_description(SharedString::from(""));
                    window.set_new_strata_min_height(SharedString::from(""));
                    window.set_new_strata_max_height(SharedString::from(""));
                    if let Err(e) = refresh_strata(&window, &mut s) {
                        tracing::error!(error = %e, "failed to refresh strata after create");
                    }
                    // Counts on the home page include strata; refresh too.
                    refresh_counts(&window, &s.app, &s.runtime);
                }
                Err(e) => {
                    let (text, is_err) = render_form_error(s.app.i18n(), e);
                    window.set_strata_status_text(text);
                    window.set_strata_status_is_error(is_err);
                }
            }
        });
    }
    {
        let state = Rc::clone(&state);
        let weak = window.as_weak();
        window.on_delete_strata(move |id| {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let mut s = state.borrow_mut();
            let result: Result<(), AppError> = s
                .runtime
                .block_on(async { delete_strata(s.app.repo(), &id).await });
            match result {
                Ok(()) => {
                    let i18n = s.app.i18n();
                    window.set_strata_status_text(SharedString::from(
                        i18n.t("status-strata-deleted"),
                    ));
                    window.set_strata_status_is_error(false);
                    if let Err(e) = refresh_strata(&window, &mut s) {
                        tracing::error!(error = %e, "failed to refresh strata after delete");
                    }
                    refresh_counts(&window, &s.app, &s.runtime);
                }
                Err(e) => {
                    let (text, is_err) = render_form_error(s.app.i18n(), FormError::Service(e));
                    window.set_strata_status_text(text);
                    window.set_strata_status_is_error(is_err);
                }
            }
        });
    }

    // --- Settings navigation + test / save / save-and-migrate ---
    {
        let state = Rc::clone(&state);
        let weak = window.as_weak();
        window.on_navigate_settings(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            refresh_settings(&window, &state.borrow());
            window.set_current_page(SharedString::from("settings"));
            window.set_settings_status_text(SharedString::from(""));
            window.set_settings_status_is_error(false);
        });
    }
    {
        let state = Rc::clone(&state);
        let weak = window.as_weak();
        window.on_settings_test_backend(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let s = state.borrow();
            let new_backend = match read_backend_from_form(&window) {
                Ok(b) => b,
                Err(text) => {
                    window.set_settings_status_text(SharedString::from(text));
                    window.set_settings_status_is_error(true);
                    return;
                }
            };
            match s.runtime.block_on(test_backend(&new_backend)) {
                Ok(()) => {
                    window.set_settings_status_text(SharedString::from(
                        s.app.i18n().t("settings-test-ok"),
                    ));
                    window.set_settings_status_is_error(false);
                }
                Err(e) => {
                    let mut args = FluentArgs::new();
                    args.set("message", e.to_string());
                    window.set_settings_status_text(SharedString::from(
                        s.app.i18n().t_args("status-planting-failed", &args),
                    ));
                    window.set_settings_status_is_error(true);
                }
            }
        });
    }
    {
        let state = Rc::clone(&state);
        let weak = window.as_weak();
        window.on_settings_save_backend(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            try_swap_backend(&window, state.clone(), false);
        });
    }
    {
        let state = Rc::clone(&state);
        let weak = window.as_weak();
        window.on_settings_save_and_migrate(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            try_swap_backend(&window, state.clone(), true);
        });
    }

    // --- Planting row click → open detail ---
    {
        let state = Rc::clone(&state);
        let weak = window.as_weak();
        window.on_planting_row_clicked(move |pid| {
            let Some(window) = weak.upgrade() else {
                return;
            };
            open_planting_detail(&window, &mut state.borrow_mut(), &pid, "plantings");
        });
    }

    // --- Calendar event click → open detail (back goes to calendar) ---
    {
        let state = Rc::clone(&state);
        let weak = window.as_weak();
        window.on_calendar_event_clicked(move |pid| {
            let Some(window) = weak.upgrade() else {
                return;
            };
            open_planting_detail(&window, &mut state.borrow_mut(), &pid, "calendar");
        });
    }

    // --- Record yearly harvest from the detail screen ---
    {
        let state = Rc::clone(&state);
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

    // --- Detail "Back" button ---
    {
        let state = Rc::clone(&state);
        let weak = window.as_weak();
        window.on_detail_go_back(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let mut s = state.borrow_mut();
            let target = s.detail_previous_page.clone();
            // Refresh the destination so it picks up any changes made while
            // the user was browsing the detail. Default to "plantings" if
            // the stored previous-page value is unknown.
            match target.as_str() {
                "calendar" => {
                    if let Err(e) = refresh_calendar(&window, &mut s) {
                        tracing::error!(error = %e, "refresh calendar on back");
                    }
                }
                _ => {
                    if let Err(e) = refresh_plantings(&window, &mut s) {
                        tracing::error!(error = %e, "refresh plantings on back");
                    }
                }
            }
            window.set_current_page(SharedString::from(target));
            window.set_status_text(SharedString::from(""));
            window.set_status_is_error(false);
        });
    }

    // --- Calendar navigation + month nav ---
    {
        let state = Rc::clone(&state);
        let weak = window.as_weak();
        window.on_navigate_calendar(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            if let Err(e) = refresh_calendar(&window, &mut state.borrow_mut()) {
                tracing::error!(error = %e, "failed to refresh calendar");
            }
            window.set_current_page(SharedString::from("calendar"));
            window.set_status_text(SharedString::from(""));
            window.set_status_is_error(false);
        });
    }
    {
        let state = Rc::clone(&state);
        let weak = window.as_weak();
        window.on_prev_month(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            {
                let mut s = state.borrow_mut();
                let (y, m) = prev_month(s.calendar_year, s.calendar_month);
                s.calendar_year = y;
                s.calendar_month = m;
            }
            if let Err(e) = refresh_calendar(&window, &mut state.borrow_mut()) {
                tracing::error!(error = %e, "failed to refresh calendar");
            }
        });
    }
    {
        let state = Rc::clone(&state);
        let weak = window.as_weak();
        window.on_next_month(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            {
                let mut s = state.borrow_mut();
                let (y, m) = next_month(s.calendar_year, s.calendar_month);
                s.calendar_year = y;
                s.calendar_month = m;
            }
            if let Err(e) = refresh_calendar(&window, &mut state.borrow_mut()) {
                tracing::error!(error = %e, "failed to refresh calendar");
            }
        });
    }
    {
        let state = Rc::clone(&state);
        let weak = window.as_weak();
        window.on_go_today(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            {
                let mut s = state.borrow_mut();
                let now = Local::now().date_naive();
                s.calendar_year = now.year();
                s.calendar_month = now.month();
            }
            if let Err(e) = refresh_calendar(&window, &mut state.borrow_mut()) {
                tracing::error!(error = %e, "failed to refresh calendar");
            }
        });
    }

    // --- Create variety ---
    {
        let state = Rc::clone(&state);
        let weak = window.as_weak();
        window.on_create_variety(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let mut s = state.borrow_mut();
            match try_create_variety(&window, &mut s) {
                Ok(()) => {
                    let i18n = s.app.i18n();
                    window.set_status_text(SharedString::from(i18n.t("status-variety-created")));
                    window.set_status_is_error(false);
                    window.set_new_variety_name(SharedString::from(""));
                    window.set_new_variety_description(SharedString::from(""));
                    if let Err(e) = refresh_cultures(&window, &mut s) {
                        tracing::error!(error = %e, "failed to refresh cultures after create");
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

    window.run().context("Slint event loop failed")?;
    Ok(())
}

/// Refresh every string the UI displays based on the active language.
// Five panes' worth of labels in one place keeps the flow easy to follow;
// clippy's 100-line cap is too tight for a UI translation broadcast.
#[allow(clippy::too_many_lines)]
fn apply_translations(window: &MainWindow, app: &App) {
    let i18n = app.i18n();
    window.set_title_text(SharedString::from("Pomone"));
    window.set_welcome_text(SharedString::from(i18n.t("welcome-summary")));
    window.set_version_text(SharedString::from(format!(
        "v{}",
        env!("CARGO_PKG_VERSION")
    )));
    window.set_label_strata(SharedString::from(i18n.t("label-strata-count")));
    window.set_label_families(SharedString::from(i18n.t("label-families-count")));
    window.set_label_location_kinds(SharedString::from(i18n.t("label-location-kinds-count")));
    window.set_section_overview_text(SharedString::from(i18n.t("section-overview")));
    window.set_language_button_text(SharedString::from(i18n.t("button-switch-language")));
    window.set_current_language_tag(SharedString::from(i18n.lang().tag()));

    // Sidebar nav
    window.set_nav_home_text(SharedString::from(i18n.t("nav-home")));
    window.set_nav_plantings_text(SharedString::from(i18n.t("nav-plantings")));
    window.set_nav_cultures_text(SharedString::from(i18n.t("nav-cultures")));
    window.set_nav_locations_text(SharedString::from(i18n.t("nav-locations")));
    window.set_nav_calendar_text(SharedString::from(i18n.t("nav-calendar")));
    window.set_nav_strata_text(SharedString::from(i18n.t("nav-strata")));

    // Strata page — static labels; the list and status come from refresh_strata.
    window.set_strata_title_text(SharedString::from(i18n.t("title-strata")));
    window.set_strata_list_title(SharedString::from(i18n.t("strata-list-title")));
    window.set_strata_empty_text(SharedString::from(i18n.t("empty-strata")));
    window.set_strata_delete_text(SharedString::from(i18n.t("button-delete")));
    window.set_strata_in_use_text(SharedString::from(i18n.t("strata-in-use")));
    window.set_strata_form_section(SharedString::from(i18n.t("section-new-strata")));
    window.set_strata_label_name(SharedString::from(i18n.t("label-strata-name")));
    window.set_strata_placeholder_name(SharedString::from(i18n.t("placeholder-strata-name")));
    window.set_strata_label_description(SharedString::from(i18n.t("label-strata-description")));
    window.set_strata_placeholder_description(SharedString::from(
        i18n.t("placeholder-strata-description"),
    ));
    window.set_strata_label_height_min(SharedString::from(i18n.t("label-strata-height-min")));
    window.set_strata_placeholder_height_min(SharedString::from(
        i18n.t("placeholder-strata-height-min"),
    ));
    window.set_strata_label_height_max(SharedString::from(i18n.t("label-strata-height-max")));
    window.set_strata_placeholder_height_max(SharedString::from(
        i18n.t("placeholder-strata-height-max"),
    ));
    window.set_strata_label_sort_order(SharedString::from(i18n.t("label-strata-sort-order")));
    window.set_strata_placeholder_sort_order(SharedString::from(
        i18n.t("placeholder-strata-sort-order"),
    ));
    window.set_strata_create_button_text(SharedString::from(i18n.t("button-create-strata")));

    // Settings page — static labels; the current-backend display is
    // refreshed by `refresh_settings`.
    window.set_nav_settings_text(SharedString::from(i18n.t("nav-settings")));
    window.set_settings_title_text(SharedString::from(i18n.t("title-settings")));
    window.set_settings_current_section(SharedString::from(i18n.t("settings-current-section")));
    window.set_settings_current_label(SharedString::from(i18n.t("settings-current-label")));
    window.set_settings_edit_section(SharedString::from(i18n.t("settings-edit-section")));
    window.set_settings_backend_kind_label(SharedString::from(i18n.t("settings-backend-kind-label")));
    let backend_kind_labels: Vec<SharedString> = [
        i18n.t("settings-backend-sqlite"),
        i18n.t("settings-backend-mariadb"),
    ]
    .into_iter()
    .map(SharedString::from)
    .collect();
    window.set_settings_backend_kind_labels(ModelRc::new(VecModel::from(backend_kind_labels)));
    window.set_settings_sqlite_path_label(SharedString::from(i18n.t("settings-sqlite-path-label")));
    window.set_settings_sqlite_path_placeholder(SharedString::from(
        i18n.t("settings-sqlite-path-placeholder"),
    ));
    window.set_settings_mariadb_host_label(SharedString::from(i18n.t("settings-mariadb-host-label")));
    window.set_settings_mariadb_host_placeholder(SharedString::from(
        i18n.t("settings-mariadb-host-placeholder"),
    ));
    window.set_settings_mariadb_port_label(SharedString::from(i18n.t("settings-mariadb-port-label")));
    window.set_settings_mariadb_port_placeholder(SharedString::from(
        i18n.t("settings-mariadb-port-placeholder"),
    ));
    window.set_settings_mariadb_user_label(SharedString::from(i18n.t("settings-mariadb-user-label")));
    window.set_settings_mariadb_user_placeholder(SharedString::from(
        i18n.t("settings-mariadb-user-placeholder"),
    ));
    window.set_settings_mariadb_password_label(SharedString::from(
        i18n.t("settings-mariadb-password-label"),
    ));
    window.set_settings_mariadb_password_placeholder(SharedString::from(
        i18n.t("settings-mariadb-password-placeholder"),
    ));
    window.set_settings_mariadb_database_label(SharedString::from(
        i18n.t("settings-mariadb-database-label"),
    ));
    window.set_settings_mariadb_database_placeholder(SharedString::from(
        i18n.t("settings-mariadb-database-placeholder"),
    ));
    window.set_settings_test_button(SharedString::from(i18n.t("settings-button-test")));
    window.set_settings_save_button(SharedString::from(i18n.t("settings-button-save")));
    window.set_settings_save_migrate_button(SharedString::from(
        i18n.t("settings-button-save-migrate"),
    ));
    window.set_settings_migrate_warning(SharedString::from(i18n.t("settings-migrate-warning")));

    // Calendar — labels + legend; the day grid is rebuilt on every refresh
    window.set_calendar_title_text(SharedString::from(i18n.t("title-calendar")));
    window.set_calendar_prev_button_text(SharedString::from(i18n.t("calendar-prev")));
    window.set_calendar_next_button_text(SharedString::from(i18n.t("calendar-next")));
    window.set_calendar_today_button_text(SharedString::from(i18n.t("calendar-today")));
    window.set_calendar_empty_state_text(SharedString::from(i18n.t("calendar-empty")));
    let weekday_labels: Vec<SharedString> = [
        i18n.t("weekday-mon-short"),
        i18n.t("weekday-tue-short"),
        i18n.t("weekday-wed-short"),
        i18n.t("weekday-thu-short"),
        i18n.t("weekday-fri-short"),
        i18n.t("weekday-sat-short"),
        i18n.t("weekday-sun-short"),
    ]
    .into_iter()
    .map(SharedString::from)
    .collect();
    window.set_calendar_weekday_labels(ModelRc::new(VecModel::from(weekday_labels)));
    let kind_labels: Vec<SharedString> = [
        i18n.t("event-sowing-label"),
        i18n.t("event-transplanting-label"),
        i18n.t("event-harvest-start-label"),
        i18n.t("event-harvest-end-label"),
        i18n.t("event-establishment-label"),
        i18n.t("event-removal-label"),
        i18n.t("event-bud-break-label"),
        i18n.t("event-flowering-label"),
    ]
    .into_iter()
    .map(SharedString::from)
    .collect();
    window.set_calendar_kind_labels(ModelRc::new(VecModel::from(kind_labels)));

    // Plantings page
    window.set_plantings_title_text(SharedString::from(i18n.t("title-plantings")));
    window.set_empty_state_text(SharedString::from(i18n.t("empty-plantings")));
    window.set_section_new_text(SharedString::from(i18n.t("section-new-planting")));
    window.set_label_variety(SharedString::from(i18n.t("label-variety")));
    window.set_label_location(SharedString::from(i18n.t("label-location")));
    window.set_label_sown_on(SharedString::from(i18n.t("label-sown-on")));
    window.set_label_established_on(SharedString::from(i18n.t("label-established-on")));
    window.set_label_removal_on(SharedString::from(i18n.t("label-removal-on")));
    window.set_placeholder_removal_date(SharedString::from(i18n.t("placeholder-removal-date")));
    window.set_label_area(SharedString::from(i18n.t("label-area")));
    window.set_label_count(SharedString::from(i18n.t("label-plants-count")));
    window.set_placeholder_date(SharedString::from(i18n.t("placeholder-date")));
    window.set_placeholder_area(SharedString::from(i18n.t("placeholder-area")));
    window.set_placeholder_count(SharedString::from(i18n.t("placeholder-count")));
    window.set_create_button_text(SharedString::from(i18n.t("button-create-planting")));
    window.set_plants_suffix(SharedString::from(i18n.t("plants-suffix")));

    // Cultures page
    window.set_cultures_title_text(SharedString::from(i18n.t("title-cultures")));
    window.set_crops_title(SharedString::from(i18n.t("crops-title")));
    window.set_empty_crops_text(SharedString::from(i18n.t("empty-crops")));
    window.set_varieties_title(SharedString::from(i18n.t("varieties-title")));
    window.set_empty_varieties_text(SharedString::from(i18n.t("empty-varieties")));
    window.set_no_crop_selected_text(SharedString::from(i18n.t("no-crop-selected")));
    window.set_new_crop_section(SharedString::from(i18n.t("new-crop-section")));
    window.set_new_variety_section(SharedString::from(i18n.t("new-variety-section")));
    window.set_label_crop_name(SharedString::from(i18n.t("label-crop-name")));
    window.set_placeholder_crop_name(SharedString::from(i18n.t("placeholder-crop-name")));
    window.set_label_crop_latin(SharedString::from(i18n.t("label-crop-latin")));
    window.set_placeholder_crop_latin(SharedString::from(i18n.t("placeholder-crop-latin")));
    window.set_label_crop_family(SharedString::from(i18n.t("label-crop-family")));
    window.set_label_crop_strata(SharedString::from(i18n.t("label-crop-strata")));
    window.set_label_lifespan(SharedString::from(i18n.t("label-lifespan")));
    window.set_label_lifespan_years(SharedString::from(i18n.t("label-lifespan-years")));
    window.set_placeholder_lifespan_years(SharedString::from(i18n.t("placeholder-lifespan-years")));
    window.set_label_years_to_first_yield(SharedString::from(i18n.t("label-years-to-first-yield")));
    window.set_placeholder_years_to_first_yield(SharedString::from(
        i18n.t("placeholder-years-to-first-yield"),
    ));
    window.set_label_pruning(SharedString::from(i18n.t("label-pruning")));
    let lifespan_labels: Vec<SharedString> = [
        i18n.t("lifespan-annual"),
        i18n.t("lifespan-pluriannual-single"),
        i18n.t("lifespan-pluriannual-recurring"),
    ]
    .into_iter()
    .map(SharedString::from)
    .collect();
    window.set_lifespan_labels(ModelRc::new(VecModel::from(lifespan_labels)));
    let pruning_labels: Vec<SharedString> = [
        i18n.t("pruning-none-label"),
        i18n.t("pruning-winter-label"),
        i18n.t("pruning-summer-label"),
        i18n.t("pruning-both-label"),
    ]
    .into_iter()
    .map(SharedString::from)
    .collect();
    window.set_pruning_labels(ModelRc::new(VecModel::from(pruning_labels)));
    window.set_label_bud_break_doy(SharedString::from(i18n.t("label-bud-break-doy")));
    window.set_placeholder_bud_break_doy(SharedString::from(i18n.t("placeholder-bud-break-doy")));
    window.set_label_flowering_doy(SharedString::from(i18n.t("label-flowering-doy")));
    window.set_placeholder_flowering_doy(SharedString::from(i18n.t("placeholder-flowering-doy")));
    window.set_label_harvest_start_doy(SharedString::from(i18n.t("label-harvest-start-doy")));
    window.set_placeholder_harvest_start_doy(SharedString::from(
        i18n.t("placeholder-harvest-start-doy"),
    ));
    window.set_label_harvest_end_doy(SharedString::from(i18n.t("label-harvest-end-doy")));
    window
        .set_placeholder_harvest_end_doy(SharedString::from(i18n.t("placeholder-harvest-end-doy")));
    window.set_label_yield_kg(SharedString::from(i18n.t("label-yield-kg")));
    window.set_placeholder_yield_kg(SharedString::from(i18n.t("placeholder-yield-kg")));
    window.set_label_variety_name(SharedString::from(i18n.t("label-variety-name")));
    window.set_placeholder_variety_name(SharedString::from(i18n.t("placeholder-variety-name")));
    window.set_label_variety_description(SharedString::from(i18n.t("label-variety-description")));
    window.set_placeholder_variety_description(SharedString::from(
        i18n.t("placeholder-variety-description"),
    ));
    window.set_label_dtt(SharedString::from(i18n.t("label-dtt")));
    window.set_label_dtm(SharedString::from(i18n.t("label-dtm")));
    window.set_label_window(SharedString::from(i18n.t("label-window")));
    window.set_placeholder_dtt(SharedString::from(i18n.t("placeholder-dtt")));
    window.set_placeholder_dtm(SharedString::from(i18n.t("placeholder-dtm")));
    window.set_placeholder_window(SharedString::from(i18n.t("placeholder-window")));
    window.set_create_crop_button_text(SharedString::from(i18n.t("button-create-crop")));
    window.set_create_variety_button_text(SharedString::from(i18n.t("button-create-variety")));

    // Planting detail page — static labels only; per-planting data is
    // refreshed by `refresh_planting_detail` whenever a row/event is clicked.
    window.set_detail_title_text(SharedString::from(i18n.t("title-planting-detail")));
    window.set_detail_back_button_text(SharedString::from(i18n.t("button-back")));
    window.set_detail_section_schedule_text(SharedString::from(i18n.t("section-schedule")));
    window.set_detail_section_summary_text(SharedString::from(i18n.t("section-summary")));
    window.set_detail_name_label(SharedString::from(i18n.t("label-planting-name")));
    window.set_detail_notes_label(SharedString::from(i18n.t("label-planting-notes")));
    window.set_detail_empty_state_text(SharedString::from(i18n.t("empty-planting-detail")));

    // Yearly-harvest section labels — content rows come from refresh_planting_detail.
    window.set_harvest_section_title(SharedString::from(i18n.t("section-yearly-harvest")));
    window.set_harvest_empty_text(SharedString::from(i18n.t("empty-yearly-harvest")));
    window.set_harvest_header_year(SharedString::from(i18n.t("harvest-header-year")));
    window.set_harvest_header_expected(SharedString::from(i18n.t("harvest-header-expected")));
    window.set_harvest_header_actual(SharedString::from(i18n.t("harvest-header-actual")));
    window.set_harvest_header_variance(SharedString::from(i18n.t("harvest-header-variance")));
    window.set_harvest_header_notes(SharedString::from(i18n.t("harvest-header-notes")));
    window.set_harvest_form_section(SharedString::from(i18n.t("section-record-harvest")));
    window.set_harvest_label_year(SharedString::from(i18n.t("label-harvest-year")));
    window.set_harvest_label_expected(SharedString::from(i18n.t("label-harvest-expected")));
    window.set_harvest_label_actual(SharedString::from(i18n.t("label-harvest-actual")));
    window.set_harvest_label_notes(SharedString::from(i18n.t("label-harvest-notes")));
    window.set_harvest_placeholder_year(SharedString::from(i18n.t("placeholder-harvest-year")));
    window.set_harvest_placeholder_kg(SharedString::from(i18n.t("placeholder-harvest-kg")));
    window.set_harvest_placeholder_notes(SharedString::from(i18n.t("placeholder-harvest-notes")));
    window.set_harvest_record_button(SharedString::from(i18n.t("button-record-harvest")));

    // Locations page
    window.set_locations_title_text(SharedString::from(i18n.t("title-locations")));
    window.set_locations_list_title(SharedString::from(i18n.t("locations-list-title")));
    window.set_empty_locations_text(SharedString::from(i18n.t("empty-locations")));
    window.set_location_form_section(SharedString::from(i18n.t("new-location-section")));
    window.set_label_loc_name(SharedString::from(i18n.t("label-loc-name")));
    window.set_placeholder_loc_name(SharedString::from(i18n.t("placeholder-loc-name")));
    window.set_label_loc_kind(SharedString::from(i18n.t("label-loc-kind")));
    window.set_label_loc_length(SharedString::from(i18n.t("label-loc-length")));
    window.set_placeholder_loc_length(SharedString::from(i18n.t("placeholder-loc-length")));
    window.set_label_loc_width(SharedString::from(i18n.t("label-loc-width")));
    window.set_placeholder_loc_width(SharedString::from(i18n.t("placeholder-loc-width")));
    window.set_label_loc_parent(SharedString::from(i18n.t("label-loc-parent")));
    window.set_label_loc_notes(SharedString::from(i18n.t("label-loc-notes")));
    window.set_placeholder_loc_notes(SharedString::from(i18n.t("placeholder-loc-notes")));
    window.set_create_loc_button_text(SharedString::from(i18n.t("button-create-location")));
}

fn refresh_counts(window: &MainWindow, app: &App, runtime: &tokio::runtime::Runtime) {
    let repo = app.repo();
    let result = runtime.block_on(async {
        let strata = repo.strata_list().await?;
        let families = repo.family_list().await?;
        let kinds = repo.location_kind_list().await?;
        Ok::<_, pomone_db::DbError>((strata.len(), families.len(), kinds.len()))
    });
    match result {
        Ok((strata, families, kinds)) => {
            window.set_count_strata(usize_to_i32(strata));
            window.set_count_families(usize_to_i32(families));
            window.set_count_location_kinds(usize_to_i32(kinds));
        }
        Err(e) => {
            tracing::error!(error = %e, "failed to refresh counts");
        }
    }
}

/// Snapshot of everything the Plantings screen needs on every refresh.
struct PlantingsSnapshot {
    varieties: Vec<VarietyOption>,
    locations: Vec<LocationOption>,
    plantings: Vec<AppPlantingRow>,
}

/// Reload the plantings list AND the dropdown options for the form. Stores
/// the option IDs in `state` so the create callback can look them up by index.
fn refresh_plantings(window: &MainWindow, state: &mut UiState) -> Result<()> {
    let snapshot: Result<PlantingsSnapshot, AppError> = state.runtime.block_on(async {
        let varieties = list_variety_options(state.app.repo()).await?;
        let locations = list_location_options(state.app.repo()).await?;
        let plantings = list_plantings(state.app.repo()).await?;
        Ok(PlantingsSnapshot {
            varieties,
            locations,
            plantings,
        })
    });
    let snapshot = snapshot.context("failed to load plantings data")?;

    state.variety_ids = snapshot.varieties.iter().map(|v| v.id.clone()).collect();
    state.variety_is_annuals_plantings = snapshot.varieties.iter().map(|v| v.is_annual).collect();
    state.location_ids = snapshot.locations.iter().map(|l| l.id.clone()).collect();

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

    let rows: Vec<SlintPlantingRow> = snapshot.plantings.into_iter().map(to_slint_row).collect();
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

    Ok(())
}

fn to_slint_row(row: AppPlantingRow) -> SlintPlantingRow {
    SlintPlantingRow {
        id: SharedString::from(row.id),
        variety_label: SharedString::from(row.variety_label),
        location_label: SharedString::from(row.location_label),
        schedule_summary: SharedString::from(row.schedule_summary),
        area_label: SharedString::from(row.area_label),
        plants_count: usize_to_i32(row.plants_count as usize),
    }
}

/// Read the form fields, validate them, build typed IDs, and call the right
/// service depending on whether the picked variety is annual or perennial.
/// Client-side validation surfaces localized messages; service-side errors
/// pass through unchanged.
fn try_create_planting(window: &MainWindow, state: &mut UiState) -> Result<(), FormError> {
    let i18n = state.app.i18n();
    let variety_idx = i32_to_usize(window.get_variety_index());
    let location_idx = i32_to_usize(window.get_location_index());
    let variety_id_str = state
        .variety_ids
        .get(variety_idx)
        .ok_or_else(|| FormError::Service(AppError::Inconsistent("no variety selected".into())))?;
    let location_id_str = state
        .location_ids
        .get(location_idx)
        .ok_or_else(|| FormError::Service(AppError::Inconsistent("no location selected".into())))?;
    let is_annual = state
        .variety_is_annuals_plantings
        .get(variety_idx)
        .copied()
        .unwrap_or(true);

    let variety_id: VarietyId = parse_id(variety_id_str).map_err(FormError::Service)?;
    let location_id: LocationId = parse_id(location_id_str).map_err(FormError::Service)?;
    let area_m2 = validate_positive_decimal(&window.get_area_text(), i18n)?;
    let plants_count = validate_positive_count(&window.get_count_text(), i18n)?;

    if is_annual {
        let sown_on = validate_iso_date(&window.get_sown_on_text(), i18n)?;
        state.runtime.block_on(async {
            services::create_annual_planting_from_sowing(
                state.app.repo(),
                variety_id,
                location_id,
                sown_on,
                area_m2,
                plants_count,
                None,
                None,
            )
            .await
            .map(|_| ())
        })?;
    } else {
        let established_on = validate_iso_date(&window.get_established_on_text(), i18n)?;
        let removal_text = window.get_removal_on_text();
        let expected_removal_on = if removal_text.trim().is_empty() {
            None
        } else {
            Some(validate_iso_date(&removal_text, i18n)?)
        };
        state.runtime.block_on(async {
            services::create_perennial_planting(
                state.app.repo(),
                variety_id,
                location_id,
                established_on,
                expected_removal_on,
                area_m2,
                plants_count,
                None,
                None,
            )
            .await
            .map(|_| ())
        })?;
    }
    Ok(())
}

fn parse_u16(s: &str, field: &'static str) -> Result<u16, AppError> {
    s.trim()
        .parse::<u16>()
        .map_err(|e| AppError::Inconsistent(format!("invalid {field} '{s}': {e}")))
}

fn parse_optional_u16(s: &str, field: &'static str) -> Result<Option<u16>, AppError> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    parse_u16(trimmed, field).map(Some)
}

fn optional_text(s: &str) -> Option<String> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_owned())
    }
}

/// Snapshot of everything the Cultures screen needs on every refresh.
struct CulturesSnapshot {
    crops: Vec<AppCropRow>,
    families: Vec<FamilyOption>,
    strata: Vec<StrataOption>,
}

/// Reload crops + dropdown options. Also refreshes the right-side varieties
/// for whichever crop is currently selected (or empties them if none).
fn refresh_cultures(window: &MainWindow, state: &mut UiState) -> Result<()> {
    let snapshot: Result<CulturesSnapshot, AppError> = state.runtime.block_on(async {
        let crops = list_crops(state.app.repo()).await?;
        let families = list_family_options(state.app.repo()).await?;
        let strata = list_strata_options(state.app.repo()).await?;
        Ok(CulturesSnapshot {
            crops,
            families,
            strata,
        })
    });
    let snapshot = snapshot.context("failed to load cultures data")?;

    state.crop_ids = snapshot.crops.iter().map(|c| c.id.clone()).collect();
    state.crop_is_annuals = snapshot.crops.iter().map(|c| c.is_annual).collect();
    state.family_ids = snapshot.families.iter().map(|f| f.id.clone()).collect();
    state.strata_ids = snapshot.strata.iter().map(|s| s.id.clone()).collect();

    let crop_rows: Vec<SlintCropRow> = snapshot.crops.into_iter().map(crop_to_slint).collect();
    window.set_crops(ModelRc::new(VecModel::from(crop_rows)));

    let family_labels: Vec<SharedString> = snapshot
        .families
        .into_iter()
        .map(|f| SharedString::from(f.label))
        .collect();
    window.set_family_labels(ModelRc::new(VecModel::from(family_labels)));

    let strata_labels: Vec<SharedString> = snapshot
        .strata
        .into_iter()
        .map(|s| SharedString::from(s.label))
        .collect();
    window.set_strata_labels(ModelRc::new(VecModel::from(strata_labels)));

    // Clamp form dropdowns; keep selected-crop-index if still valid.
    if i32_to_usize(window.get_family_index()) >= state.family_ids.len() {
        window.set_family_index(0);
    }
    if i32_to_usize(window.get_strata_index()) >= state.strata_ids.len() {
        window.set_strata_index(0);
    }
    let selected_idx = window.get_selected_crop_index();
    if selected_idx < 0 || i32_to_usize(selected_idx) >= state.crop_ids.len() {
        window.set_selected_crop_index(-1);
    }
    refresh_varieties_of_selected_crop(window, state)
}

/// Re-read the variety list for the currently selected crop. If no crop is
/// selected (`selected-crop-index < 0`), the list is cleared.
fn refresh_varieties_of_selected_crop(window: &MainWindow, state: &mut UiState) -> Result<()> {
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

fn crop_to_slint(row: AppCropRow) -> SlintCropRow {
    SlintCropRow {
        id: SharedString::from(row.id),
        name: SharedString::from(row.name),
        family_label: SharedString::from(row.family_label),
        strata_label: SharedString::from(row.strata_label),
        lifespan_label: SharedString::from(row.lifespan_label),
        pruning_label: SharedString::from(row.pruning_label),
        variety_count: usize_to_i32(row.variety_count as usize),
        is_annual: row.is_annual,
    }
}

fn variety_to_slint(row: AppVarietyRow) -> SlintVarietyRow {
    SlintVarietyRow {
        id: SharedString::from(row.id),
        name: SharedString::from(row.name),
        description: SharedString::from(row.description),
        profile_label: SharedString::from(row.profile_label),
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

fn try_create_crop(window: &MainWindow, state: &mut UiState) -> Result<(), FormError> {
    let i18n = state.app.i18n();
    let family_idx = i32_to_usize(window.get_family_index());
    let strata_idx = i32_to_usize(window.get_strata_index());
    let family_id_str = state
        .family_ids
        .get(family_idx)
        .ok_or_else(|| FormError::Service(AppError::Inconsistent("no family selected".into())))?
        .clone();
    let strata_id_str = state
        .strata_ids
        .get(strata_idx)
        .ok_or_else(|| FormError::Service(AppError::Inconsistent("no strata selected".into())))?
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

    state
        .runtime
        .block_on(async {
            create_crop(
                state.app.repo(),
                CropInput {
                    family_id_str,
                    strata_id_str,
                    name,
                    latin_name,
                    lifespan_kind,
                    lifespan_years,
                    years_to_first_yield,
                    pruning_season,
                },
            )
            .await
            .map(|_| ())
        })
        .map_err(FormError::Service)
}

fn try_create_variety(window: &MainWindow, state: &mut UiState) -> Result<(), FormError> {
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

    state
        .runtime
        .block_on(async { create_variety(state.app.repo(), input).await.map(|_| ()) })
        .map_err(FormError::Service)
}

fn parse_u8(s: &str, field: &'static str) -> Result<u8, AppError> {
    s.trim()
        .parse::<u8>()
        .map_err(|e| AppError::Inconsistent(format!("invalid {field} '{s}': {e}")))
}

fn parse_optional_decimal(s: &str, field: &'static str) -> Result<Option<Decimal>, AppError> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    Decimal::from_str(trimmed)
        .map(Some)
        .map_err(|e| AppError::Inconsistent(format!("invalid {field} '{s}': {e}")))
}

/// Snapshot of everything the Locations screen needs on every refresh.
struct LocationsSnapshot {
    items: Vec<LocationListItem>,
    kinds: Vec<LocationKindOption>,
    parents: Vec<ParentLocationOption>,
}

/// Reload the location tree + dropdown options (kinds, parents). Indices are
/// clamped to stay valid after a refresh.
fn refresh_locations(window: &MainWindow, state: &mut UiState) -> Result<()> {
    // "(aucun) / (none)" label for the synthetic root-parent option.
    let none_label = state.app.i18n().t("parent-none");
    let snapshot: Result<LocationsSnapshot, AppError> = state.runtime.block_on(async {
        let items = list_locations_tree(state.app.repo()).await?;
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

fn location_to_slint(item: LocationListItem) -> SlintLocationItem {
    SlintLocationItem {
        id: SharedString::from(item.id),
        name: SharedString::from(item.name),
        kind_label: SharedString::from(item.kind_label),
        area_label: SharedString::from(item.area_label),
        dimensions_label: SharedString::from(item.dimensions_label),
        parent_label: SharedString::from(item.parent_label),
        full_path: SharedString::from(item.full_path),
        depth: usize_to_i32(item.depth as usize),
    }
}

fn try_create_location(window: &MainWindow, state: &mut UiState) -> Result<(), FormError> {
    let i18n = state.app.i18n();
    let kind_idx = i32_to_usize(window.get_loc_kind_index());
    let parent_idx = i32_to_usize(window.get_loc_parent_index());
    let kind_id_str = state
        .location_kind_ids
        .get(kind_idx)
        .ok_or_else(|| {
            FormError::Service(AppError::Inconsistent("no location kind selected".into()))
        })?
        .clone();
    let parent_id_str = state
        .parent_location_ids
        .get(parent_idx)
        .cloned()
        .unwrap_or_default();
    let name = validate_required_name(&window.get_new_loc_name(), i18n)?;
    let length_m = validate_positive_decimal(&window.get_new_loc_length(), i18n)?;
    let width_m = validate_positive_decimal(&window.get_new_loc_width(), i18n)?;
    let notes = optional_text(&window.get_new_loc_notes());

    state
        .runtime
        .block_on(async {
            create_location(
                state.app.repo(),
                LocationInput {
                    kind_id_str,
                    name,
                    length_m,
                    width_m,
                    parent_id_str,
                    notes,
                },
            )
            .await
            .map(|_| ())
        })
        .map_err(FormError::Service)
}

fn today_iso() -> String {
    Local::now().date_naive().format("%Y-%m-%d").to_string()
}

/// Push the active backend onto the Settings header and pre-fill the edit
/// form so the user can tweak it without retyping everything.
fn refresh_settings(window: &MainWindow, state: &UiState) {
    let cfg = state.app.config();
    let value = backend_display(&cfg.backend);
    window.set_settings_current_value(SharedString::from(value));

    match &cfg.backend {
        BackendConfig::Sqlite { path } => {
            window.set_settings_backend_kind_index(0);
            window.set_settings_sqlite_path(SharedString::from(path.display().to_string()));
        }
        BackendConfig::Mariadb { url } => {
            window.set_settings_backend_kind_index(1);
            // Best-effort split of the URL back into structured fields so
            // the user sees something usable. Falls back to leaving fields
            // empty if the URL doesn't match the canonical shape.
            let (host, port, user, password, db) = split_mariadb_url(url);
            window.set_settings_mariadb_host(SharedString::from(host));
            window.set_settings_mariadb_port(SharedString::from(port));
            window.set_settings_mariadb_user(SharedString::from(user));
            window.set_settings_mariadb_password(SharedString::from(password));
            window.set_settings_mariadb_database(SharedString::from(db));
        }
    }
}

/// Human-readable rendering of a backend for the Settings header.
fn backend_display(b: &BackendConfig) -> String {
    match b {
        BackendConfig::Sqlite { path } => format!("SQLite — {}", path.display()),
        BackendConfig::Mariadb { url } => format!("MariaDB — {}", redact_password(url)),
    }
}

/// Replace the password in `mysql://user:pass@host…` with `***` so the
/// banner doesn't leak credentials when the user takes screenshots.
fn redact_password(url: &str) -> String {
    if let Some(scheme_end) = url.find("://") {
        let (scheme, rest) = url.split_at(scheme_end + 3);
        if let Some(at_pos) = rest.find('@') {
            let (creds, tail) = rest.split_at(at_pos);
            if let Some(colon_pos) = creds.find(':') {
                let (user, _) = creds.split_at(colon_pos);
                return format!("{scheme}{user}:***{tail}");
            }
        }
    }
    url.to_owned()
}

/// Best-effort decomposition of a `mysql://user:pass@host:port/db` URL into
/// its five components. Returns empty strings for anything missing.
fn split_mariadb_url(url: &str) -> (String, String, String, String, String) {
    let mut port = "3306".to_owned();
    let mut user = String::new();
    let mut password = String::new();
    let rest = url.strip_prefix("mysql://").unwrap_or(url);
    let (creds, tail) = match rest.find('@') {
        Some(p) => (&rest[..p], &rest[p + 1..]),
        None => ("", rest),
    };
    if !creds.is_empty() {
        if let Some(colon) = creds.find(':') {
            creds[..colon].clone_into(&mut user);
            creds[colon + 1..].clone_into(&mut password);
        } else {
            creds.clone_into(&mut user);
        }
    }
    let (hostport, after) = match tail.find('/') {
        Some(p) => (&tail[..p], &tail[p + 1..]),
        None => (tail, ""),
    };
    let host = if let Some(colon) = hostport.find(':') {
        hostport[colon + 1..].clone_into(&mut port);
        hostport[..colon].to_owned()
    } else {
        hostport.to_owned()
    };
    let db = after.split('?').next().unwrap_or("").to_owned();
    (host, port, user, password, db)
}

/// Read the Settings form and assemble a [`BackendConfig`]. Returns a
/// short, localized validation message on error.
fn read_backend_from_form(window: &MainWindow) -> Result<BackendConfig, String> {
    if window.get_settings_backend_kind_index() == 0 {
        let path = window.get_settings_sqlite_path();
        let trimmed = path.trim();
        if trimmed.is_empty() {
            return Err("SQLite path is required".to_owned());
        }
        Ok(BackendConfig::Sqlite {
            path: PathBuf::from(trimmed),
        })
    } else {
        let host = window.get_settings_mariadb_host().trim().to_owned();
        let port = window.get_settings_mariadb_port().trim().to_owned();
        let user = window.get_settings_mariadb_user().trim().to_owned();
        let password = window.get_settings_mariadb_password().to_string();
        let db = window.get_settings_mariadb_database().trim().to_owned();
        if host.is_empty() || user.is_empty() || db.is_empty() {
            return Err("MariaDB host, user and database are required".to_owned());
        }
        let port = if port.is_empty() {
            "3306".to_owned()
        } else {
            port
        };
        // sqlx accepts `mysql://user:pass@host:port/db`. Password may
        // contain URL-reserved chars; for v1 we trust the user — a
        // proper percent-encoder is a follow-up if needed.
        let url = if password.is_empty() {
            format!("mysql://{user}@{host}:{port}/{db}")
        } else {
            format!("mysql://{user}:{password}@{host}:{port}/{db}")
        };
        Ok(BackendConfig::Mariadb { url })
    }
}

/// Localized one-liner summarising a [`MigrationReport`].
fn format_migration_report(report: &MigrationReport, i18n: &pomone_app::I18n) -> String {
    fn n(v: usize) -> i64 {
        i64::try_from(v).unwrap_or(i64::MAX)
    }
    let mut args = FluentArgs::new();
    args.set("families", n(report.families));
    args.set("strata", n(report.strata));
    args.set("kinds", n(report.location_kinds));
    args.set("locations", n(report.locations));
    args.set("crops", n(report.crops));
    args.set("varieties", n(report.varieties));
    args.set("plantings", n(report.plantings));
    args.set("harvests", n(report.yearly_harvests));
    i18n.t_args("settings-report", &args)
}

/// Wire the Save / Save+Migrate buttons. Validates the form, calls
/// `App::swap_backend`, refreshes every screen so the new data shows up,
/// and writes a localized status line.
fn try_swap_backend(window: &MainWindow, state: Rc<RefCell<UiState>>, migrate: bool) {
    let new_backend = match read_backend_from_form(window) {
        Ok(b) => b,
        Err(text) => {
            window.set_settings_status_text(SharedString::from(text));
            window.set_settings_status_is_error(true);
            return;
        }
    };
    let mut s = state.borrow_mut();
    // Split-borrow: swap_backend needs `&mut app` but the runtime needs to
    // outlive that mutable borrow. Destructuring through reborrow gives the
    // compiler two independent slots from the same `RefMut`.
    let result: Result<MigrationReport, AppError> = {
        let UiState {
            ref runtime,
            ref mut app,
            ..
        } = *s;
        runtime.block_on(async { app.swap_backend(new_backend, migrate).await })
    };
    match result {
        Ok(report) => {
            let i18n = s.app.i18n();
            let backend_text = backend_display(&s.app.config().backend);
            let mut args = FluentArgs::new();
            args.set("backend", backend_text.clone());
            let msg = if migrate {
                let report_text = format_migration_report(&report, i18n);
                args.set("report", report_text);
                i18n.t_args("settings-migrate-ok", &args)
            } else {
                i18n.t_args("settings-save-ok", &args)
            };
            window.set_settings_status_text(SharedString::from(msg));
            window.set_settings_status_is_error(false);

            // Every list-based screen now points at a different repo; reload them all.
            refresh_counts(window, &s.app, &s.runtime);
            let _ = refresh_plantings(window, &mut s);
            let _ = refresh_cultures(window, &mut s);
            let _ = refresh_locations(window, &mut s);
            let _ = refresh_calendar(window, &mut s);
            let _ = refresh_strata(window, &mut s);
            refresh_settings(window, &s);
        }
        Err(e) => {
            let mut args = FluentArgs::new();
            args.set("message", e.to_string());
            window.set_settings_status_text(SharedString::from(
                s.app.i18n().t_args("status-planting-failed", &args),
            ));
            window.set_settings_status_is_error(true);
        }
    }
}

/// Either a localized client-validation message or a service error that
/// still needs translation. Lets create handlers branch on prefix
/// ("Validation:" vs "Creation failed:") instead of mixing the two.
enum FormError {
    /// Already-localized text from a pre-submit validator.
    Validation(String),
    /// Service-level error; rendered via the existing `status-…-failed`
    /// template that prefixes "Échec :" / "Failed:".
    Service(AppError),
}

impl From<AppError> for FormError {
    fn from(e: AppError) -> Self {
        Self::Service(e)
    }
}

/// Trim and require a non-empty string. Returns the trimmed copy on success
/// or a localized "name required" message on failure.
fn validate_required_name(value: &str, i18n: &pomone_app::I18n) -> Result<String, FormError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        Err(FormError::Validation(i18n.t("error-name-required")))
    } else {
        Ok(trimmed.to_owned())
    }
}

/// Parse a `YYYY-MM-DD` date. Returns a localized "invalid date" message on
/// any parse failure (empty string included).
fn validate_iso_date(value: &str, i18n: &pomone_app::I18n) -> Result<NaiveDate, FormError> {
    NaiveDate::parse_from_str(value.trim(), "%Y-%m-%d")
        .map_err(|_| FormError::Validation(i18n.t("error-date-invalid")))
}

/// Parse a strictly-positive decimal. Empty or zero/negative input yields a
/// localized "positive required" message.
fn validate_positive_decimal(value: &str, i18n: &pomone_app::I18n) -> Result<Decimal, FormError> {
    let parsed = Decimal::from_str(value.trim())
        .map_err(|_| FormError::Validation(i18n.t("error-number-invalid")))?;
    if parsed <= Decimal::ZERO {
        return Err(FormError::Validation(i18n.t("error-positive-required")));
    }
    Ok(parsed)
}

/// Parse a strictly-positive `u32` count.
fn validate_positive_count(value: &str, i18n: &pomone_app::I18n) -> Result<u32, FormError> {
    let parsed = value
        .trim()
        .parse::<u32>()
        .map_err(|_| FormError::Validation(i18n.t("error-number-invalid")))?;
    if parsed == 0 {
        return Err(FormError::Validation(i18n.t("error-positive-required")));
    }
    Ok(parsed)
}

/// Parse a calendar year (required). Anything that doesn't fit `i32` or is
/// blank gets the localized "year required" message.
fn validate_year(value: &str, i18n: &pomone_app::I18n) -> Result<i32, FormError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(FormError::Validation(i18n.t("error-year-required")));
    }
    trimmed
        .parse::<i32>()
        .map_err(|_| FormError::Validation(i18n.t("error-year-required")))
}

/// Parse an optional decimal (empty → `None`). Errors are localized.
fn validate_optional_decimal(
    value: &str,
    i18n: &pomone_app::I18n,
) -> Result<Option<Decimal>, FormError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    Decimal::from_str(trimmed)
        .map(Some)
        .map_err(|_| FormError::Validation(i18n.t("error-number-invalid")))
}

/// Push a `FormError` onto a status banner with the appropriate Fluent
/// template (validation errors get the no-prefix template; service errors
/// keep the legacy "Échec :" prefix).
fn render_form_error(i18n: &pomone_app::I18n, err: FormError) -> (SharedString, bool) {
    let msg = match err {
        FormError::Validation(text) => {
            let mut args = FluentArgs::new();
            args.set("message", text);
            i18n.t_args("status-validation-failed", &args)
        }
        FormError::Service(app_err) => {
            let mut args = FluentArgs::new();
            args.set("message", app_err.to_string());
            i18n.t_args("status-planting-failed", &args)
        }
    };
    (SharedString::from(msg), true)
}

fn refresh_strata(window: &MainWindow, state: &mut UiState) -> Result<()> {
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

fn try_create_strata(window: &MainWindow, state: &mut UiState) -> Result<(), FormError> {
    let i18n = state.app.i18n();
    let name = validate_required_name(&window.get_new_strata_name(), i18n)?;
    let description = optional_text(&window.get_new_strata_description());
    let min_height = validate_optional_decimal(&window.get_new_strata_min_height(), i18n)?;
    let max_height = validate_optional_decimal(&window.get_new_strata_max_height(), i18n)?;
    let sort_order =
        parse_i32(&window.get_new_strata_sort_order(), "sort order").map_err(FormError::Service)?;

    // Surface a friendly range message client-side; the domain would also
    // reject this but its error string is technical.
    if let (Some(min), Some(max)) = (min_height, max_height) {
        if min > max {
            return Err(FormError::Validation(i18n.t("error-height-range")));
        }
    }

    state
        .runtime
        .block_on(async {
            create_strata(
                state.app.repo(),
                StrataInput {
                    name,
                    description,
                    min_height_m: min_height,
                    max_height_m: max_height,
                    sort_order,
                },
            )
            .await
            .map(|_| ())
        })
        .map_err(FormError::Service)
}

/// Load one planting's detail, push it to the UI and switch to the detail
/// page. `previous_page` is stored on the state so the Back button knows
/// where to return.
fn open_planting_detail(
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
    let expected = validate_optional_decimal(&window.get_new_harvest_expected(), i18n)?;
    let actual = validate_optional_decimal(&window.get_new_harvest_actual(), i18n)?;
    let notes = optional_text(&window.get_new_harvest_notes());

    state
        .runtime
        .block_on(async {
            services::record_yearly_harvest(
                state.app.repo(),
                planting_id,
                year,
                expected,
                actual,
                notes,
            )
            .await
            .map(|_| ())
        })
        .map_err(FormError::Service)
}

fn parse_i32(s: &str, field: &'static str) -> Result<i32, AppError> {
    s.trim()
        .parse::<i32>()
        .map_err(|e| AppError::Inconsistent(format!("invalid {field} '{s}': {e}")))
}

fn refresh_planting_detail(
    window: &MainWindow,
    state: &mut UiState,
    planting_id: &str,
) -> Result<()> {
    let snapshot: Result<(AppPlantingDetail, Vec<AppYearlyHarvestRow>), AppError> =
        state.runtime.block_on(async {
            let detail = get_planting_detail(state.app.repo(), planting_id).await?;
            // The yearly-harvest table is empty for annuals; querying it
            // anyway keeps the code path uniform and the SQL is a no-op.
            let harvests = list_yearly_harvests_for_planting(state.app.repo(), planting_id).await?;
            Ok((detail, harvests))
        });
    let (detail, harvests) = snapshot.context("failed to load planting detail")?;

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
    window.set_detail_area_label(SharedString::from(detail.area_label));
    window.set_detail_plants_count(usize_to_i32(detail.plants_count as usize));
    window.set_detail_name_value(SharedString::from(detail.name.unwrap_or_default()));
    window.set_detail_notes_value(SharedString::from(detail.notes.unwrap_or_default()));
    window.set_detail_schedule_lines(ModelRc::new(VecModel::from(lines)));
    window.set_detail_has_detail(true);
    window.set_detail_is_perennial(detail.is_perennial);
    window.set_harvest_rows(ModelRc::new(VecModel::from(harvest_rows)));
    // Clear stale form/status from the previous detail open.
    window.set_harvest_status_text(SharedString::from(""));
    window.set_harvest_status_is_error(false);
    Ok(())
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

/// First day of `(year, month)` as a `NaiveDate`. Panics only if the inputs
/// are out of `chrono`'s range, which the UI cannot produce.
fn first_of_month(year: i32, month: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, 1).expect("valid year/month from calendar state")
}

/// Map a `Weekday` to its 0-based offset with Monday as the first day of the
/// week (Mon=0, Sun=6). The calendar grid is rendered Monday-first.
fn weekday_offset_mon(d: NaiveDate) -> u32 {
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
fn kind_to_int(k: CalendarEventKind) -> i32 {
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
fn kind_glyph_key(k: CalendarEventKind) -> &'static str {
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

/// Rebuild the 42-cell day model + month label for the currently selected
/// `(calendar_year, calendar_month)` and push it to the window.
fn refresh_calendar(window: &MainWindow, state: &mut UiState) -> Result<()> {
    let year = state.calendar_year;
    let month = state.calendar_month;

    // Window of 42 days starting on the Monday on/before the 1st of the
    // selected month. Events are queried over the same window so off-month
    // cells can still surface a pill (e.g. a sowing on Apr 28 when looking
    // at May).
    let first = first_of_month(year, month);
    let lead = weekday_offset_mon(first);
    let grid_start = first
        .checked_sub_days(Days::new(u64::from(lead)))
        .context("calendar grid underflow")?;
    let grid_end = grid_start
        .checked_add_days(Days::new(41))
        .context("calendar grid overflow")?;

    let events: Vec<AppCalendarEvent> = state
        .runtime
        .block_on(async { list_events_in_range(state.app.repo(), grid_start, grid_end).await })
        .context("failed to load calendar events")?;

    // Bucket events by date for O(1) lookup per cell.
    let mut by_date: std::collections::HashMap<NaiveDate, Vec<&AppCalendarEvent>> =
        std::collections::HashMap::new();
    for e in &events {
        by_date.entry(e.date).or_default().push(e);
    }

    let i18n = state.app.i18n();
    let today = Local::now().date_naive();

    let mut days: Vec<SlintCalendarDay> = Vec::with_capacity(42);
    for offset in 0..42 {
        let date = grid_start
            .checked_add_days(Days::new(offset))
            .context("calendar cell overflow")?;
        let in_current_month = date.year() == year && date.month() == month;
        // Day numbers from leading/trailing months are suppressed so the
        // cell renders blank — the dimmer background already signals the
        // off-month state.
        let day_number = if in_current_month {
            i32::try_from(date.day()).unwrap_or(0)
        } else {
            0
        };
        let cell_events: Vec<SlintCalendarEvent> = by_date
            .get(&date)
            .map(|v| {
                v.iter()
                    .map(|e| SlintCalendarEvent {
                        kind: kind_to_int(e.kind),
                        glyph: SharedString::from(i18n.t(kind_glyph_key(e.kind))),
                        label: SharedString::from(e.label.clone()),
                        planting_id: SharedString::from(e.planting_id.clone()),
                    })
                    .collect()
            })
            .unwrap_or_default();
        days.push(SlintCalendarDay {
            day_number,
            in_current_month,
            is_today: date == today,
            events: ModelRc::new(VecModel::from(cell_events)),
        });
    }
    window.set_calendar_days(ModelRc::new(VecModel::from(days)));
    window.set_calendar_any_events(!events.is_empty());

    let month_key = format!("month-{month}");
    let month_name = i18n.t(&month_key);
    window.set_calendar_month_label(SharedString::from(format!("{month_name} {year}")));

    Ok(())
}

fn usize_to_i32(n: usize) -> i32 {
    i32::try_from(n).unwrap_or(i32::MAX)
}

/// Saturating cast of a signed `i32` index from Slint into a `usize`. A
/// negative value (Slint's "no current item") clamps to 0.
fn i32_to_usize(n: i32) -> usize {
    usize::try_from(n).unwrap_or(0)
}

#[allow(dead_code)]
fn _config_for_dev_fallback() -> AppConfig {
    AppConfig {
        backend: BackendConfig::Sqlite {
            path: "pomone.sqlite".into(),
        },
        language: "fr".into(),
    }
}
