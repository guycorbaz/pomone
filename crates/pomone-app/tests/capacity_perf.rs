//! Perf budget for the placement curve (story 3.2, AC 4).
//!
//! The placement screen must feel instant: recomputing the occupancy curve for
//! a farm-scale set of placements has to stay **under 100 ms for ≤ 500
//! placements**. This test seeds 500 placed plantings across a bed hierarchy
//! and times a full `occupancy_curve` recompute.
//!
//! `std::time::Instant` wall-clock timing is legitimate here: the project's
//! `now()` ban is about *agronomic* time flowing into business logic below the
//! UI (AR12), not test instrumentation.

use std::time::{Duration, Instant};

use chrono::NaiveDate;
use pomone_app::capacity_view::occupancy_curve;
use pomone_db::{
    CropRepo, FamilyRepo, LocationKindRepo, LocationRepo, PlantingRepo, SqliteRepository,
    StrataRepo, VarietyRepo,
};
use pomone_domain::{
    AnnualProfile, Crop, Family, Lifespan, Location, LocationKind, Planting, PlantingSchedule,
    Strata, Variety, VarietyProfile,
};
use rust_decimal_macros::dec;

const PLACEMENTS: usize = 500;
const SEASON: i32 = 2026;

fn d(m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(SEASON, m, day).unwrap()
}

#[tokio::test]
async fn occupancy_curve_recomputes_under_100ms_for_500_placements() {
    let repo = SqliteRepository::in_memory().await.unwrap();

    // Minimal catalogue.
    let family = Family::new("Solanaceae", None, None).unwrap();
    repo.family_create(&family).await.unwrap();
    let strata = Strata::new("Herbacée", None, None, None, 40).unwrap();
    repo.strata_create(&strata).await.unwrap();
    let kind = LocationKind::new("Planche", None).unwrap();
    repo.location_kind_create(&kind).await.unwrap();
    let crop = Crop::new(
        family.id,
        "Tomate",
        None,
        Lifespan::Annual,
        pomone_domain::PruningSeason::None,
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

    // One farm root with 500 beds, one placed planting each (inserted directly
    // to keep setup fast — task autogen is not part of the measured path).
    let farm = Location::new(kind.id, "Ferme", dec!(500), dec!(400), None, None).unwrap();
    repo.location_create(&farm).await.unwrap();
    for i in 0..PLACEMENTS {
        let bed = Location::new(
            kind.id,
            format!("Planche {i}"),
            dec!(25),
            dec!(0.8),
            Some(farm.id),
            None,
        )
        .unwrap();
        repo.location_create(&bed).await.unwrap();
        // Stagger harvest windows across the season so the curve is non-trivial.
        let month = 4 + u32::try_from(i % 6).unwrap();
        let planting = Planting::new(
            variety.id,
            bed.id,
            strata.id,
            Lifespan::Annual,
            dec!(16),
            80,
            PlantingSchedule::cycle(Some(d(3, 1)), None, d(month, 1), d(month, 28)).unwrap(),
            None,
            None,
        )
        .unwrap();
        repo.planting_create(&planting).await.unwrap();
    }

    // Warm-up (prime any lazy caches / connection), then measure.
    let _ = occupancy_curve(&repo, SEASON).await.unwrap();
    let start = Instant::now();
    let curve = occupancy_curve(&repo, SEASON).await.unwrap();
    let elapsed = start.elapsed();

    assert_eq!(curve.points.len(), 52);
    assert!(curve.peak_open > 0.0, "the seeded farm has occupancy");
    assert!(
        elapsed < Duration::from_millis(100),
        "curve recompute took {elapsed:?} for {PLACEMENTS} placements (budget 100 ms)"
    );
}
