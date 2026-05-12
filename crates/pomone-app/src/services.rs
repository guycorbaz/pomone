//! Composed use cases that combine domain logic with the [`Repository`].
//!
//! These functions take `&dyn Repository` so they're trivial to call from
//! the UI, the CLI, or tests, with any backend.

use crate::error::{AppError, AppResult};
use chrono::NaiveDate;
use pomone_db::Repository;
use pomone_domain::{
    date_calc, LocationId, Planting, PlantingId, PlantingSchedule, VarietyId, VarietyProfile,
    YearlyHarvest,
};
use rust_decimal::Decimal;

/// Create an annual `Cycle` planting whose dates are inferred from the
/// variety's `AnnualProfile` and a sowing date.
///
/// Returns an `Inconsistent` error if the variety has a `PluriannualProfile`
/// (callers must use a different code path for perennial establishments).
pub async fn create_annual_planting_from_sowing(
    repo: &dyn Repository,
    variety_id: VarietyId,
    location_id: LocationId,
    sown_on: NaiveDate,
    area_m2: Decimal,
    plants_count: u32,
    name: Option<String>,
    notes: Option<String>,
) -> AppResult<Planting> {
    let variety = repo
        .variety_get(variety_id)
        .await?
        .ok_or_else(|| AppError::NotFound {
            kind: "variety",
            id: variety_id.to_string(),
        })?;
    let crop = repo
        .crop_get(variety.crop_id)
        .await?
        .ok_or_else(|| AppError::NotFound {
            kind: "crop",
            id: variety.crop_id.to_string(),
        })?;
    let annual_profile = match variety.profile {
        VarietyProfile::Annual(p) => p,
        VarietyProfile::Pluriannual(_) => {
            return Err(AppError::Inconsistent(
                "cannot infer a cycle from a pluriannual variety profile".into(),
            ));
        }
    };
    let inferred = date_calc::infer_cycle_from_sowing(sown_on, annual_profile)?;
    let schedule = PlantingSchedule::cycle(
        Some(inferred.sown_on),
        inferred.transplanted_on,
        inferred.first_harvest_on,
        inferred.last_harvest_on,
    )?;
    let planting = Planting::new(
        variety_id,
        location_id,
        crop.lifespan,
        area_m2,
        plants_count,
        schedule,
        name,
        notes,
    )?;
    repo.planting_create(&planting).await?;
    Ok(planting)
}

/// Create a perennial planting (a long-lived productive plant tracked by
/// yearly harvests). Rejects annual varieties — the caller must use
/// [`create_annual_planting_from_sowing`] for those.
pub async fn create_perennial_planting(
    repo: &dyn Repository,
    variety_id: VarietyId,
    location_id: LocationId,
    established_on: NaiveDate,
    expected_removal_on: Option<NaiveDate>,
    area_m2: Decimal,
    plants_count: u32,
    name: Option<String>,
    notes: Option<String>,
) -> AppResult<Planting> {
    let variety = repo
        .variety_get(variety_id)
        .await?
        .ok_or_else(|| AppError::NotFound {
            kind: "variety",
            id: variety_id.to_string(),
        })?;
    let crop = repo
        .crop_get(variety.crop_id)
        .await?
        .ok_or_else(|| AppError::NotFound {
            kind: "crop",
            id: variety.crop_id.to_string(),
        })?;
    if !crop.lifespan.is_recurring() {
        return Err(AppError::Inconsistent(
            "perennial planting requires a recurring pluriannual crop (apple, raspberry…)".into(),
        ));
    }
    let schedule = PlantingSchedule::perennial(established_on, expected_removal_on)?;
    let planting = Planting::new(
        variety_id,
        location_id,
        crop.lifespan,
        area_m2,
        plants_count,
        schedule,
        name,
        notes,
    )?;
    repo.planting_create(&planting).await?;
    Ok(planting)
}

/// Verify cross-entity invariants on an existing planting:
/// - Its variety still exists.
/// - Its variety's crop still exists.
/// - Its `PlantingSchedule` is still compatible with the crop's `Lifespan`.
/// - The variety's profile kind matches the crop's lifespan.
pub async fn validate_planting_consistency(
    repo: &dyn Repository,
    planting_id: PlantingId,
) -> AppResult<()> {
    let planting = repo
        .planting_get(planting_id)
        .await?
        .ok_or_else(|| AppError::NotFound {
            kind: "planting",
            id: planting_id.to_string(),
        })?;
    let variety = repo
        .variety_get(planting.variety_id)
        .await?
        .ok_or_else(|| AppError::NotFound {
            kind: "variety",
            id: planting.variety_id.to_string(),
        })?;
    let crop = repo
        .crop_get(variety.crop_id)
        .await?
        .ok_or_else(|| AppError::NotFound {
            kind: "crop",
            id: variety.crop_id.to_string(),
        })?;
    planting.schedule.check_compatible(crop.lifespan)?;
    variety.profile.check_compatible(crop.lifespan)?;
    Ok(())
}

