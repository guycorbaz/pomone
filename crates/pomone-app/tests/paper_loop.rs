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

    // The needs list (story 2.7) aggregates that one non-draft line: 6 × 15 m =
    // 90 bed-meters for the single variety, buy-by = its first sow.
    let needs = pomone_app::list_needs(app.repo(), app.i18n())
        .await
        .expect("needs list");
    assert_eq!(needs.len(), 1, "one variety in the needs list");
    assert_eq!(needs[0].quantity_bed_meters, "90");
    assert_eq!(needs[0].buy_by, "2026-04-01");
    assert!(needs[0].variety_label.ends_with("Batavia"));
}

/// E3 placement + capacity dataset (story 3.4). Places the successions E2
/// planned onto a real bed and checks the capacity curve reacts; then carries
/// the two perennial guarantees the epic closes on:
///
/// * **retro-entry** — a 1996 orchard row enters `active` with zero past tasks
///   (FR14), so the kill/replay cycle also proves the guarantee is *persisted*,
///   not just computed;
/// * **termination frees occupancy** (FR15) — the row stops loading the curve
///   from its termination date, while its past occupancy stays visible.
async fn step_e3_placement(app: &App) {
    let (bed_id, strata_id) = seed_placement_geometry(app).await;
    place_the_planned_successions(app, bed_id, strata_id).await;
    retro_enter_and_terminate_the_orchard(app, bed_id, strata_id).await;
}

/// One open-field parcel with a single 30 m bed beneath it, plus a stratum —
/// the minimum geometry the capacity engine needs to have something to load.
async fn seed_placement_geometry(
    app: &App,
) -> (pomone_domain::LocationId, pomone_domain::StrataId) {
    use pomone_domain::{Location, LocationKind, Strata};
    use rust_decimal_macros::dec;

    let kind = LocationKind::new("Planche (paper-loop)", None).unwrap();
    app.repo().location_kind_create(&kind).await.unwrap();
    let parcel = Location::new(kind.id, "Parcelle Est", dec!(100), dec!(30), None, None).unwrap();
    app.repo().location_create(&parcel).await.unwrap();
    let bed = Location::new(
        kind.id,
        "Planche E1",
        dec!(30),
        dec!(1.2),
        Some(parcel.id),
        None,
    )
    .unwrap();
    app.repo().location_create(&bed).await.unwrap();
    let strata = Strata::new("Herbacée (paper-loop)", None, None, None, 40).unwrap();
    app.repo().strata_create(&strata).await.unwrap();
    (bed.id, strata.id)
}

/// Place the six successions E2 generated — placement turns each planned row
/// into a real `Planting` and generates its tasks — then check the curve reacts.
async fn place_the_planned_successions(
    app: &App,
    bed_id: pomone_domain::LocationId,
    strata_id: pomone_domain::StrataId,
) {
    use pomone_app::capacity_view::occupancy_curve;
    use pomone_app::services::{place_planned_planting, PlacementRequest};

    let today = fixed_today();
    let planned = app.repo().planned_planting_list_all().await.unwrap();
    assert_eq!(planned.len(), 6, "E2 left six successions to place");
    for pp in &planned {
        place_planned_planting(
            app.repo(),
            PlacementRequest::new(pp.id, bed_id, strata_id, 60),
            today,
        )
        .await
        .expect("place a planned succession");
    }
    let still_unplaced = app
        .repo()
        .planned_planting_list_all()
        .await
        .unwrap()
        .into_iter()
        .filter(|pp| pp.placed_planting_id.is_none())
        .count();
    assert_eq!(still_unplaced, 0, "every succession is placed");

    // The curve reacts: the season carries occupancy, all of it open-field.
    let curve = occupancy_curve(app.repo(), 2026).await.expect("curve");
    assert!(
        curve.peak_open > 0.0,
        "placing six successions must load the open-field curve"
    );
    assert!(
        curve.peak_covered.abs() < f32::EPSILON,
        "nothing was placed under cover"
    );
}

