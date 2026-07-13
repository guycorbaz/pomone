//! Composed use cases that combine domain logic with the [`Repository`].
//!
//! These functions take `&dyn Repository` so they're trivial to call from
//! the UI, the CLI, or tests, with any backend.

use crate::error::{AppError, AppResult};
use chrono::NaiveDate;
use pomone_db::Repository;
use pomone_domain::{
    date_calc, LocationId, Planting, PlantingId, PlantingSchedule, PlantingStatus, StrataId,
    Treatment, VarietyId, VarietyProfile, YearlyHarvest,
};
use rust_decimal::Decimal;

/// How an annual cropping is established — Qrop's three classic paths. Chosen
/// per planting (not stored: it's encoded by which dates the `Cycle` carries),
/// it drives both the inferred dates and the auto-generated tasks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EstablishmentMethod {
    /// Sown straight in place: Sow + Harvest (no transplant).
    DirectSow,
    /// Raised under cover then transplanted: Sow + Transplant + Harvest.
    RaisedTransplant,
    /// Bought ready-made plants: Plantation + Harvest (no sow).
    BoughtPlants,
}

/// Request for [`create_annual_planting`]. Build it from the required fields via
/// [`AnnualPlantingRequest::from_sowing`] (the raise-then-transplant default),
/// then layer optionals with the `with_*` setters. Grouping the parameters here
/// means E1/E2 can add an optional field without breaking any call site.
#[derive(Debug, Clone)]
pub struct AnnualPlantingRequest {
    pub variety_id: VarietyId,
    pub location_id: LocationId,
    pub strata_id: StrataId,
    /// `date` is the sowing date for `DirectSow`/`RaisedTransplant`, or the
    /// planting date for `BoughtPlants`.
    pub method: EstablishmentMethod,
    pub date: NaiveDate,
    pub area_m2: Decimal,
    pub plants_count: u32,
    pub name: Option<String>,
    pub notes: Option<String>,
}

impl AnnualPlantingRequest {
    /// Raise-then-transplant from a sowing date — the common default (formerly
    /// `create_annual_planting_from_sowing`). Use [`Self::with_method`] to pick
    /// direct-sow or bought-plants instead.
    #[must_use]
    pub fn from_sowing(
        variety_id: VarietyId,
        location_id: LocationId,
        strata_id: StrataId,
        sown_on: NaiveDate,
        area_m2: Decimal,
        plants_count: u32,
    ) -> Self {
        Self {
            variety_id,
            location_id,
            strata_id,
            method: EstablishmentMethod::RaisedTransplant,
            date: sown_on,
            area_m2,
            plants_count,
            name: None,
            notes: None,
        }
    }

    #[must_use]
    pub fn with_method(mut self, method: EstablishmentMethod) -> Self {
        self.method = method;
        self
    }

    #[must_use]
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    #[must_use]
    pub fn with_notes(mut self, notes: impl Into<String>) -> Self {
        self.notes = Some(notes.into());
        self
    }
}

/// Create an annual `Cycle` planting with the chosen establishment method.
/// Harvest dates are inferred from the variety's `AnnualProfile`.
/// `Inconsistent` if the variety is pluriannual.
pub async fn create_annual_planting(
    repo: &dyn Repository,
    request: AnnualPlantingRequest,
) -> AppResult<Planting> {
    let AnnualPlantingRequest {
        variety_id,
        location_id,
        strata_id,
        method,
        date,
        area_m2,
        plants_count,
        name,
        notes,
    } = request;
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
    // Build the schedule: which of sown_on / transplanted_on are set encodes
    // the method, and the auto-generator reads that back (Sow / Transplant /
    // Plantation). Harvest dates come from the variety profile.
    let schedule = match method {
        EstablishmentMethod::DirectSow => {
            let c = date_calc::infer_cycle_from_transplant(date, annual_profile)?;
            PlantingSchedule::cycle(Some(date), None, c.first_harvest_on, c.last_harvest_on)?
        }
        EstablishmentMethod::RaisedTransplant => {
            let c = date_calc::infer_cycle_from_sowing(date, annual_profile)?;
            PlantingSchedule::cycle(
                Some(c.sown_on),
                c.transplanted_on,
                c.first_harvest_on,
                c.last_harvest_on,
            )?
        }
        EstablishmentMethod::BoughtPlants => {
            let c = date_calc::infer_cycle_from_transplant(date, annual_profile)?;
            PlantingSchedule::cycle(None, Some(date), c.first_harvest_on, c.last_harvest_on)?
        }
    };
    let planting = Planting::new(
        variety_id,
        location_id,
        strata_id,
        crop.lifespan,
        area_m2,
        plants_count,
        schedule,
        name,
        notes,
    )?;
    repo.planting_create(&planting).await?;
    // Best-effort auto-generation of the operational tasks. A failure here only
    // logs — the planting is already saved and the user can re-trigger later.
    if let Err(e) = crate::task_autogen::generate_tasks_for_planting(repo, &planting).await {
        tracing::warn!(error = %e, planting_id = %planting.id, "failed to auto-generate tasks");
    }
    Ok(planting)
}

