//! Scenarios exercised against EVERY backend.
//!
//! Each scenario takes `&dyn Repository` and is replayed on both SQLite
//! (in-memory) and MariaDB (testcontainer). The MariaDB tests are
//! `#[ignore]`d by default; run with `cargo test -- --ignored`.

use crate::error::DbError;
use crate::repository::{FactOutcome, Repository, TaskProjection};
use crate::seed::seed_defaults;
use chrono::{NaiveDate, NaiveDateTime};
use pomone_domain::{
    skip_payload, AnnualProfile, Crop, FactKind, Family, FieldEvent, FieldEventId, Lifespan,
    Location, LocationKind, Planting, PlantingSchedule, PluriannualProfile, PruningSeason,
    SkipReason, Strata, Task, TaskId, Treatment, Variety, VarietyProfile, YearlyHarvest,
};
use rust_decimal_macros::dec;
use uuid::Uuid;

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

fn dt(y: i32, m: u32, day: u32, h: u32, mi: u32) -> NaiveDateTime {
    d(y, m, day).and_hms_opt(h, mi, 0).unwrap()
}

/// Compact `FieldEvent` builder for the `task`-targeted fact scenarios.
fn task_event(
    kind: FactKind,
    task_id: TaskId,
    on: NaiveDate,
    at: NaiveDateTime,
    payload: &str,
    corrects: Option<FieldEventId>,
) -> FieldEvent {
    FieldEvent::new(kind, "task", task_id.as_uuid(), on, at, payload, corrects).unwrap()
}

// ============================================================
// Scenarios (backend-agnostic)
// ============================================================

async fn scenario_seed_defaults(repo: &dyn Repository) {
    seed_defaults(repo).await.unwrap();
    assert_eq!(repo.strata_list().await.unwrap().len(), 7);
    assert_eq!(repo.location_kind_list().await.unwrap().len(), 6);
    assert_eq!(repo.family_list().await.unwrap().len(), 13);

    // Idempotency
    seed_defaults(repo).await.unwrap();
    assert_eq!(repo.strata_list().await.unwrap().len(), 7);
}

async fn scenario_full_perennial_chain(repo: &dyn Repository) {
    // Build the entire chain: Family → Crop → Variety → Planting → YearlyHarvest
    let family = Family::new("Rosaceae", None, None).unwrap();
    let strata = Strata::new("Sous-étage", None, None, None, 20).unwrap();
    let kind = LocationKind::new("Verger", None).unwrap();
    repo.family_create(&family).await.unwrap();
    repo.strata_create(&strata).await.unwrap();
    repo.location_kind_create(&kind).await.unwrap();

    let lifespan = Lifespan::perennial(40, 3).unwrap();
    let crop = Crop::new(
        family.id,
        "Pommier",
        Some("Malus domestica".into()),
        lifespan,
        PruningSeason::Winter,
    )
    .unwrap();
    repo.crop_create(&crop).await.unwrap();

    let variety = Variety::new(
        crop.id,
        lifespan,
        "Reine des Reinettes",
        None,
        VarietyProfile::Pluriannual(
            PluriannualProfile::new(Some(80), Some(110), 220, 280, Some(dec!(15.5))).unwrap(),
        ),
    )
    .unwrap();
    repo.variety_create(&variety).await.unwrap();

    let location = Location::new(kind.id, "Verger nord", dec!(50), dec!(40), None, None).unwrap();
    repo.location_create(&location).await.unwrap();

    let planting = Planting::new(
        variety.id,
        location.id,
        strata.id,
        lifespan,
        dec!(2000),
        50,
        PlantingSchedule::perennial(d(2026, 3, 15), Some(d(2056, 12, 31))).unwrap(),
        Some("Verger nord".into()),
        None,
    )
    .unwrap();
    repo.planting_create(&planting).await.unwrap();

    // YearlyHarvest: record three years of yield
    for (year, expected, actual) in [
        (2029, dec!(50), dec!(45)),
        (2030, dec!(80), dec!(95)),
        (2031, dec!(120), dec!(110)),
    ] {
        let h = YearlyHarvest::new(planting.id, year, Some(expected), Some(actual), None).unwrap();
        repo.yearly_harvest_upsert(&h).await.unwrap();
    }

    let harvests = repo
        .yearly_harvest_list_for_planting(planting.id)
        .await
        .unwrap();
    assert_eq!(harvests.len(), 3);
    assert_eq!(harvests[0].year, 2029);
    assert_eq!(harvests[2].actual_yield_kg, Some(dec!(110)));
    // Variance check: year 2030 had actual > expected
    assert_eq!(harvests[1].variance_kg(), Some(dec!(15)));

    // Roundtrip: refetch the planting and verify the perennial schedule survived
    let got = repo.planting_get(planting.id).await.unwrap().unwrap();
    assert_eq!(got, planting);
}

