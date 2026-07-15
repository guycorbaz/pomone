//! Dogfooding database-compatibility smoke test (story 1.7).
//!
//! Migrates a **sanitized copy of the owner's real database** to the current
//! schema and smoke-tests that the migrated data still serves the app — the
//! guard that "decade-old data round-trips all migrations" (NFR8). It never
//! touches the real file: it copies it to a temp dir first and only ever opens
//! the copy.
//!
//! It is `#[ignore]`d (CI-ignored, local-only) and gated on the
//! `POMONE_DOGFOOD_DB` environment variable pointing at a sanitized copy. Run it
//! once per schema change:
//!
//! ```sh
//! POMONE_DOGFOOD_DB=/path/to/sanitized-copy.sqlite \
//!   cargo test -p pomone-app --test dogfood_db -- --ignored --nocapture
//! ```
//!
//! See `docs/dogfooding-db-runbook.md` for the full procedure.

use chrono::NaiveDate;
use pomone_app::agenda_view::list_agenda;
use pomone_app::printdoc::build_week_sheet;
use pomone_app::{App, AppConfig, BackendConfig};

/// Environment variable holding the path to a **sanitized copy** of the real DB.
const DOGFOOD_ENV: &str = "POMONE_DOGFOOD_DB";

#[tokio::test]
#[ignore = "local dogfooding smoke — set POMONE_DOGFOOD_DB to a sanitized DB copy; runs once per schema change"]
async fn dogfood_real_database_migrates_and_smokes() {
    let Ok(src) = std::env::var(DOGFOOD_ENV) else {
        // Opt-in only: no path configured → nothing to check. Print how to run
        // it so an accidental `--ignored` invocation is self-explaining.
        eprintln!(
            "{DOGFOOD_ENV} not set — skipping dogfooding smoke. \
             Set it to a sanitized copy of the real database (see \
             docs/dogfooding-db-runbook.md)."
        );
        return;
    };
    let src = std::path::PathBuf::from(src);
    assert!(
        src.is_file(),
        "{DOGFOOD_ENV} = {} is not a readable file",
        src.display()
    );

    // Work on a COPY only — migrations mutate the schema/rows, and the real
    // (even sanitized) source must be left untouched.
    let dir = tempfile::tempdir().expect("temp dir");
    let copy = dir.path().join("dogfood.sqlite");
    std::fs::copy(&src, &copy).expect("copy the sanitized database");
    // Copy the WAL/SHM sidecars too if present, so no committed page is missed.
    for ext in ["-wal", "-shm"] {
        let side = with_suffix(&src, ext);
        if side.is_file() {
            let _ = std::fs::copy(&side, with_suffix(&copy, ext));
        }
    }

    // Opening the app runs every embedded migration up to the current schema on
    // the copy — the core of the compatibility check. A migration that can't
    // apply to real historical data fails here.
    let config = AppConfig {
        backend: BackendConfig::Sqlite { path: copy },
        language: "fr".to_owned(),
        holiday_region: String::new(),
        area_unit: "m2".to_owned(),
        mass_unit: "kg".to_owned(),
    };
    let app = App::new(config)
        .await
        .expect("migrate + open the real DB copy");

    // Smoke: the migrated data reads back through the domain repositories and
    // the presentation layer without error — the schema still serves the app.
    let crops = app.repo().crop_list().await.expect("crops read back");
    let plantings = app
        .repo()
        .planting_list()
        .await
        .expect("plantings read back");
    let tasks = app.repo().task_list().await.expect("tasks read back");

    // Exercise the fact-projection columns specifically (the Epic-1 additions):
    // the agenda and week sheet both read completed_on/skipped_on/skip_reason.
    let today = NaiveDate::from_ymd_opt(2026, 3, 2).unwrap();
    let agenda = list_agenda(app.repo(), app.i18n(), today)
        .await
        .expect("agenda builds from migrated data");
    let monday = today; // 2026-03-02 is a Monday
    build_week_sheet(app.repo(), monday)
        .await
        .expect("week sheet builds from migrated data");

    eprintln!(
        "dogfood OK: migrated + smoked — {} crops, {} plantings, {} tasks, {} agenda rows",
        crops.len(),
        plantings.len(),
        tasks.len(),
        agenda.len()
    );
}

/// Append a suffix to a path's filename (`foo.sqlite`, `-wal` → `foo.sqlite-wal`).
fn with_suffix(path: &std::path::Path, suffix: &str) -> std::path::PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(suffix);
    path.with_file_name(name)
}
