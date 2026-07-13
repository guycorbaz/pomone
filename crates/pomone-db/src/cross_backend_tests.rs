//! Scenarios exercised against EVERY backend.
//!
//! Each scenario takes `&dyn Repository` and is replayed on both SQLite
//! (in-memory) and MariaDB (testcontainer). The MariaDB tests are
//! `#[ignore]`d by default; run with `cargo test -- --ignored`.

use crate::repository::Repository;
use crate::seed::seed_defaults;
use chrono::{NaiveDate, NaiveDateTime};
use pomone_domain::{
    AnnualProfile, Crop, FactKind, Family, FieldEvent, Lifespan, Location, LocationKind, Planting,
    PlantingSchedule, PluriannualProfile, PruningSeason, Strata, Treatment, Variety,
    VarietyProfile, YearlyHarvest,
};
use rust_decimal_macros::dec;
use uuid::Uuid;

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

fn dt(y: i32, m: u32, day: u32, h: u32, mi: u32) -> NaiveDateTime {
    d(y, m, day).and_hms_opt(h, mi, 0).unwrap()
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
}