async fn scenario_annual_cycle_with_full_dates(repo: &dyn Repository) {
    let family = Family::new("Solanaceae", None, None).unwrap();
    let strata = Strata::new("Herbacée", None, None, None, 40).unwrap();
    let kind = LocationKind::new("Planche", None).unwrap();
    repo.family_create(&family).await.unwrap();
    repo.strata_create(&strata).await.unwrap();
    repo.location_kind_create(&kind).await.unwrap();

    let crop = Crop::new(
        family.id,
        "Tomate",
        None,
        Lifespan::Annual,
        PruningSeason::None,
    )
    .unwrap();
    repo.crop_create(&crop).await.unwrap();

    let variety = Variety::new(
        crop.id,
        Lifespan::Annual,
        "Marmande",
        None,
        VarietyProfile::Annual(AnnualProfile::new(Some(35), 70, 60).unwrap()),
    )
    .unwrap();
    repo.variety_create(&variety).await.unwrap();

    let location = Location::new(kind.id, "Planche A", dec!(25), dec!(0.82), None, None).unwrap();
    repo.location_create(&location).await.unwrap();

    let planting = Planting::new(
        variety.id,
        location.id,
        strata.id,
        Lifespan::Annual,
        dec!(20.5),
        100,
        PlantingSchedule::cycle(
            Some(d(2026, 3, 1)),
            Some(d(2026, 5, 1)),
            d(2026, 7, 1),
            d(2026, 10, 1),
        )
        .unwrap(),
        Some("Tomates Marmande planche A".into()),
        Some("paillage".into()),
    )
    .unwrap();
    repo.planting_create(&planting).await.unwrap();

    let got = repo.planting_get(planting.id).await.unwrap().unwrap();
    assert_eq!(got, planting);

    // Treatments: record two applications, newest listed first, then verify
    // the roundtrip and that deleting the planting cascades.
    let t1 = Treatment::new(
        planting.id,
        d(2026, 6, 10),
        "cuivre",
        "Bouillie bordelaise",
        dec!(1.25),
        "kg/ha",
        Some("avant pluie".into()),
    )
    .unwrap();
    let t2 = Treatment::new(
        planting.id,
        d(2026, 7, 5),
        "soufre",
        "Thiovit",
        dec!(3),
        "g/m²",
        None,
    )
    .unwrap();
    repo.treatment_create(&t1).await.unwrap();
    repo.treatment_create(&t2).await.unwrap();

    let treatments = repo.treatment_list_for_planting(planting.id).await.unwrap();
    assert_eq!(treatments.len(), 2);
    assert_eq!(treatments[0], t2, "newest treatment must come first");
    assert_eq!(treatments[1], t1);

    repo.treatment_delete(t2.id).await.unwrap();
    assert!(repo.treatment_get(t2.id).await.unwrap().is_none());
    assert_eq!(
        repo.treatment_list_for_planting(planting.id)
            .await
            .unwrap()
            .len(),
        1
    );

    repo.planting_delete(planting.id).await.unwrap();
    assert!(
        repo.treatment_get(t1.id).await.unwrap().is_none(),
        "planting delete must cascade to treatments"
    );
}