/// Request for [`create_perennial_planting`]. Build the required fields with
/// [`PerennialPlantingRequest::new`], then add optionals via the `with_*`
/// setters.
#[derive(Debug, Clone)]
pub struct PerennialPlantingRequest {
    pub variety_id: VarietyId,
    pub location_id: LocationId,
    pub strata_id: StrataId,
    pub established_on: NaiveDate,
    pub expected_removal_on: Option<NaiveDate>,
    pub area_m2: Decimal,
    pub plants_count: u32,
    pub name: Option<String>,
    pub notes: Option<String>,
}

impl PerennialPlantingRequest {
    #[must_use]
    pub fn new(
        variety_id: VarietyId,
        location_id: LocationId,
        strata_id: StrataId,
        established_on: NaiveDate,
        area_m2: Decimal,
        plants_count: u32,
    ) -> Self {
        Self {
            variety_id,
            location_id,
            strata_id,
            established_on,
            expected_removal_on: None,
            area_m2,
            plants_count,
            name: None,
            notes: None,
        }
    }

    #[must_use]
    pub fn with_expected_removal(mut self, expected_removal_on: NaiveDate) -> Self {
        self.expected_removal_on = Some(expected_removal_on);
        self
    }

    #[must_use]
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    #[must_use]
    pub fn with_notes(mut self, notes: impl Into<String>) -> Self {
        self.notes = Some(notes.into());
        self
    }
}

/// Create a perennial planting (a long-lived productive plant tracked by
/// yearly harvests). Rejects annual varieties — the caller must use
/// [`create_annual_planting`] for those.
pub async fn create_perennial_planting(
    repo: &dyn Repository,
    request: PerennialPlantingRequest,
) -> AppResult<Planting> {
    let PerennialPlantingRequest {
        variety_id,
        location_id,
        strata_id,
        established_on,
        expected_removal_on,
        area_m2,
        plants_count,
        name,
        notes,
    } = request;
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
        strata_id,
        crop.lifespan,
        area_m2,
        plants_count,
        schedule,
        name,
        notes,
    )?;
    repo.planting_create(&planting).await?;
    if let Err(e) = crate::task_autogen::generate_tasks_for_planting(repo, &planting).await {
        tracing::warn!(error = %e, planting_id = %planting.id, "failed to auto-generate tasks");
    }
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

/// Whether a planting carries *real* activity — a completed task or logged
/// labor hours. Future, not-yet-done auto-generated tasks (sow / transplant /
/// harvest reminders) don't count: they hold no history worth keeping.
///
/// This is the gate for [`delete_planting`]: a planting with activity is kept
/// (and marked terminal instead), while a freshly-created mistake can still be
/// removed cleanly.
pub async fn planting_has_activity(
    repo: &dyn Repository,
    planting_id: PlantingId,
) -> AppResult<bool> {
    let tasks = repo.task_list_for_planting(planting_id).await?;
    Ok(tasks
        .iter()
        .any(|t| t.completed_on.is_some() || t.labor_hours.is_some_and(|h| h > Decimal::ZERO)))
}

