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
use chrono::{Datelike, Local};
use pomone_app::{
    extend_series_if_needed, parse_id, App, AppConfig, BackendConfig, WindowGeometry,
};
use slint::{ComponentHandle, SharedString};

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
use generated::MainWindow;

mod forms;
mod refresh;
mod state;
mod translations;
mod wiring;

pub(crate) use forms::{
    i32_to_usize, localize_app_error, optional_text, parse_hex_color, parse_i32,
    parse_optional_decimal, parse_optional_u16, parse_u16, parse_u8, render_family_form_error,
    render_form_error, today_iso, usize_to_i32, validate_iso_date, validate_optional_decimal,
    validate_positive_count, validate_positive_decimal, validate_required_name, validate_year,
    FormError,
};
pub(crate) use refresh::{
    all_category_keys, color_chooser_palette, first_of_month, open_planting_detail,
    refresh_after_task_form, refresh_agenda, refresh_bed_usage, refresh_cultures, refresh_families,
    refresh_locations, refresh_plan, refresh_planting_detail, refresh_plantings, refresh_strata,
    refresh_task_calendar, refresh_varieties_of_selected_crop, reset_crop_form_to_create,
    reset_families_form_to_create, reset_variety_form_to_create, weekday_offset_mon, AXIS_DAYS,
};
pub(crate) use state::{PendingDelete, UiState};
pub(crate) use translations::{apply_translations, apply_unit_labels};
pub(crate) use wiring::task_form::{open_task_form_for_edit, render_task_form_error};
pub(crate) use wiring::task_types::{
    refresh_task_types, render_task_type_form_error, reset_task_types_form_to_create,
};

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
        pending_delete: None,
        runtime,
        variety_ids: Vec::new(),
        variety_is_annuals_plantings: Vec::new(),
        plantings_sort_column: "variety".to_owned(),
        plantings_sort_asc: true,
        location_ids: Vec::new(),
        family_ids: Vec::new(),
        strata_ids: Vec::new(),
        crop_ids: Vec::new(),
        crop_is_annuals: Vec::new(),
        location_kind_ids: Vec::new(),
        parent_location_ids: Vec::new(),
        task_calendar_year: today_local.year(),
        task_calendar_month: today_local.month(),
        detail_previous_page: "plantings".to_owned(),
        detail_planting_id: String::new(),
        task_form_type_ids: Vec::new(),
        task_form_planting_ids: Vec::new(),
        editing_task_id: String::new(),
        task_form_previous_page: "tasks".to_owned(),
        plan_rows: Vec::new(),
        plan_variety_option_ids: Vec::new(),
        agenda_skip_target: String::new(),
        agenda_skip_reason_keys: Vec::new(),
        task_type_admin_ids: Vec::new(),
        task_type_category_keys: Vec::new(),
        editing_task_type_id: String::new(),
        editing_family_id: String::new(),
        editing_crop_id: String::new(),
        editing_variety_id: String::new(),
        editing_location_id: String::new(),
        task_filter_categories: all_category_keys().into_iter().collect(),
        show_milestones: true,
        task_form_recurrence_unit_keys: Vec::new(),
        crop_map_location_ids: Vec::new(),
    }));

    let window = MainWindow::new().context("failed to create MainWindow")?;
    let today = today_iso();
    window.set_sown_on_text(SharedString::from(today.clone()));
    window.set_established_on_text(SharedString::from(today));

    apply_translations(&window, &state.borrow().app);
    refresh_bed_usage(&window, &state.borrow().app, &state.borrow().runtime);
    refresh_plantings(&window, &mut state.borrow_mut())?;
    refresh_cultures(&window, &mut state.borrow_mut())?;
    refresh_locations(&window, &mut state.borrow_mut())?;
    refresh_strata(&window, &mut state.borrow_mut())?;
    wiring::settings::refresh_settings(&window, &state.borrow());
    // Materialize any pending occurrences of open-ended series up to the
    // 1-year horizon. Idempotent and cheap on a small DB.
    {
        let s = state.borrow();
        let today = Local::now().date_naive();
        if let Err(e) = s
            .runtime
            .block_on(async { extend_series_if_needed(s.app.repo(), today).await })
        {
            tracing::warn!(error = %e, "failed to extend recurring task series at startup");
        }
    }

    // --- Per-screen wiring (one call per screen — see wiring/mod.rs) ---
    wiring::settings::wire_settings(&window, &state);
    wiring::cultures::wire_cultures(&window, &state);
    wiring::locations::wire_locations(&window, &state);
    wiring::strata::wire_strata(&window, &state);
    wiring::families::wire_families(&window, &state);
    wiring::plantings::wire_plantings(&window, &state);
    wiring::crop_map::wire_crop_map(&window, &state);
    wiring::planting_detail::wire_planting_detail(&window, &state);
    wiring::home::wire_home(&window, &state);
    wiring::confirm::wire_confirm(&window, &state);
    wiring::task_calendar::wire_task_calendar(&window, &state);
    wiring::agenda::wire_agenda(&window, &state);
    wiring::plan::wire_plan(&window, &state);
    wiring::task_form::wire_task_form(&window, &state);
    wiring::task_types::wire_task_types(&window, &state);

    // Restore the last window size (falls back to the .slint default on first
    // launch); persist it again when the event loop exits.
    restore_window_geometry(&window);
    window.run().context("Slint event loop failed")?;
    save_window_geometry(&window);
    Ok(())
}

/// Resize the window to the last saved geometry, if any. Clamps to the minimum
/// so a corrupt/tiny value can't make the window unusable.
#[allow(clippy::cast_precision_loss)]
fn restore_window_geometry(window: &MainWindow) {
    if let Some(geom) = WindowGeometry::load() {
        let w = geom.width.max(880) as f32;
        let h = geom.height.max(600) as f32;
        window.window().set_size(slint::LogicalSize::new(w, h));
    }
}

/// Persist the current window size on exit (logical px, DPI-independent).
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
fn save_window_geometry(window: &MainWindow) {
    let size = window.window().size();
    let scale = window.window().scale_factor();
    if scale > 0.0 && size.width > 0 && size.height > 0 {
        let geom = WindowGeometry {
            width: (size.width as f32 / scale).round() as u32,
            height: (size.height as f32 / scale).round() as u32,
        };
        if let Err(e) = geom.save() {
            tracing::warn!(error = %e, "failed to save window geometry");
        }
    }
}

#[allow(dead_code)]
fn _config_for_dev_fallback() -> AppConfig {
    AppConfig {
        backend: BackendConfig::Sqlite {
            path: "pomone.sqlite".into(),
        },
        language: "fr".into(),
        holiday_region: "ch-vd".to_owned(),
        area_unit: "m2".to_owned(),
        mass_unit: "kg".to_owned(),
    }
}