async fn scenario_fk_cascade_on_crop_delete(repo: &dyn Repository) {
    let family = Family::new("Test", None, None).unwrap();
    let strata = Strata::new("Test", None, None, None, 0).unwrap();
    repo.family_create(&family).await.unwrap();
    repo.strata_create(&strata).await.unwrap();
    let crop = Crop::new(
        family.id,
        "Crop",
        None,
        Lifespan::Annual,
        PruningSeason::None,
    )
    .unwrap();
    repo.crop_create(&crop).await.unwrap();
    let variety = Variety::new(
        crop.id,
        Lifespan::Annual,
        "V",
        None,
        VarietyProfile::Annual(AnnualProfile::new(None, 60, 30).unwrap()),
    )
    .unwrap();
    repo.variety_create(&variety).await.unwrap();

    repo.crop_delete(crop.id).await.unwrap();
    // Cascade: variety must be gone
    assert!(repo.variety_get(variety.id).await.unwrap().is_none());
}

async fn scenario_fk_restrict_on_family_delete(repo: &dyn Repository) {
    let family = Family::new("Held", None, None).unwrap();
    let strata = Strata::new("Held", None, None, None, 0).unwrap();
    repo.family_create(&family).await.unwrap();
    repo.strata_create(&strata).await.unwrap();
    let crop = Crop::new(family.id, "C", None, Lifespan::Annual, PruningSeason::None).unwrap();
    repo.crop_create(&crop).await.unwrap();

    // Family is RESTRICT-referenced by Crop → deletion should fail
    let err = repo.family_delete(family.id).await;
    assert!(err.is_err(), "expected FK restrict to block family delete");
}

/// The append-only field-event journal (story 1.1): round-trip, idempotent
/// duplicate-id insert (conflict-no-op), ordering, and a correction pointing
/// back at the corrected event. `FactKind` literals round-trip identically on
/// both backends because both go through the shared codec.
async fn scenario_field_events(repo: &dyn Repository) {
    let target = Uuid::new_v4();

    let done = FieldEvent::new(
        FactKind::TaskDone,
        "task",
        target,
        d(2026, 3, 2),
        dt(2026, 3, 2, 9, 30),
        "{\"labor_h\":1.5}",
        None,
    )
    .unwrap();
    repo.field_event_create(&done).await.unwrap();

    // Round-trip: what comes back is byte-for-byte what went in.
    let got = repo.field_event_get(done.id).await.unwrap().unwrap();
    assert_eq!(got, done);

    // Idempotent: re-inserting the same id is a silent no-op, not an error,
    // and does not duplicate the row.
    repo.field_event_create(&done).await.unwrap();
    assert_eq!(
        repo.field_event_list_for_target("task", target)
            .await
            .unwrap()
            .len(),
        1
    );

    // A correction is a new event pointing at the one it amends; the original
    // is untouched and both are listed, oldest first.
    let correction = FieldEvent::new(
        FactKind::TaskSkipped,
        "task",
        target,
        d(2026, 3, 2),
        dt(2026, 3, 3, 8, 0),
        "{\"reason\":\"weather\"}",
        Some(done.id),
    )
    .unwrap();
    repo.field_event_create(&correction).await.unwrap();

    let events = repo
        .field_event_list_for_target("task", target)
        .await
        .unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].id, done.id, "oldest first by recorded_at");
    assert_eq!(events[1].corrects, Some(done.id));

    // A different target is isolated; the whole-journal view sees everything.
    assert!(repo
        .field_event_list_for_target("task", Uuid::new_v4())
        .await
        .unwrap()
        .is_empty());
    assert_eq!(repo.field_event_list_all().await.unwrap().len(), 2);
}