/// The two perennial guarantees Epic 3 closes on: retro-entry generates zero
/// past tasks (FR14), and terminating frees the ground from its date (FR15)
/// without erasing the occupancy the planting really had.
async fn retro_enter_and_terminate_the_orchard(
    app: &App,
    bed_id: pomone_domain::LocationId,
    strata_id: pomone_domain::StrataId,
) {
    use pomone_app::capacity_view::occupancy_curve;
    use pomone_app::plantings_view::retro_entry_notice;
    use pomone_app::services::{
        create_perennial_planting, set_planting_status, PerennialPlantingRequest,
    };
    use pomone_domain::{
        Crop, Family, Lifespan, PluriannualProfile, PruningSeason, Variety, VarietyProfile,
    };
    use rust_decimal_macros::dec;

    let today = fixed_today();
    let family = Family::new("Rosaceae (paper-loop)", None, None).unwrap();
    app.repo().family_create(&family).await.unwrap();
    let lifespan = Lifespan::perennial(30, 4).unwrap();
    let crop = Crop::new(family.id, "Pommier", None, lifespan, PruningSeason::Winter).unwrap();
    app.repo().crop_create(&crop).await.unwrap();
    let variety = Variety::new(
        crop.id,
        lifespan,
        "Reinette",
        None,
        VarietyProfile::Pluriannual(
            PluriannualProfile::new(Some(100), Some(120), 250, 280, None).unwrap(),
        ),
    )
    .unwrap();
    app.repo().variety_create(&variety).await.unwrap();

    let orchard = create_perennial_planting(
        app.repo(),
        PerennialPlantingRequest::new(
            variety.id,
            bed_id,
            strata_id,
            NaiveDate::from_ymd_opt(1996, 4, 15).unwrap(),
            dec!(36),
            12,
        ),
        today,
    )
    .await
    .expect("retro-enter the 1996 orchard row");

    let tasks = app.repo().task_list_for_planting(orchard.id).await.unwrap();
    assert!(
        tasks.is_empty(),
        "a 1996 establishment must generate zero past tasks, got {:?}",
        tasks.iter().map(|t| t.planned_on).collect::<Vec<_>>()
    );
    let notice = retro_entry_notice(app.repo(), app.i18n(), &orchard, today)
        .await
        .expect("notice")
        .expect("a past-established perennial gets the reassurance line");
    assert!(
        notice.contains("1996"),
        "the notice names the year: {notice}"
    );

    // --- Termination frees occupancy (FR15) ---------------------------------
    // Open-ended perennial: before termination it loads the curve to the
    // horizon; after, only up to its termination date.
    let before = occupancy_curve(app.repo(), 2027).await.expect("curve 2027");
    assert!(
        before.peak_open > 0.0,
        "the orchard row occupies its bed in 2027 while it lives"
    );

    set_planting_status(
        app.repo(),
        orchard.id,
        pomone_domain::PlantingStatus::Failed,
        Some(NaiveDate::from_ymd_opt(2026, 6, 1).unwrap()),
    )
    .await
    .expect("terminate the orchard row");

    let after = occupancy_curve(app.repo(), 2027).await.expect("curve 2027");
    assert!(
        after.peak_open.abs() < f32::EPSILON,
        "a terminated perennial must stop occupying its ground (FR15): \
         2027 still shows {} bed-metres",
        after.peak_open
    );
    // …but its past is not erased: 2026 still carries the load it really had.
    let past = occupancy_curve(app.repo(), 2026).await.expect("curve 2026");
    assert!(
        past.peak_open > 0.0,
        "terminating must shorten the interval, not erase the planting's history"
    );
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
    step_e3_placement(app).await;
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
    // …and the needs list (story 2.7), derived from the same lines, still
    // aggregates them after the reopen.
    let needs = pomone_app::list_needs(app.repo(), app.i18n())
        .await
        .expect("needs list after restart");
    assert_eq!(
        needs.len(),
        1,
        "{mode:?}: needs list did not survive the crash/reopen"
    );
    assert_eq!(needs[0].quantity_bed_meters, "90");

    // The E3 placement + perennial dataset (story 3.4) survives the crash too.
    let plantings = app.repo().planting_list().await.expect("list plantings");
    let placed = plantings
        .iter()
        .filter(|p| matches!(p.schedule, pomone_domain::PlantingSchedule::Cycle { .. }))
        .count();
    assert_eq!(
        placed, 6,
        "{mode:?}: the six placed successions did not survive the crash/reopen"
    );

    let orchard = plantings
        .iter()
        .find(|p| {
            matches!(
                p.schedule,
                pomone_domain::PlantingSchedule::Perennial { .. }
            )
        })
        .expect("the retro-entered orchard row survived");
    assert_eq!(
        orchard.terminated_on,
        Some(NaiveDate::from_ymd_opt(2026, 6, 1).unwrap()),
        "{mode:?}: the termination date must be durable — it is what frees the curve"
    );
    assert!(
        app.repo()
            .task_list_for_planting(orchard.id)
            .await
            .unwrap()
            .is_empty(),
        "{mode:?}: the retro-entry guarantee (zero past tasks) must be persisted, not recomputed"
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