/// Upsert a yearly harvest record, but only when the planting is a perennial
/// (recurring) cultivation. Annual cycles record their harvest entirely
/// within `PlantingSchedule::Cycle` and don't use `YearlyHarvest`.
pub async fn record_yearly_harvest(
    repo: &dyn Repository,
    planting_id: PlantingId,
    year: i32,
    expected_yield_kg: Option<Decimal>,
    actual_yield_kg: Option<Decimal>,
    notes: Option<String>,
) -> AppResult<YearlyHarvest> {
    let planting = repo
        .planting_get(planting_id)
        .await?
        .ok_or_else(|| AppError::NotFound {
            kind: "planting",
            id: planting_id.to_string(),
        })?;
    if !matches!(planting.schedule, PlantingSchedule::Perennial { .. }) {
        return Err(AppError::Inconsistent(
            "yearly harvests are only meaningful for perennial plantings".into(),
        ));
    }
    let harvest = YearlyHarvest::new(planting_id, year, expected_yield_kg, actual_yield_kg, notes)?;
    repo.yearly_harvest_upsert(&harvest).await?;
    Ok(harvest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pomone_db::{
        CropRepo, FamilyRepo, LocationKindRepo, LocationRepo, PlantingRepo, SqliteRepository,
        StrataRepo, VarietyRepo, YearlyHarvestRepo,
    };
    use pomone_domain::{
        AnnualProfile, Crop, Family, Lifespan, Location, LocationKind, PluriannualProfile,
        PruningSeason, Strata, Variety,
    };
    use rust_decimal_macros::dec;

    /// Build a fully populated test repo with a single annual variety
    /// (Tomate Marmande) ready for planting.
    async fn setup_annual() -> (SqliteRepository, VarietyId, LocationId) {
        let repo = SqliteRepository::in_memory().await.unwrap();
        let f = Family::new("Solanaceae", None, None).unwrap();
        let s = Strata::new("Herbacée", None, None, None, 40).unwrap();
        let k = LocationKind::new("Planche", None).unwrap();
        repo.family_create(&f).await.unwrap();
        repo.strata_create(&s).await.unwrap();
        repo.location_kind_create(&k).await.unwrap();
        let crop = Crop::new(
            f.id,
            s.id,
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
        let loc = Location::new(k.id, "Planche A", dec!(25), dec!(0.8), None, None).unwrap();
        repo.location_create(&loc).await.unwrap();
        (repo, variety.id, loc.id)
    }

    /// Build a perennial setup (Pommier Reine des Reinettes).
    async fn setup_perennial() -> (SqliteRepository, VarietyId, LocationId) {
        let repo = SqliteRepository::in_memory().await.unwrap();
        let f = Family::new("Rosaceae", None, None).unwrap();
        let s = Strata::new("Sous-étage", None, None, None, 20).unwrap();
        let k = LocationKind::new("Verger", None).unwrap();
        repo.family_create(&f).await.unwrap();
        repo.strata_create(&s).await.unwrap();
        repo.location_kind_create(&k).await.unwrap();
        let lifespan = Lifespan::perennial(40, 3).unwrap();
        let crop = Crop::new(f.id, s.id, "Pommier", None, lifespan, PruningSeason::Winter).unwrap();
        repo.crop_create(&crop).await.unwrap();
        let variety = Variety::new(
            crop.id,
            lifespan,
            "Reine des Reinettes",
            None,
            VarietyProfile::Pluriannual(
                PluriannualProfile::new(None, None, 220, 280, Some(dec!(15.5))).unwrap(),
            ),
        )
        .unwrap();
        repo.variety_create(&variety).await.unwrap();
        let loc = Location::new(k.id, "Verger nord", dec!(50), dec!(40), None, None).unwrap();
        repo.location_create(&loc).await.unwrap();
        (repo, variety.id, loc.id)
    }

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    #[tokio::test]
    async fn annual_planting_inferred_dates() {
        let (repo, vid, lid) = setup_annual().await;
        let p = create_annual_planting_from_sowing(
            &repo,
            vid,
            lid,
            d(2026, 3, 1),
            dec!(20),
            100,
            Some("Tomates Marmande".into()),
            None,
        )
        .await
        .unwrap();
        match p.schedule {
            PlantingSchedule::Cycle {
                sown_on,
                transplanted_on,
                first_harvest_on,
                last_harvest_on,
            } => {
                assert_eq!(sown_on, Some(d(2026, 3, 1)));
                // DTT=35 → transplant 5 April
                assert_eq!(transplanted_on, Some(d(2026, 4, 5)));
                // DTM=70 from transplant → first harvest 14 June
                assert_eq!(first_harvest_on, d(2026, 6, 14));
                // window=60 → last harvest 12 August
                assert_eq!(last_harvest_on, d(2026, 8, 12));
            }
            PlantingSchedule::Perennial { .. } => panic!("expected Cycle"),
        }
        // Persisted in repo
        assert_eq!(repo.planting_get(p.id).await.unwrap().unwrap(), p);
    }

    #[tokio::test]
    async fn annual_planting_creation_rejects_pluriannual_variety() {
        let (repo, vid, lid) = setup_perennial().await;
        let err = create_annual_planting_from_sowing(
            &repo,
            vid,
            lid,
            d(2026, 3, 1),
            dec!(20),
            10,
            None,
            None,
        )
        .await
        .unwrap_err();
        assert!(matches!(err, AppError::Inconsistent(_)));
    }

    #[tokio::test]
    async fn annual_planting_unknown_variety() {
        let repo = SqliteRepository::in_memory().await.unwrap();
        let kind = LocationKind::new("Test", None).unwrap();
        repo.location_kind_create(&kind).await.unwrap();
        let loc = Location::new(kind.id, "L", dec!(5), dec!(2), None, None).unwrap();
        repo.location_create(&loc).await.unwrap();

        let err = create_annual_planting_from_sowing(
            &repo,
            VarietyId::new(),
            loc.id,
            d(2026, 3, 1),
            dec!(10),
            5,
            None,
            None,
        )
        .await
        .unwrap_err();
        assert!(matches!(
            err,
            AppError::NotFound {
                kind: "variety",
                ..
            }
        ));
    }

    #[tokio::test]
    async fn validate_consistency_passes_for_well_formed_planting() {
        let (repo, vid, lid) = setup_annual().await;
        let p = create_annual_planting_from_sowing(
            &repo,
            vid,
            lid,
            d(2026, 3, 1),
            dec!(20),
            100,
            None,
            None,
        )
        .await
        .unwrap();
        validate_planting_consistency(&repo, p.id).await.unwrap();
    }

    #[tokio::test]
    async fn validate_consistency_unknown_planting() {
        let (repo, _, _) = setup_annual().await;
        let err = validate_planting_consistency(&repo, PlantingId::new())
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            AppError::NotFound {
                kind: "planting",
                ..
            }
        ));
    }

    #[tokio::test]
    async fn create_perennial_planting_persists_with_recurring_variety() {
        let (repo, vid, lid) = setup_perennial().await;
        let p = create_perennial_planting(
            &repo,
            vid,
            lid,
            d(2026, 3, 15),
            Some(d(2056, 12, 31)),
            dec!(2000),
            50,
            Some("Verger Sud".into()),
            None,
        )
        .await
        .unwrap();
        match p.schedule {
            PlantingSchedule::Perennial {
                established_on,
                expected_removal_on,
            } => {
                assert_eq!(established_on, d(2026, 3, 15));
                assert_eq!(expected_removal_on, Some(d(2056, 12, 31)));
            }
            PlantingSchedule::Cycle { .. } => panic!("expected Perennial schedule"),
        }
        assert_eq!(repo.planting_get(p.id).await.unwrap().unwrap(), p);
    }

    #[tokio::test]
    async fn create_perennial_planting_rejects_annual_variety() {
        let (repo, vid, lid) = setup_annual().await;
        let err = create_perennial_planting(
            &repo,
            vid,
            lid,
            d(2026, 3, 15),
            None,
            dec!(10),
            5,
            None,
            None,
        )
        .await
        .unwrap_err();
        assert!(matches!(err, AppError::Inconsistent(_)));
    }

    #[tokio::test]
    async fn record_yearly_harvest_on_perennial_planting() {
        let (repo, vid, lid) = setup_perennial().await;
        let lifespan = Lifespan::perennial(40, 3).unwrap();
        let p = Planting::new(
            vid,
            lid,
            lifespan,
            dec!(2000),
            50,
            PlantingSchedule::perennial(d(2026, 3, 15), None).unwrap(),
            None,
            None,
        )
        .unwrap();
        repo.planting_create(&p).await.unwrap();

        let h = record_yearly_harvest(
            &repo,
            p.id,
            2030,
            Some(dec!(50)),
            Some(dec!(45)),
            Some("first real crop".into()),
        )
        .await
        .unwrap();
        assert_eq!(h.year, 2030);
        let stored = repo.yearly_harvest_get(p.id, 2030).await.unwrap().unwrap();
        assert_eq!(stored.actual_yield_kg, Some(dec!(45)));
    }

    #[tokio::test]
    async fn record_yearly_harvest_rejects_annual_planting() {
        let (repo, vid, lid) = setup_annual().await;
        let p = create_annual_planting_from_sowing(
            &repo,
            vid,
            lid,
            d(2026, 3, 1),
            dec!(20),
            100,
            None,
            None,
        )
        .await
        .unwrap();
        let err = record_yearly_harvest(&repo, p.id, 2026, Some(dec!(10)), None, None)
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::Inconsistent(_)));
    }
}