/// Delete a planting, but only if it has no recorded activity.
///
/// Deleting a planting cascades to its tasks and task series (FK
/// `ON DELETE CASCADE`). That is harmless for a planting that was just created
/// by mistake — only future reminders disappear — but it would silently wipe
/// real history. So we refuse with [`AppError::PlantingHasActivity`] whenever
/// [`planting_has_activity`] is true; the caller should set a terminal status
/// via [`set_planting_status`] instead (issue #63).
pub async fn delete_planting(repo: &dyn Repository, planting_id: PlantingId) -> AppResult<()> {
    // Surface a clear NotFound rather than a silent no-op if the id is stale.
    if repo.planting_get(planting_id).await?.is_none() {
        return Err(AppError::NotFound {
            kind: "planting",
            id: planting_id.to_string(),
        });
    }
    if planting_has_activity(repo, planting_id).await? {
        return Err(AppError::PlantingHasActivity);
    }
    repo.planting_delete(planting_id).await?;
    Ok(())
}

/// Set a planting's life-cycle status (Active / Completed / Failed /
/// Abandoned). This is the non-destructive alternative to deletion for a
/// planting that has already happened (issue #63).
pub async fn set_planting_status(
    repo: &dyn Repository,
    planting_id: PlantingId,
    status: PlantingStatus,
) -> AppResult<()> {
    let mut planting = repo
        .planting_get(planting_id)
        .await?
        .ok_or_else(|| AppError::NotFound {
            kind: "planting",
            id: planting_id.to_string(),
        })?;
    planting.status = status;
    repo.planting_update(&planting).await?;
    Ok(())
}

/// Request for [`record_yearly_harvest`]. Build with
/// [`YearlyHarvestRequest::new`] (planting + year), then attach the optional
/// yields and notes via the `with_*` setters.
#[derive(Debug, Clone)]
pub struct YearlyHarvestRequest {
    pub planting_id: PlantingId,
    pub year: i32,
    pub expected_yield_kg: Option<Decimal>,
    pub actual_yield_kg: Option<Decimal>,
    pub notes: Option<String>,
}

impl YearlyHarvestRequest {
    #[must_use]
    pub fn new(planting_id: PlantingId, year: i32) -> Self {
        Self {
            planting_id,
            year,
            expected_yield_kg: None,
            actual_yield_kg: None,
            notes: None,
        }
    }

    #[must_use]
    pub fn with_expected_yield(mut self, expected_yield_kg: Decimal) -> Self {
        self.expected_yield_kg = Some(expected_yield_kg);
        self
    }

    #[must_use]
    pub fn with_actual_yield(mut self, actual_yield_kg: Decimal) -> Self {
        self.actual_yield_kg = Some(actual_yield_kg);
        self
    }

    #[must_use]
    pub fn with_notes(mut self, notes: impl Into<String>) -> Self {
        self.notes = Some(notes.into());
        self
    }
}

