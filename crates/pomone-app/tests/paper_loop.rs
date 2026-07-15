//! Paper-loop harness — skeleton (story 0.7).
//!
//! This is the single harness every convergence epic extends. It drives the
//! app through **services and view-models only** (never the UI), against a
//! **file-backed, isolated** SQLite database, and exercises a kill/replay
//! cycle on both `FailureMode`s: seed → fail → restart → assert the database
//! reopens cleanly and the gestures survived.
//!
//! Today it is deliberately near-empty: one real view-model gesture and the
//! reopen assertion, plus the scaffolding each later epic plugs into —
//! - `FailureMode` (Kill | NetworkDrop),
//! - an **injected fixed clock** (`fixed_today`) so golden output is stable,
//! - **normalization helpers** (`normalize::sorted`, `normalize::snapshot`),
//! - explicit `// TODO(E_n)` no-op step hooks.
//!
//! It is CI-blocking from day one: `cargo test --workspace` runs it, so a
//! database that fails to reopen after a simulated crash breaks the build.
//!
//! Isolation: the harness passes an explicit temp path in `AppConfig`, so it
//! never reads or writes the real XDG data/config directories.

use std::path::{Path, PathBuf};

use chrono::NaiveDate;
use pomone_app::families_view::{create_family, list_families_admin};
use pomone_app::{App, AppConfig, BackendConfig};

/// How the app is torn down between seed and restart. On the SQLite backend
/// both modes currently converge on an abrupt drop + clean reopen; the enum
/// exists so later epics can inject real faults (a `SIGKILL` mid-write for
/// `Kill`, a dropped socket for `NetworkDrop` on the MariaDB backend).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FailureMode {
    Kill,
    NetworkDrop,
}

/// The injected clock. The harness never reads the wall clock, so every seed
/// and every golden snapshot is identical on every machine and every run.
fn fixed_today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 3, 2).expect("valid fixed harness date")
}

/// Deterministic marker for the baseline gesture — clock-stable via the
/// injected `fixed_today`.
fn marker_name() -> String {
    format!("Paper-loop {}", fixed_today())
}

/// Normalization policy for golden comparisons: fixed clock (see
/// `fixed_today`), stable ordering, locale-stable formatting. Later epics
/// compare normalized snapshots of documents and view-models through these.
mod normalize {
    /// Stable ordering — sort so a snapshot is independent of row insertion
    /// or query order.
    pub(crate) fn sorted(mut values: Vec<String>) -> Vec<String> {
        values.sort();
        values
    }

    /// Canonical multi-line snapshot: one sorted value per line.
    pub(crate) fn snapshot(values: &[String]) -> String {
        sorted(values.to_vec()).join("\n")
    }
}

// --- Per-epic step hooks --------------------------------------------------
// Each is a no-op today. The named epic replaces its body with real gestures
// (recorded facts, plan lines, placement, documents, reconciliation) and the
// harness assertions grow with it. Kept synchronous while empty; an epic that
// needs I/O makes its own hook `async`.

/// E1 oracle: a recorded fact projects state, and that projection surfaces in
/// the virtual PrintDoc (facts → projection → document). This is the contract
/// epic 4 will render to PDF, asserted here at the data level.
async fn step_e1_record_facts(app: &App) {
    use pomone_app::facts::{record_fact, Fact};
    use pomone_app::printdoc::{build_week_sheet, EntryState, PRINTDOC_VERSION};
    use pomone_domain::{SkipReason, Task};

    let monday = NaiveDate::from_ymd_opt(2026, 3, 2).unwrap();
    let task_type = app
        .repo()
        .task_type_list()
        .await
        .expect("seeded task types")[0]
        .id;
    let task = Task::new(
        None, None, task_type, None, None, monday, None, None, None, None,
    );
    app.repo().task_create(&task).await.expect("create task");

    // Skip it — the fact projects onto the task in one transaction.
    record_fact(
        app.repo(),
        Fact::Skipped {
            task_id: task.id,
            on: monday,
            reason: SkipReason::Weather,
            note: None,
        },
        monday.and_hms_opt(9, 0, 0).unwrap(),
    )
    .await
    .expect("record skip fact");

    // The virtual PrintDoc reflects that projection.
    let sheet = build_week_sheet(app.repo(), monday)
        .await
        .expect("build week sheet");
    assert_eq!(sheet.version, PRINTDOC_VERSION, "frozen contract version");
    let entry = sheet
        .days
        .iter()
        .flat_map(|day| &day.entries)
        .find(|e| e.task_id == task.id)
        .expect("the task appears on the week sheet");
    assert_eq!(
        entry.state,
        EntryState::Skipped,
        "facts → PrintDoc oracle: a skip must project to the sheet",
    );
}