/// `SkipReason` must round-trip through the DB identically on both backends
/// (AC1). The skip columns' *projection* is story 1.2's, but persistence has to
/// work now — so build a task with the columns set directly and read it back.
async fn scenario_task_skip_roundtrip(repo: &dyn Repository) {
    seed_defaults(repo).await.unwrap();
    let task_type = repo.task_type_list().await.unwrap()[0].id;

    let mut task = Task::new(
        None,
        None,
        task_type,
        None,
        None,
        d(2026, 3, 2),
        None,
        None,
        None,
        None,
    );
    task.skipped_on = Some(d(2026, 3, 2));
    task.skip_reason = Some(SkipReason::Weather);
    task.skip_note = Some("trop humide".into());
    repo.task_create(&task).await.unwrap();

    let got = repo.task_get(task.id).await.unwrap().unwrap();
    assert_eq!(got, task, "skip columns must round-trip");
    assert_eq!(got.skip_reason, Some(SkipReason::Weather));
    assert_eq!(got.skipped_on, Some(d(2026, 3, 2)));
    assert_eq!(got.skip_note.as_deref(), Some("trop humide"));
}

/// `record_fact` (story 1.2): the event insert and the task projection commit
/// atomically, idempotently on a replayed id, identically on both backends.
async fn scenario_record_fact(repo: &dyn Repository) {
    seed_defaults(repo).await.unwrap();
    let task_type = repo.task_type_list().await.unwrap()[0].id;
    let task = Task::new(
        None,
        None,
        task_type,
        None,
        None,
        d(2026, 3, 1),
        None,
        None,
        None,
        None,
    );
    repo.task_create(&task).await.unwrap();

    // Done: appends one event AND projects completion, in one transaction.
    let done = task_event(
        FactKind::TaskDone,
        task.id,
        d(2026, 3, 2),
        dt(2026, 3, 2, 9, 0),
        "{}",
        None,
    );
    let done_proj = TaskProjection::Done {
        task_id: task.id,
        on: d(2026, 3, 2),
    };
    assert_eq!(
        repo.record_fact(&done, &done_proj).await.unwrap(),
        FactOutcome::Recorded
    );
    let got = repo.task_get(task.id).await.unwrap().unwrap();
    assert_eq!(got.completed_on, Some(d(2026, 3, 2)));
    assert_eq!(
        repo.field_event_list_for_target("task", task.id.as_uuid())
            .await
            .unwrap()
            .len(),
        1
    );

    // Idempotent replay: same id → AlreadyRecorded, no second event, no change.
    assert_eq!(
        repo.record_fact(&done, &done_proj).await.unwrap(),
        FactOutcome::AlreadyRecorded
    );
    assert_eq!(
        repo.field_event_list_for_target("task", task.id.as_uuid())
            .await
            .unwrap()
            .len(),
        1
    );

    // Skip projects the reason and clears completion.
    let skip = task_event(
        FactKind::TaskSkipped,
        task.id,
        d(2026, 3, 3),
        dt(2026, 3, 3, 8, 0),
        &skip_payload(SkipReason::Weather, Some("humide")),
        None,
    );
    let skip_proj = TaskProjection::Skipped {
        task_id: task.id,
        on: d(2026, 3, 3),
        reason: SkipReason::Weather,
        note: Some("humide".into()),
    };
    repo.record_fact(&skip, &skip_proj).await.unwrap();
    let got = repo.task_get(task.id).await.unwrap().unwrap();
    assert_eq!(got.skip_reason, Some(SkipReason::Weather));
    assert_eq!(got.skip_note.as_deref(), Some("humide"));
    assert!(got.completed_on.is_none(), "skip clears completion");

    // Reopen: a correction that clears state and points `corrects` at the event
    // it amends. Exercises TaskReopened + a non-null corrects write on BOTH
    // backends (was SQLite-only before).
    let reopen = task_event(
        FactKind::TaskReopened,
        task.id,
        d(2026, 3, 4),
        dt(2026, 3, 4, 7, 0),
        "{}",
        Some(skip.id),
    );
    repo.record_fact(&reopen, &TaskProjection::Reopen { task_id: task.id })
        .await
        .unwrap();
    let got = repo.task_get(task.id).await.unwrap().unwrap();
    assert!(
        got.completed_on.is_none() && got.skip_reason.is_none(),
        "reopen clears all settled state"
    );
    let stored = repo.field_event_get(reopen.id).await.unwrap().unwrap();
    assert_eq!(stored.kind, FactKind::TaskReopened);
    assert_eq!(
        stored.corrects,
        Some(skip.id),
        "correction links to its target"
    );
}

