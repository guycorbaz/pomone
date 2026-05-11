//! Pomone desktop UI binary entry point (Slint).
//!
//! Phase 6 step 2: two screens (Home with counts + language toggle, Plantings
//! with list + add form). The Rust side owns all data — translations come
//! from Fluent and lists come from the repository through the
//! `pomone_app::plantings_view` helpers — and feeds it to Slint via plain
//! in-properties.

use std::cell::RefCell;
use std::rc::Rc;

use anyhow::{Context, Result};
use chrono::Local;
use fluent::FluentArgs;
use pomone_app::{
    list_location_options, list_plantings, list_variety_options, parse_id, parse_iso_date,
    seed_demo, services, App, AppConfig, AppError, BackendConfig, Lang, LocationOption,
    PlantingRow as AppPlantingRow, VarietyOption,
};
use pomone_domain::{LocationId, VarietyId};
use rust_decimal::Decimal;
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
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
use generated::{MainWindow, PlantingRow as SlintPlantingRow};

/// Mutable, single-threaded UI state. Slint runs on the main thread and tokio
/// drives async DB calls via `Runtime::block_on` inside callbacks (SQLite
/// queries finish in microseconds — blocking the UI thread is fine here).
struct UiState {
    app: App,
    runtime: tokio::runtime::Runtime,
    /// Stringified `VarietyId`s, parallel to the `variety-labels` Slint model.
    variety_ids: Vec<String>,
    /// Stringified `LocationId`s, parallel to the `location-labels` Slint model.
    location_ids: Vec<String>,
}

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

    // First-launch demo seed: one crop + two varieties + one parcel-with-bed
    // so the Plantings screen has something to pick. No-op on subsequent runs.
    runtime
        .block_on(seed_demo(app.repo()))
        .context("failed to seed demo data")?;

    let state = Rc::new(RefCell::new(UiState {
        app,
        runtime,
        variety_ids: Vec::new(),
        location_ids: Vec::new(),
    }));

    let window = MainWindow::new().context("failed to create MainWindow")?;
    window.set_sown_on_text(SharedString::from(today_iso()));

    apply_translations(&window, &state.borrow().app);
    refresh_counts(&window, &state.borrow().app, &state.borrow().runtime);
    refresh_plantings(&window, &mut state.borrow_mut())?;

    // --- Home page callbacks ---
    {
        let state = Rc::clone(&state);
        let weak = window.as_weak();
        window.on_refresh_counts(move || {
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
                    let i18n = s.app.i18n();
                    let mut args = FluentArgs::new();
                    args.set("message", e.to_string());
                    window.set_status_text(SharedString::from(
                        i18n.t_args("status-planting-failed", &args),
                    ));
                    window.set_status_is_error(true);
                }
            }
        });
    }

    window.run().context("Slint event loop failed")?;
    Ok(())
}

/// Refresh every string the UI displays based on the active language.
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
    window.set_refresh_button_text(SharedString::from(i18n.t("button-refresh")));
    window.set_language_button_text(SharedString::from(i18n.t("button-switch-language")));
    window.set_plantings_button_text(SharedString::from(i18n.t("button-plantings")));
    window.set_current_language_tag(SharedString::from(i18n.lang().tag()));

    // Plantings page
    window.set_plantings_title_text(SharedString::from(i18n.t("title-plantings")));
    window.set_back_button_text(SharedString::from(i18n.t("button-back")));
    window.set_empty_state_text(SharedString::from(i18n.t("empty-plantings")));
    window.set_section_new_text(SharedString::from(i18n.t("section-new-planting")));
    window.set_label_variety(SharedString::from(i18n.t("label-variety")));
    window.set_label_location(SharedString::from(i18n.t("label-location")));
    window.set_label_sown_on(SharedString::from(i18n.t("label-sown-on")));
    window.set_label_area(SharedString::from(i18n.t("label-area")));
    window.set_label_count(SharedString::from(i18n.t("label-plants-count")));
    window.set_placeholder_date(SharedString::from(i18n.t("placeholder-date")));
    window.set_placeholder_area(SharedString::from(i18n.t("placeholder-area")));
    window.set_placeholder_count(SharedString::from(i18n.t("placeholder-count")));
    window.set_create_button_text(SharedString::from(i18n.t("button-create-planting")));
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
    state.location_ids = snapshot.locations.iter().map(|l| l.id.clone()).collect();

    let variety_labels: Vec<SharedString> = snapshot
        .varieties
        .into_iter()
        .map(|v| SharedString::from(v.label))
        .collect();
    window.set_variety_labels(ModelRc::new(VecModel::from(variety_labels)));

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

/// Read the form fields, validate them, build typed IDs, and call the service.
fn try_create_planting(window: &MainWindow, state: &mut UiState) -> Result<(), AppError> {
    let variety_idx = i32_to_usize(window.get_variety_index());
    let location_idx = i32_to_usize(window.get_location_index());
    let variety_id_str = state
        .variety_ids
        .get(variety_idx)
        .ok_or_else(|| AppError::Inconsistent("no variety selected".to_owned()))?;
    let location_id_str = state
        .location_ids
        .get(location_idx)
        .ok_or_else(|| AppError::Inconsistent("no location selected".to_owned()))?;

    let variety_id: VarietyId = parse_id(variety_id_str)?;
    let location_id: LocationId = parse_id(location_id_str)?;
    let sown_on = parse_iso_date(&window.get_sown_on_text())?;
    let area_m2 = parse_decimal(&window.get_area_text(), "area")?;
    let plants_count = parse_count(&window.get_count_text())?;

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
    })
}

fn parse_decimal(s: &str, field: &'static str) -> Result<Decimal, AppError> {
    Decimal::from_str(s.trim())
        .map_err(|e| AppError::Inconsistent(format!("invalid {field} '{s}': {e}")))
}

fn parse_count(s: &str) -> Result<u32, AppError> {
    s.trim()
        .parse::<u32>()
        .map_err(|e| AppError::Inconsistent(format!("invalid plant count '{s}': {e}")))
}

fn today_iso() -> String {
    Local::now().date_naive().format("%Y-%m-%d").to_string()
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