/// E2 planning dataset: a crop-plan line generates its staggered planned
/// plantings (line-linked, unplaced). The harness now carries the plan so the
/// kill/replay cycle proves the planning layer survives an interruption too.
async fn step_e2_plan_lines(app: &App) {
    use pomone_app::generation::generate_from_plan_line;
    use pomone_app::plan_view::{save_plan_line, PlanLineInput};
    use pomone_domain::{
        AnnualProfile, Crop, Family, Lifespan, PruningSeason, Variety, VarietyProfile,
    };

    let family = Family::new("Asteraceae (paper-loop)", None, None).unwrap();
    app.repo().family_create(&family).await.unwrap();
    let crop = Crop::new(
        family.id,
        "Laitue",
        None,
        Lifespan::Annual,
        PruningSeason::None,
    )
    .unwrap();
    app.repo().crop_create(&crop).await.unwrap();
    let variety = Variety::new(
        crop.id,
        Lifespan::Annual,
        "Batavia",
        None,
        VarietyProfile::Annual(AnnualProfile::new(Some(20), 45, 30).unwrap()),
    )
    .unwrap();
    app.repo().variety_create(&variety).await.unwrap();

    let line_id = save_plan_line(
        app.repo(),
        &PlanLineInput {
            variety_id: variety.id.to_string(),
            series: "6".into(),
            bed_meters: "15".into(),
            stagger_days: "14".into(),
            first_on: "2026-04-01".into(),
            draft: false,
            ..Default::default()
        },
    )
    .await
    .expect("save plan line");

    let n = generate_from_plan_line(app.repo(), &line_id)
        .await
        .expect("generate from plan line");
    assert_eq!(n, 6, "the line generates 6 staggered planned plantings");
    assert_eq!(
        app.repo().planned_planting_list_all().await.unwrap().len(),
        6,
    );
}

fn step_e3_placement(_app: &App) {
    // TODO(E3): place plantings against the capacity engine.
}

fn step_e4_documents(_app: &App) {
    // TODO(E4): render the PrintDoc documents and snapshot them.
}

fn step_e5_reconcile(_app: &App) {
    // TODO(E5): walk the weekly corridor (done / skip / carry-over / correct).
}

// --- Harness plumbing -----------------------------------------------------

/// An isolated, empty database directory unique to this process and mode.
fn isolated_dir(mode: FailureMode) -> PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push(format!("pomone_paper_loop_{}_{mode:?}", std::process::id()));
    // Start from a clean slate even if a previous aborted run left files.
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create isolated harness dir");
    dir
}

fn harness_config(db_path: &Path) -> AppConfig {
    AppConfig {
        backend: BackendConfig::Sqlite {
            path: db_path.to_path_buf(),
        },
        language: "fr".to_owned(),
        holiday_region: String::new(),
        area_unit: "m2".to_owned(),
        mass_unit: "kg".to_owned(),
    }
}

/// Open (or reopen) the app on the isolated database — the same path each
/// time, which is what makes restart == reopen.
async fn open_app(config: &AppConfig) -> App {
    App::new(config.clone())
        .await
        .expect("open app on isolated database")
}

/// Run every no-op epic hook, then perform one durable view-model gesture so
/// the restart has something real to recover. Returns the created family id.
async fn seed_baseline(app: &App) -> String {
    step_e1_record_facts(app).await;
    step_e2_plan_lines(app).await;
    step_e3_placement(app);
    step_e4_documents(app);
    step_e5_reconcile(app);

    create_family(
        app.repo(),
        &marker_name(),
        "",
        "Paper-loop baseline",
        "#4caf50",
    )
    .await
    .expect("create baseline family via view-model")
}

/// Assert the database reopened cleanly after the failure and the baseline
/// gesture survived, exercising the normalization helpers on the way.
async fn assert_reopens_clean(app: &App, created_id: &str, mode: FailureMode) {
    let rows = list_families_admin(app.repo())
        .await
        .expect("list families after restart");

    assert!(
        rows.iter().any(|r| r.id == created_id),
        "{mode:?}: baseline family {created_id} missing after restart — DB did not reopen cleanly"
    );

    // Golden snapshot: sorted, clock-stable, order-independent.
    let names: Vec<String> = rows.iter().map(|r| r.name.clone()).collect();
    let snapshot = normalize::snapshot(&names);
    assert!(
        snapshot.contains(&marker_name()),
        "{mode:?}: normalized snapshot missing the baseline marker"
    );
    // Determinism: the snapshot does not depend on input ordering.
    let mut reversed = names.clone();
    reversed.reverse();
    assert_eq!(
        snapshot,
        normalize::snapshot(&reversed),
        "{mode:?}: normalization is not order-stable"
    );

    // The planning dataset (story 2.6) must survive the crash too: the 6 planned
    // plantings generated in step_e2 are still there after the reopen.
    let planned = app
        .repo()
        .planned_planting_list_all()
        .await
        .expect("list planned plantings after restart");
    assert_eq!(
        planned.len(),
        6,
        "{mode:?}: generated planned plantings did not survive the crash/reopen"
    );
}

/// One full paper loop for a single failure mode.
async fn run_paper_loop(mode: FailureMode) {
    let dir = isolated_dir(mode);
    let config = harness_config(&dir.join("pomone.sqlite"));

    // Phase 1 — seed, then fail by abandoning the app without a graceful close.
    let created_id = {
        let app = open_app(&config).await;
        let id = seed_baseline(&app).await;
        drop(app); // the "crash"
        id
    };

    // NetworkDrop additionally models a transient reconnect (a dropped socket
    // the client re-establishes) before the final restart; on SQLite that is
    // one extra clean reopen. Kill goes straight to the restart.
    if mode == FailureMode::NetworkDrop {
        drop(open_app(&config).await);
    }

    // Phase 2 — restart and verify.
    let app = open_app(&config).await;
    assert_reopens_clean(&app, &created_id, mode).await;
    drop(app);

    // Best-effort cleanup of the isolated directory.
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn paper_loop_survives_kill_and_reopen_on_both_failure_modes() {
    for mode in [FailureMode::Kill, FailureMode::NetworkDrop] {
        run_paper_loop(mode).await;
    }
}
