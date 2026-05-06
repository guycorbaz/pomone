//! Pomone desktop UI binary entry point (Slint).
//!
//! Phase 6 step 1: opens a single window that proves the full pipeline works
//! — App build → DB connection → seeded lookup data → i18n labels →
//! interactive language switch and counts refresh.
//!
//! Future sessions will add the planting management screens.

use std::cell::RefCell;
use std::rc::Rc;

use anyhow::{Context, Result};
use pomone_app::{App, AppConfig, BackendConfig, Lang};
use slint::{ComponentHandle, SharedString};

// Slint-generated bindings live in their own module so we can silence the
// lint warnings their macro-expanded code would otherwise raise (missing
// Debug impls, unsafe blocks for the VTable plumbing, pedantic clippy hits…).
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

/// Mutable, single-threaded UI state shared between the Slint event loop and
/// the callbacks. Slint runs on the main thread; tokio drives async DB calls
/// via `Runtime::block_on` from inside callbacks (queries are local SQLite
/// and finish in microseconds, so blocking the UI thread is acceptable here).
struct UiState {
    app: App,
    runtime: tokio::runtime::Runtime,
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

    // Use the user's saved config if present, otherwise fall back to defaults.
    let config = AppConfig::load_or_default().context("failed to load config")?;
    tracing::info!(?config.backend, lang = %config.language, "loaded config");
    let app = runtime
        .block_on(App::new(config))
        .context("failed to initialise App (DB connection / migrations / seed)")?;

    let state = Rc::new(RefCell::new(UiState { app, runtime }));

    let window = MainWindow::new().context("failed to create MainWindow")?;
    apply_translations(&window, &state.borrow().app);
    refresh_counts(&window, &state.borrow().app, &state.borrow().runtime);

    // Refresh button — re-reads counts from the DB.
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

    // Language toggle — flips Fr ↔ En and re-applies all labels.
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

    window.run().context("Slint event loop failed")?;
    Ok(())
}

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
    window.set_refresh_button_text(SharedString::from(i18n.t("button-refresh")));
    window.set_language_button_text(SharedString::from(i18n.t("button-switch-language")));
    window.set_current_language_tag(SharedString::from(i18n.lang().tag()));
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

/// Saturating cast of a `usize` count into Slint's `i32` model. We don't
/// expect counts above `i32::MAX` in practice; saturate rather than panic.
fn usize_to_i32(n: usize) -> i32 {
    i32::try_from(n).unwrap_or(i32::MAX)
}

#[allow(dead_code)]
fn _config_for_dev_fallback() -> AppConfig {
    // Kept for documentation: a SQLite in-memory App is impractical from the
    // GUI binary because each pool reconnect would lose data. We default to
    // a file-backed SQLite at the OS-specific path; an in-memory backend
    // makes sense only for tests where data lives one process.
    AppConfig {
        backend: BackendConfig::Sqlite {
            path: "pomone.sqlite".into(),
        },
        language: "fr".into(),
    }
}