/// A fact whose projection matches no task is rejected (0-row projection) and
/// commits NO orphan event — the whole transaction rolls back, on both backends.
async fn scenario_record_fact_rejects_missing_task(repo: &dyn Repository) {
    let ghost = TaskId::new();
    let event = task_event(
        FactKind::TaskDone,
        ghost,
        d(2026, 3, 5),
        dt(2026, 3, 5, 9, 0),
        "{}",
        None,
    );
    let err = repo
        .record_fact(
            &event,
            &TaskProjection::Done {
                task_id: ghost,
                on: d(2026, 3, 5),
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(err, DbError::NotFound { kind: "task", .. }));
    assert!(
        repo.field_event_get(event.id).await.unwrap().is_none(),
        "a rejected fact leaves no event"
    );
}

/// Crop-plan line (story 2.1): full CRUD round-trip identically on both
/// backends, plus the `variety` FK `RESTRICT` guard. Exercises the decimal
/// (`bed_meters`), the two `u32` columns and the `bool` `draft` — the columns
/// whose SQLite (TEXT/INTEGER) vs MariaDB (DECIMAL/INT/BOOLEAN) encodings must
/// decode to the same domain value.
async fn scenario_crop_plan_line(repo: &dyn Repository) {
    use pomone_domain::CropPlanLine;

    let family = Family::new("Asteraceae", None, None).unwrap();
    repo.family_create(&family).await.unwrap();
    let crop = Crop::new(
        family.id,
        "Laitue",
        None,
        Lifespan::Annual,
        PruningSeason::None,
    )
    .unwrap();
    repo.crop_create(&crop).await.unwrap();
    let variety = Variety::new(
        crop.id,
        Lifespan::Annual,
        "Batavia",
        None,
        VarietyProfile::Annual(AnnualProfile::new(Some(20), 45, 30).unwrap()),
    )
    .unwrap();
    repo.variety_create(&variety).await.unwrap();

    // Create + get: every column round-trips (draft true, stagger 14, dec meters).
    let line =
        CropPlanLine::new(variety.id, 6, dec!(15.5), 14, true, Some("batavia".into())).unwrap();
    repo.crop_plan_line_create(&line).await.unwrap();
    assert_eq!(
        repo.crop_plan_line_get(line.id).await.unwrap().unwrap(),
        line
    );

    // Update (promote from draft, change quantities) keeps identity.
    let promoted = line
        .clone()
        .with_updates(variety.id, 8, dec!(20), 0, false, None)
        .unwrap();
    repo.crop_plan_line_update(&promoted).await.unwrap();
    let got = repo.crop_plan_line_get(line.id).await.unwrap().unwrap();
    assert_eq!(got, promoted);
    assert!(!got.draft && got.stagger_days == 0 && got.notes.is_none());

    // List sees it.
    assert_eq!(repo.crop_plan_line_list().await.unwrap().len(), 1);

    // A variety planned by a line cannot be deleted (ON DELETE RESTRICT).
    assert!(
        repo.variety_delete(variety.id).await.is_err(),
        "variety FK RESTRICT must block deleting a planned variety"
    );

    // Delete the line, then the variety frees up; a second delete is NotFound.
    repo.crop_plan_line_delete(line.id).await.unwrap();
    assert!(repo.crop_plan_line_get(line.id).await.unwrap().is_none());
    let err = repo.crop_plan_line_delete(line.id).await.unwrap_err();
    assert!(matches!(
        err,
        DbError::NotFound {
            kind: "crop_plan_line",
            ..
        }
    ));
    repo.variety_delete(variety.id).await.unwrap();
}

/// ITK templates + activities (story 2.2): full round-trip on both backends,
/// the revived dormant `task_method`/`task_implement` FKs (optional, SET NULL on
/// delete), one-template-per-crop (UNIQUE), and the template→activity cascade.
async fn scenario_itk(repo: &dyn Repository) {
    use pomone_domain::{
        ItkActivity, ItkTemplate, TaskCategory, TaskImplement, TaskMethod, TaskType,
    };

    let family = Family::new("Asteraceae", None, None).unwrap();
    repo.family_create(&family).await.unwrap();
    let crop = Crop::new(
        family.id,
        "Laitue",
        None,
        Lifespan::Annual,
        PruningSeason::None,
    )
    .unwrap();
    repo.crop_create(&crop).await.unwrap();
    let tt = TaskType::new("Préparation", TaskCategory::Other, "#8a8a8a").unwrap();
    repo.task_type_create(&tt).await.unwrap();
    let method = TaskMethod::new("Manuel", None).unwrap();
    repo.task_method_create(&method).await.unwrap();
    let implement = TaskImplement::new("Grelinette", None).unwrap();
    repo.task_implement_create(&implement).await.unwrap();

    // Template + two activities (one pins the dormant method/implement FKs).
    let template = ItkTemplate::new(crop.id);
    repo.itk_template_create(&template).await.unwrap();
    assert_eq!(
        repo.itk_template_get_for_crop(crop.id).await.unwrap(),
        Some(template.clone())
    );

    let a0 = ItkActivity::new(
        template.id,
        tt.id,
        -10,
        Some(method.id),
        Some(implement.id),
        Some("préparation planche".into()),
        0,
        Some("béchage".into()),
    );
    let a1 = ItkActivity::new(
        template.id,
        tt.id,
        20,
        None,
        None,
        Some("désherbage".into()),
        1,
        None,
    );
    // Insert out of order — the list must come back position-sorted.
    repo.itk_activity_create(&a1).await.unwrap();
    repo.itk_activity_create(&a0).await.unwrap();
    let got = repo
        .itk_activity_list_for_template(template.id)
        .await
        .unwrap();
    assert_eq!(
        got,
        vec![a0.clone(), a1.clone()],
        "activities ordered by position"
    );
    assert_eq!(got[0].method_id, Some(method.id));

    // A second template for the same crop is refused (UNIQUE crop_id).
    assert!(repo
        .itk_template_create(&ItkTemplate::new(crop.id))
        .await
        .is_err());

    // Deleting the method SET NULLs the activity's method_id (dormant-FK revive).
    repo.task_method_delete(method.id).await.unwrap();
    let after = repo
        .itk_activity_list_for_template(template.id)
        .await
        .unwrap();
    assert_eq!(
        after[0].method_id, None,
        "method delete must SET NULL, not cascade"
    );
    assert_eq!(
        after[0].implement_id,
        Some(implement.id),
        "implement untouched"
    );

    // Deleting the template cascades to its activities.
    repo.itk_template_delete(template.id).await.unwrap();
    assert!(repo
        .itk_activity_list_for_template(template.id)
        .await
        .unwrap()
        .is_empty());
    let err = repo.itk_template_delete(template.id).await.unwrap_err();
    assert!(matches!(
        err,
        DbError::NotFound {
            kind: "itk_template",
            ..
        }
    ));
}

// ============================================================
// SQLite test entry points (always run)
// ============================================================

mod sqlite_backend {
    use super::*;
    use crate::SqliteRepository;

    async fn fresh() -> SqliteRepository {
        SqliteRepository::in_memory().await.unwrap()
    }

    #[tokio::test]
    async fn seed_defaults() {
        scenario_seed_defaults(&fresh().await).await;
    }

    #[tokio::test]
    async fn full_perennial_chain() {
        scenario_full_perennial_chain(&fresh().await).await;
    }

    #[tokio::test]
    async fn annual_cycle_with_full_dates() {
        scenario_annual_cycle_with_full_dates(&fresh().await).await;
    }

    #[tokio::test]
    async fn fk_cascade_on_crop_delete() {
        scenario_fk_cascade_on_crop_delete(&fresh().await).await;
    }

    #[tokio::test]
    async fn fk_restrict_on_family_delete() {
        scenario_fk_restrict_on_family_delete(&fresh().await).await;
    }

    #[tokio::test]
    async fn field_events() {
        scenario_field_events(&fresh().await).await;
    }

    #[tokio::test]
    async fn task_skip_roundtrip() {
        scenario_task_skip_roundtrip(&fresh().await).await;
    }

    #[tokio::test]
    async fn record_fact() {
        scenario_record_fact(&fresh().await).await;
    }

    #[tokio::test]
    async fn record_fact_rejects_missing_task() {
        scenario_record_fact_rejects_missing_task(&fresh().await).await;
    }

    #[tokio::test]
    async fn crop_plan_line() {
        scenario_crop_plan_line(&fresh().await).await;
    }

    #[tokio::test]
    async fn itk() {
        scenario_itk(&fresh().await).await;
    }
}

// ============================================================
// MariaDB test entry points (require Docker, ignored by default)
// ============================================================

mod mariadb_backend {
    use super::*;
    use crate::mariadb::test_helpers::fresh_repo;

    #[tokio::test]
    #[ignore = "requires Docker for MariaDB testcontainer"]
    async fn seed_defaults() {
        scenario_seed_defaults(&fresh_repo().await).await;
    }

    #[tokio::test]
    #[ignore = "requires Docker for MariaDB testcontainer"]
    async fn full_perennial_chain() {
        scenario_full_perennial_chain(&fresh_repo().await).await;
    }

    #[tokio::test]
    #[ignore = "requires Docker for MariaDB testcontainer"]
    async fn annual_cycle_with_full_dates() {
        scenario_annual_cycle_with_full_dates(&fresh_repo().await).await;
    }

    #[tokio::test]
    #[ignore = "requires Docker for MariaDB testcontainer"]
    async fn fk_cascade_on_crop_delete() {
        scenario_fk_cascade_on_crop_delete(&fresh_repo().await).await;
    }

    #[tokio::test]
    #[ignore = "requires Docker for MariaDB testcontainer"]
    async fn fk_restrict_on_family_delete() {
        scenario_fk_restrict_on_family_delete(&fresh_repo().await).await;
    }

    #[tokio::test]
    #[ignore = "requires Docker for MariaDB testcontainer"]
    async fn field_events() {
        scenario_field_events(&fresh_repo().await).await;
    }

    #[tokio::test]
    #[ignore = "requires Docker for MariaDB testcontainer"]
    async fn task_skip_roundtrip() {
        scenario_task_skip_roundtrip(&fresh_repo().await).await;
    }

    #[tokio::test]
    #[ignore = "requires Docker for MariaDB testcontainer"]
    async fn record_fact() {
        scenario_record_fact(&fresh_repo().await).await;
    }

    #[tokio::test]
    #[ignore = "requires Docker for MariaDB testcontainer"]
    async fn record_fact_rejects_missing_task() {
        scenario_record_fact_rejects_missing_task(&fresh_repo().await).await;
    }

    #[tokio::test]
    #[ignore = "requires Docker for MariaDB testcontainer"]
    async fn crop_plan_line() {
        scenario_crop_plan_line(&fresh_repo().await).await;
    }

    #[tokio::test]
    #[ignore = "requires Docker for MariaDB testcontainer"]
    async fn itk() {
        scenario_itk(&fresh_repo().await).await;
    }
}