/// Upsert a yearly harvest record, but only when the planting is a perennial
/// (recurring) cultivation. Annual cycles record their harvest entirely
/// within `PlantingSchedule::Cycle` and don't use `YearlyHarvest`.
pub async fn record_yearly_harvest(
    repo: &dyn Repository,
    request: YearlyHarvestRequest,
) -> AppResult<YearlyHarvest> {
    let YearlyHarvestRequest {
        planting_id,
        year,
        expected_yield_kg,
        actual_yield_kg,
        notes,
    } = request;
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

/// Request for [`record_treatment`]. Build with [`TreatmentRequest::new`] (all
/// fields except notes are required), then optionally add notes.
#[derive(Debug, Clone)]
pub struct TreatmentRequest {
    pub planting_id: PlantingId,
    pub applied_on: NaiveDate,
    pub active_substance: String,
    pub product_name: String,
    pub dose: Decimal,
    pub dose_unit: String,
    pub notes: Option<String>,
}

impl TreatmentRequest {
    #[must_use]
    pub fn new(
        planting_id: PlantingId,
        applied_on: NaiveDate,
        active_substance: impl Into<String>,
        product_name: impl Into<String>,
        dose: Decimal,
        dose_unit: impl Into<String>,
    ) -> Self {
        Self {
            planting_id,
            applied_on,
            active_substance: active_substance.into(),
            product_name: product_name.into(),
            dose,
            dose_unit: dose_unit.into(),
            notes: None,
        }
    }

    #[must_use]
    pub fn with_notes(mut self, notes: impl Into<String>) -> Self {
        self.notes = Some(notes.into());
        self
    }
}

/// Record a phytosanitary treatment applied to a planting (issue #82).
/// Unlike yearly harvests, treatments make sense for every planting kind
/// (annual or perennial), so the only guard is that the planting exists.
pub async fn record_treatment(
    repo: &dyn Repository,
    request: TreatmentRequest,
) -> AppResult<Treatment> {
    let TreatmentRequest {
        planting_id,
        applied_on,
        active_substance,
        product_name,
        dose,
        dose_unit,
        notes,
    } = request;
    if repo.planting_get(planting_id).await?.is_none() {
        return Err(AppError::NotFound {
            kind: "planting",
            id: planting_id.to_string(),
        });
    }
    let treatment = Treatment::new(
        planting_id,
        applied_on,
        active_substance,
        product_name,
        dose,
        dose_unit,
        notes,
    )?;
    repo.treatment_create(&treatment).await?;
    Ok(treatment)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pomone_db::{
        seed_defaults, CropRepo, FamilyRepo, LocationKindRepo, LocationRepo, PlantingRepo,
        SqliteRepository, StrataRepo, TaskRepo, TaskTypeRepo, VarietyRepo, YearlyHarvestRepo,
    };
    use pomone_domain::{
        AnnualProfile, Crop, Family, Lifespan, Location, LocationKind, PluriannualProfile,
        PruningSeason, Strata, Task, TaskCategory, Variety,
    };
    use rust_decimal_macros::dec;

    /// Build a fully populated test repo with a single annual variety
    /// (Tomate Marmande) ready for planting.
    async fn setup_annual() -> (SqliteRepository, VarietyId, LocationId, StrataId) {
        let repo = SqliteRepository::in_memory().await.unwrap();
        let f = Family::new("Solanaceae", None, None).unwrap();
        let s = Strata::new("Herbacée", None, None, None, 40).unwrap();
        let k = LocationKind::new("Planche", None).unwrap();
        repo.family_create(&f).await.unwrap();
        repo.strata_create(&s).await.unwrap();
        repo.location_kind_create(&k).await.unwrap();
        let crop = Crop::new(f.id, "Tomate", None, Lifespan::Annual, PruningSeason::None).unwrap();
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
        (repo, variety.id, loc.id, s.id)
    }

    /// Build a perennial setup (Pommier Reine des Reinettes).
    async fn setup_perennial() -> (SqliteRepository, VarietyId, LocationId, StrataId) {
        let repo = SqliteRepository::in_memory().await.unwrap();
        let f = Family::new("Rosaceae", None, None).unwrap();
        let s = Strata::new("Sous-étage", None, None, None, 20).unwrap();
        let k = LocationKind::new("Verger", None).unwrap();
        repo.family_create(&f).await.unwrap();
        repo.strata_create(&s).await.unwrap();
        repo.location_kind_create(&k).await.unwrap();
        let lifespan = Lifespan::perennial(40, 3).unwrap();
        let crop = Crop::new(f.id, "Pommier", None, lifespan, PruningSeason::Winter).unwrap();
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
        (repo, variety.id, loc.id, s.id)
    }

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    #[tokio::test]
    async fn annual_planting_inferred_dates() {
        let (repo, vid, lid, sid) = setup_annual().await;
        let p = create_annual_planting(
            &repo,
            AnnualPlantingRequest::from_sowing(vid, lid, sid, d(2026, 3, 1), dec!(20), 100)
                .with_name("Tomates Marmande"),
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
        let (repo, vid, lid, sid) = setup_perennial().await;
        let err = create_annual_planting(
            &repo,
            AnnualPlantingRequest::from_sowing(vid, lid, sid, d(2026, 3, 1), dec!(20), 10),
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

        let err = create_annual_planting(
            &repo,
            AnnualPlantingRequest::from_sowing(
                VarietyId::new(),
                loc.id,
                StrataId::new(),
                d(2026, 3, 1),
                dec!(10),
                5,
            ),
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
        let (repo, vid, lid, sid) = setup_annual().await;
        let p = create_annual_planting(
            &repo,
            AnnualPlantingRequest::from_sowing(vid, lid, sid, d(2026, 3, 1), dec!(20), 100),
        )
        .await
        .unwrap();
        validate_planting_consistency(&repo, p.id).await.unwrap();
    }

    #[tokio::test]
    async fn validate_consistency_unknown_planting() {
        let (repo, _, _, _) = setup_annual().await;
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
        let (repo, vid, lid, sid) = setup_perennial().await;
        let p = create_perennial_planting(
            &repo,
            PerennialPlantingRequest::new(vid, lid, sid, d(2026, 3, 15), dec!(2000), 50)
                .with_expected_removal(d(2056, 12, 31))
                .with_name("Verger Sud"),
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
        let (repo, vid, lid, sid) = setup_annual().await;
        let err = create_perennial_planting(
            &repo,
            PerennialPlantingRequest::new(vid, lid, sid, d(2026, 3, 15), dec!(10), 5),
        )
        .await
        .unwrap_err();
        assert!(matches!(err, AppError::Inconsistent(_)));
    }

    #[tokio::test]
    async fn record_yearly_harvest_on_perennial_planting() {
        let (repo, vid, lid, sid) = setup_perennial().await;
        let lifespan = Lifespan::perennial(40, 3).unwrap();
        let p = Planting::new(
            vid,
            lid,
            sid,
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
            YearlyHarvestRequest::new(p.id, 2030)
                .with_expected_yield(dec!(50))
                .with_actual_yield(dec!(45))
                .with_notes("first real crop"),
        )
        .await
        .unwrap();
        assert_eq!(h.year, 2030);
        let stored = repo.yearly_harvest_get(p.id, 2030).await.unwrap().unwrap();
        assert_eq!(stored.actual_yield_kg, Some(dec!(45)));
    }

    #[tokio::test]
    async fn record_yearly_harvest_rejects_annual_planting() {
        let (repo, vid, lid, sid) = setup_annual().await;
        let p = create_annual_planting(
            &repo,
            AnnualPlantingRequest::from_sowing(vid, lid, sid, d(2026, 3, 1), dec!(20), 100),
        )
        .await
        .unwrap();
        let err = record_yearly_harvest(
            &repo,
            YearlyHarvestRequest::new(p.id, 2026).with_expected_yield(dec!(10)),
        )
        .await
        .unwrap_err();
        assert!(matches!(err, AppError::Inconsistent(_)));
    }

    // ----- Task auto-generation ------------------------------------------

    #[tokio::test]
    async fn creating_annual_planting_autogenerates_tasks() {
        let (repo, vid, lid, sid) = setup_annual().await;
        // Seed the default TaskTypes so the auto-generator finds matches.
        seed_defaults(&repo).await.unwrap();
        let p = create_annual_planting(
            &repo,
            AnnualPlantingRequest::from_sowing(vid, lid, sid, d(2026, 3, 1), dec!(20), 100),
        )
        .await
        .unwrap();
        let tasks = repo.task_list_for_planting(p.id).await.unwrap();
        // The Marmande profile has DTT set → Sow + Transplant + Harvest.
        assert_eq!(tasks.len(), 3, "expected sow + transplant + harvest");
        // Resolve task types so we can look up by category instead of name.
        let types = repo.task_type_list().await.unwrap();
        let cat_of = |t: &Task| {
            types
                .iter()
                .find(|tt| tt.id == t.task_type_id)
                .unwrap()
                .category
        };
        let cats: std::collections::HashSet<_> = tasks.iter().map(cat_of).collect();
        assert!(cats.contains(&TaskCategory::Sow));
        assert!(cats.contains(&TaskCategory::Transplant));
        assert!(cats.contains(&TaskCategory::Harvest));
    }

    #[tokio::test]
    async fn creating_perennial_planting_generates_a_plantation_task() {
        // Perennials are planted from bought stock → a single "Plantation"
        // task at establishment (not "Repiquage").
        let (repo, vid, lid, sid) = setup_perennial().await;
        seed_defaults(&repo).await.unwrap();
        let p = create_perennial_planting(
            &repo,
            PerennialPlantingRequest::new(vid, lid, sid, d(2026, 3, 15), dec!(2000), 50),
        )
        .await
        .unwrap();
        let tasks = repo.task_list_for_planting(p.id).await.unwrap();
        assert_eq!(tasks.len(), 1);
        let types = repo.task_type_list().await.unwrap();
        let tt = types
            .iter()
            .find(|t| t.id == tasks[0].task_type_id)
            .unwrap();
        assert_eq!(tt.name, "Plantation");
        assert_eq!(tasks[0].planned_on, d(2026, 3, 15));
    }

    #[tokio::test]
    async fn bought_plants_annual_generates_plantation_and_harvest() {
        let (repo, vid, lid, sid) = setup_annual().await;
        seed_defaults(&repo).await.unwrap();
        let p = create_annual_planting(
            &repo,
            AnnualPlantingRequest::from_sowing(vid, lid, sid, d(2026, 4, 1), dec!(20), 50)
                .with_method(EstablishmentMethod::BoughtPlants),
        )
        .await
        .unwrap();
        // No sowing recorded; a planting date instead.
        match p.schedule {
            PlantingSchedule::Cycle {
                sown_on,
                transplanted_on,
                ..
            } => {
                assert!(sown_on.is_none());
                assert_eq!(transplanted_on, Some(d(2026, 4, 1)));
            }
            PlantingSchedule::Perennial { .. } => panic!("expected Cycle"),
        }
        let tasks = repo.task_list_for_planting(p.id).await.unwrap();
        let types = repo.task_type_list().await.unwrap();
        let names: Vec<&str> = tasks
            .iter()
            .map(|t| {
                types
                    .iter()
                    .find(|tt| tt.id == t.task_type_id)
                    .unwrap()
                    .name
                    .as_str()
            })
            .collect();
        assert!(names.contains(&"Plantation"));
        assert!(names.contains(&"Récolte"));
        assert!(!names.contains(&"Repiquage"));
        assert!(!names.contains(&"Semis"));
    }

    #[tokio::test]
    async fn direct_sow_annual_generates_sow_and_harvest_only() {
        let (repo, vid, lid, sid) = setup_annual().await;
        seed_defaults(&repo).await.unwrap();
        let p = create_annual_planting(
            &repo,
            AnnualPlantingRequest::from_sowing(vid, lid, sid, d(2026, 3, 1), dec!(20), 100)
                .with_method(EstablishmentMethod::DirectSow),
        )
        .await
        .unwrap();
        match p.schedule {
            PlantingSchedule::Cycle {
                sown_on,
                transplanted_on,
                ..
            } => {
                assert_eq!(sown_on, Some(d(2026, 3, 1)));
                assert!(transplanted_on.is_none());
            }
            PlantingSchedule::Perennial { .. } => panic!("expected Cycle"),
        }
        let tasks = repo.task_list_for_planting(p.id).await.unwrap();
        assert_eq!(tasks.len(), 2, "Sow + Harvest, no transplant");
    }

    #[tokio::test]
    async fn planting_without_seeded_types_still_saves_logs_only() {
        // No seed_defaults call here → task_type list is empty.
        let (repo, vid, lid, sid) = setup_annual().await;
        let p = create_annual_planting(
            &repo,
            AnnualPlantingRequest::from_sowing(vid, lid, sid, d(2026, 3, 1), dec!(20), 100),
        )
        .await
        .unwrap();
        // Planting persisted (the auto-gen failure is logged, not bubbled).
        assert!(repo.planting_get(p.id).await.unwrap().is_some());
        // No tasks created.
        assert!(repo.task_list_for_planting(p.id).await.unwrap().is_empty());
    }

    // ----- Life-cycle status & protected delete (issue #63) --------------

    #[tokio::test]
    async fn delete_planting_succeeds_without_activity() {
        let (repo, vid, lid, sid) = setup_annual().await;
        seed_defaults(&repo).await.unwrap();
        let p = create_annual_planting(
            &repo,
            AnnualPlantingRequest::from_sowing(vid, lid, sid, d(2026, 3, 1), dec!(20), 100),
        )
        .await
        .unwrap();
        // Auto-generated tasks exist but none is completed → no real activity.
        assert!(!repo.task_list_for_planting(p.id).await.unwrap().is_empty());
        assert!(!planting_has_activity(&repo, p.id).await.unwrap());
        delete_planting(&repo, p.id).await.unwrap();
        assert!(repo.planting_get(p.id).await.unwrap().is_none());
        // The future reminders cascaded away with it.
        assert!(repo.task_list_for_planting(p.id).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn delete_planting_refused_when_a_task_is_completed() {
        let (repo, vid, lid, sid) = setup_annual().await;
        seed_defaults(&repo).await.unwrap();
        let p = create_annual_planting(
            &repo,
            AnnualPlantingRequest::from_sowing(vid, lid, sid, d(2026, 3, 1), dec!(20), 100),
        )
        .await
        .unwrap();
        let task = repo.task_list_for_planting(p.id).await.unwrap().remove(0);
        // Completion goes through the single fact write path (story 1.2).
        crate::facts::record_fact(
            &repo,
            crate::facts::Fact::Done {
                task_id: task.id,
                on: d(2026, 3, 2),
            },
            d(2026, 3, 2).and_hms_opt(0, 0, 0).unwrap(),
        )
        .await
        .unwrap();
        assert!(planting_has_activity(&repo, p.id).await.unwrap());
        let err = delete_planting(&repo, p.id).await.unwrap_err();
        assert!(matches!(err, AppError::PlantingHasActivity));
        // Still there — history preserved.
        assert!(repo.planting_get(p.id).await.unwrap().is_some());
    }

    #[tokio::test]
    async fn delete_planting_refused_when_labor_hours_logged() {
        let (repo, vid, lid, sid) = setup_annual().await;
        seed_defaults(&repo).await.unwrap();
        let p = create_annual_planting(
            &repo,
            AnnualPlantingRequest::from_sowing(vid, lid, sid, d(2026, 3, 1), dec!(20), 100),
        )
        .await
        .unwrap();
        let mut task = repo.task_list_for_planting(p.id).await.unwrap().remove(0);
        task.labor_hours = Some(dec!(1.5));
        repo.task_update(&task).await.unwrap();
        assert!(planting_has_activity(&repo, p.id).await.unwrap());
        assert!(matches!(
            delete_planting(&repo, p.id).await.unwrap_err(),
            AppError::PlantingHasActivity
        ));
    }

    #[tokio::test]
    async fn delete_planting_unknown_id_is_not_found() {
        let (repo, _, _, _) = setup_annual().await;
        let err = delete_planting(&repo, PlantingId::new()).await.unwrap_err();
        assert!(matches!(
            err,
            AppError::NotFound {
                kind: "planting",
                ..
            }
        ));
    }

    #[tokio::test]
    async fn set_planting_status_persists() {
        let (repo, vid, lid, sid) = setup_annual().await;
        seed_defaults(&repo).await.unwrap();
        let p = create_annual_planting(
            &repo,
            AnnualPlantingRequest::from_sowing(vid, lid, sid, d(2026, 3, 1), dec!(20), 100),
        )
        .await
        .unwrap();
        // Fresh plantings start Active.
        assert_eq!(p.status, PlantingStatus::Active);
        set_planting_status(&repo, p.id, PlantingStatus::Failed)
            .await
            .unwrap();
        let got = repo.planting_get(p.id).await.unwrap().unwrap();
        assert_eq!(got.status, PlantingStatus::Failed);
    }
}
