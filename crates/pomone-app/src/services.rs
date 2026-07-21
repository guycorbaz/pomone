//! Composed use cases that combine domain logic with the [`Repository`].
//!
//! These functions take `&dyn Repository` so they're trivial to call from
//! the UI, the CLI, or tests, with any backend.

use crate::error::{AppError, AppResult};
use chrono::NaiveDate;
use pomone_db::Repository;
use pomone_domain::{
    date_calc, LocationId, PlannedPlanting, PlannedPlantingId, Planting, PlantingId,
    PlantingSchedule, PlantingStatus, StrataId, Treatment, VarietyId, VarietyProfile,
    YearlyHarvest,
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
///
/// `today` is the caller-injected reference date handed to task generation (no
/// clock below the UI/CLI layer, AR12 — story 0.5 reserved this third slot for
/// exactly that). A cycle never suppresses past-dated tasks, so `today` does not
/// change what a healthy annual generates; it is threaded so both creation paths
/// share one generator contract.
pub async fn create_annual_planting(
    repo: &dyn Repository,
    request: AnnualPlantingRequest,
    today: NaiveDate,
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
    if let Err(e) = crate::task_autogen::generate_tasks_for_planting(repo, &planting, today).await {
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
///
/// **Retro-entry (story 3.4, FR14):** `today` is the caller-injected reference
/// date. A perennial established in the past generates **no task dated before
/// `today`** — retro-entering a 1996 orchard yields zero past tasks instead of
/// thirty years of phantom prunings.
pub async fn create_perennial_planting(
    repo: &dyn Repository,
    request: PerennialPlantingRequest,
    today: NaiveDate,
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
    if let Err(e) = crate::task_autogen::generate_tasks_for_planting(repo, &planting, today).await {
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
    // "Real activity" = a task actually *done* (or logged labor). A skipped task
    // is a decision, not work performed, so it never counts as done here
    // (story 1.6) — `is_completed` is the done-only predicate, never `is_settled`.
    Ok(tasks
        .iter()
        .any(|t| t.is_completed() || t.labor_hours.is_some_and(|h| h > Decimal::ZERO)))
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
///
/// **A terminal status carries its date** (story 3.4, FR26): `terminated_on`
/// must be `Some` for Completed / Failed / Abandoned, because that date is what
/// frees the ground on the capacity curve (FR15) — a terminal status without one
/// would leave a dead planting occupying its bed to the horizon. Going back to
/// `Active` clears the date and is the reversal path (FR24).
///
/// The transition goes through the domain (`Planting::terminate` / `reopen`) so
/// the "cannot end before it started" invariant runs; the field is never
/// assigned here.
pub async fn set_planting_status(
    repo: &dyn Repository,
    planting_id: PlantingId,
    status: PlantingStatus,
    terminated_on: Option<NaiveDate>,
) -> AppResult<()> {
    let mut planting = repo
        .planting_get(planting_id)
        .await?
        .ok_or_else(|| AppError::NotFound {
            kind: "planting",
            id: planting_id.to_string(),
        })?;
    if status == PlantingStatus::Active {
        planting.reopen();
    } else {
        let on = terminated_on.ok_or(AppError::TerminationDateRequired)?;
        planting.terminate(status, on)?;
    }
    repo.planting_update(&planting).await?;
    Ok(())
}

// ============================================================
// Placement (story 3.2): planned succession → real Planting
// ============================================================

/// Request to **place** an unplaced planned succession onto a bed, turning it
/// into a real [`Planting`].
///
/// The plan line only carries bed-metres, so placement supplies the two pieces
/// a `Planting` also needs: the vegetation `strata_id` and `plants_count`. The
/// `area_m2` is *derived* — `bed_meters × bed width` — so the capacity engine
/// recovers the original running metres exactly (`area ÷ width = bed_meters`).
#[derive(Debug, Clone, Copy)]
pub struct PlacementRequest {
    pub planned_planting_id: PlannedPlantingId,
    /// The leaf bed to place onto.
    pub location_id: LocationId,
    /// The vegetation stratum, chosen at placement.
    pub strata_id: StrataId,
    /// Plants on this succession (`> 0`), entered at placement.
    pub plants_count: u32,
}

impl PlacementRequest {
    #[must_use]
    pub const fn new(
        planned_planting_id: PlannedPlantingId,
        location_id: LocationId,
        strata_id: StrataId,
        plants_count: u32,
    ) -> Self {
        Self {
            planned_planting_id,
            location_id,
            strata_id,
            plants_count,
        }
    }
}

/// Fetch a planned planting by id (the repo exposes list, not get).
async fn planned_planting_by_id(
    repo: &dyn Repository,
    id: PlannedPlantingId,
) -> AppResult<PlannedPlanting> {
    repo.planned_planting_list_all()
        .await?
        .into_iter()
        .find(|pp| pp.id == id)
        .ok_or_else(|| AppError::NotFound {
            kind: "planned_planting",
            id: id.to_string(),
        })
}

/// Place a planned succession: create the real [`Planting`] on the chosen bed
/// and record the link on the planned row (so it leaves the unplaced list).
///
/// Annual varieties become a `Cycle` planting via [`create_annual_planting`];
/// perennials become a `Perennial` planting via [`create_perennial_planting`].
/// Refuses a succession that is already placed.
pub async fn place_planned_planting(
    repo: &dyn Repository,
    request: PlacementRequest,
    today: NaiveDate,
) -> AppResult<Planting> {
    let mut pp = planned_planting_by_id(repo, request.planned_planting_id).await?;
    if pp.is_placed() {
        return Err(AppError::Inconsistent(
            "planned succession is already placed".into(),
        ));
    }
    let bed = repo
        .location_get(request.location_id)
        .await?
        .ok_or_else(|| AppError::NotFound {
            kind: "location",
            id: request.location_id.to_string(),
        })?;
    // Derived area = running bed-metres × bed width. Saturating: `bed_meters` is
    // a persisted (unbounded) TEXT decimal, so a pathological row must not panic.
    let area_m2 = pp.bed_meters.saturating_mul(bed.width_m);

    let variety = repo
        .variety_get(pp.variety_id)
        .await?
        .ok_or_else(|| AppError::NotFound {
            kind: "variety",
            id: pp.variety_id.to_string(),
        })?;

    let planting = match variety.profile {
        VarietyProfile::Annual(_) => {
            create_annual_planting(
                repo,
                AnnualPlantingRequest::from_sowing(
                    pp.variety_id,
                    request.location_id,
                    request.strata_id,
                    pp.planned_on,
                    area_m2,
                    request.plants_count,
                ),
                today,
            )
            .await?
        }
        VarietyProfile::Pluriannual(_) => {
            create_perennial_planting(
                repo,
                PerennialPlantingRequest::new(
                    pp.variety_id,
                    request.location_id,
                    request.strata_id,
                    pp.planned_on,
                    area_m2,
                    request.plants_count,
                ),
                today,
            )
            .await?
        }
    };

    pp.placed_planting_id = Some(planting.id);
    repo.planned_planting_update(&pp).await?;
    Ok(planting)
}

/// Undo a placement: delete the placed [`Planting`] and return the succession to
/// the unplaced list. Allowed only while the planting carries **no recorded
/// activity** (a done task or logged labour) — same guard as [`delete_planting`],
/// which surfaces [`AppError::PlantingHasActivity`] otherwise. The FK
/// `ON DELETE SET NULL` clears `placed_planting_id` when the planting goes.
pub async fn unplace_planned_planting(
    repo: &dyn Repository,
    planned_planting_id: PlannedPlantingId,
) -> AppResult<()> {
    let pp = planned_planting_by_id(repo, planned_planting_id).await?;
    let Some(planting_id) = pp.placed_planting_id else {
        return Err(AppError::Inconsistent(
            "planned succession is not placed".into(),
        ));
    };
    // Guards activity and cascades ON DELETE SET NULL onto the planned row.
    delete_planting(repo, planting_id).await?;
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
        seed_defaults, CropRepo, FamilyRepo, LocationKindRepo, LocationRepo, PlannedPlantingRepo,
        PlantingRepo, SqliteRepository, StrataRepo, TaskRepo, TaskTypeRepo, VarietyRepo,
        YearlyHarvestRepo,
    };
    use pomone_domain::{
        AnnualProfile, Crop, CropPlanLine, Family, Lifespan, Location, LocationKind,
        PluriannualProfile, PruningSeason, Strata, Task, TaskCategory, Variety,
    };
    use rust_decimal_macros::dec;

    /// Create an unplaced planned succession of `variety_id` and return its id.
    async fn make_planned(
        repo: &dyn Repository,
        variety_id: VarietyId,
        bed_meters: Decimal,
        planned_on: NaiveDate,
    ) -> PlannedPlantingId {
        let line = CropPlanLine::new(variety_id, 1, bed_meters, 14, Some(planned_on), false, None)
            .unwrap();
        repo.crop_plan_line_create(&line).await.unwrap();
        let pp = PlannedPlanting::new(line.id, variety_id, 0, planned_on, bed_meters).unwrap();
        repo.planned_planting_create(&pp).await.unwrap();
        pp.id
    }

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

    // ---- placement (story 3.2) ---------------------------------------------

    #[tokio::test]
    async fn place_annual_converts_and_links() {
        let (repo, vid, lid, sid) = setup_annual().await;
        let pp_id = make_planned(&repo, vid, dec!(15), d(2026, 4, 1)).await;

        let planting = place_planned_planting(
            &repo,
            PlacementRequest::new(pp_id, lid, sid, 80),
            crate::test_helpers::no_cutoff_today(),
        )
        .await
        .unwrap();

        // A real annual Cycle planting now exists on the bed.
        assert_eq!(planting.location_id, lid);
        assert!(matches!(planting.schedule, PlantingSchedule::Cycle { .. }));
        // Derived area = bed_meters (15) × bed width (0.8) = 12 m².
        assert_eq!(planting.area_m2, dec!(12.0));
        assert_eq!(planting.plants_count, 80);

        // The planned row is now linked and off the unplaced list.
        let pp = repo.planned_planting_list_all().await.unwrap()[0].clone();
        assert_eq!(pp.placed_planting_id, Some(planting.id));
        assert!(pp.is_placed());
    }

    #[tokio::test]
    async fn place_perennial_converts() {
        let (repo, vid, lid, sid) = setup_perennial().await;
        let pp_id = make_planned(&repo, vid, dec!(10), d(2026, 3, 15)).await;

        let planting = place_planned_planting(
            &repo,
            PlacementRequest::new(pp_id, lid, sid, 5),
            crate::test_helpers::no_cutoff_today(),
        )
        .await
        .unwrap();
        assert!(matches!(
            planting.schedule,
            PlantingSchedule::Perennial { .. }
        ));
    }

    #[tokio::test]
    async fn place_twice_is_refused() {
        let (repo, vid, lid, sid) = setup_annual().await;
        let pp_id = make_planned(&repo, vid, dec!(15), d(2026, 4, 1)).await;
        place_planned_planting(
            &repo,
            PlacementRequest::new(pp_id, lid, sid, 80),
            crate::test_helpers::no_cutoff_today(),
        )
        .await
        .unwrap();
        let err = place_planned_planting(
            &repo,
            PlacementRequest::new(pp_id, lid, sid, 80),
            crate::test_helpers::no_cutoff_today(),
        )
        .await
        .unwrap_err();
        assert!(matches!(err, AppError::Inconsistent(_)));
    }

    #[tokio::test]
    async fn unplace_restores_the_unplaced_row() {
        let (repo, vid, lid, sid) = setup_annual().await;
        let pp_id = make_planned(&repo, vid, dec!(15), d(2026, 4, 1)).await;
        let planting = place_planned_planting(
            &repo,
            PlacementRequest::new(pp_id, lid, sid, 80),
            crate::test_helpers::no_cutoff_today(),
        )
        .await
        .unwrap();

        unplace_planned_planting(&repo, pp_id).await.unwrap();

        // Planting gone; the planned row is back to unplaced (ON DELETE SET NULL).
        assert!(repo.planting_get(planting.id).await.unwrap().is_none());
        let pp = repo.planned_planting_list_all().await.unwrap()[0].clone();
        assert_eq!(pp.placed_planting_id, None);
        assert!(!pp.is_placed());
    }

    #[tokio::test]
    async fn unplace_not_placed_is_refused() {
        let (repo, vid, _lid, _sid) = setup_annual().await;
        let pp_id = make_planned(&repo, vid, dec!(15), d(2026, 4, 1)).await;
        let err = unplace_planned_planting(&repo, pp_id).await.unwrap_err();
        assert!(matches!(err, AppError::Inconsistent(_)));
    }

    #[tokio::test]
    async fn unplace_blocked_when_planting_has_activity() {
        let (repo, vid, lid, sid) = setup_annual().await;
        seed_defaults(&repo).await.unwrap(); // task types for autogen
        let pp_id = make_planned(&repo, vid, dec!(15), d(2026, 4, 1)).await;
        let planting = place_planned_planting(
            &repo,
            PlacementRequest::new(pp_id, lid, sid, 80),
            crate::test_helpers::no_cutoff_today(),
        )
        .await
        .unwrap();

        // Log labour on an auto-generated task → real activity.
        let tasks = repo.task_list_for_planting(planting.id).await.unwrap();
        if let Some(mut t) = tasks.into_iter().next() {
            t.labor_hours = Some(dec!(2));
            repo.task_update(&t).await.unwrap();
            let err = unplace_planned_planting(&repo, pp_id).await.unwrap_err();
            assert!(matches!(err, AppError::PlantingHasActivity));
        }
    }

    #[tokio::test]
    async fn annual_planting_inferred_dates() {
        let (repo, vid, lid, sid) = setup_annual().await;
        let p = create_annual_planting(
            &repo,
            AnnualPlantingRequest::from_sowing(vid, lid, sid, d(2026, 3, 1), dec!(20), 100)
                .with_name("Tomates Marmande"),
            crate::test_helpers::no_cutoff_today(),
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
            crate::test_helpers::no_cutoff_today(),
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
            crate::test_helpers::no_cutoff_today(),
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
            crate::test_helpers::no_cutoff_today(),
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
            crate::test_helpers::no_cutoff_today(),
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
            crate::test_helpers::no_cutoff_today(),
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
            crate::test_helpers::no_cutoff_today(),
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
            crate::test_helpers::no_cutoff_today(),
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
            crate::test_helpers::no_cutoff_today(),
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
            crate::test_helpers::no_cutoff_today(),
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
            crate::test_helpers::no_cutoff_today(),
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
            crate::test_helpers::no_cutoff_today(),
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
            crate::test_helpers::no_cutoff_today(),
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
            crate::test_helpers::no_cutoff_today(),
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
            crate::test_helpers::no_cutoff_today(),
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
            crate::test_helpers::no_cutoff_today(),
        )
        .await
        .unwrap();
        // Fresh plantings start Active, with no termination date.
        assert_eq!(p.status, PlantingStatus::Active);
        assert_eq!(p.terminated_on, None);
        set_planting_status(&repo, p.id, PlantingStatus::Failed, Some(d(2026, 6, 10)))
            .await
            .unwrap();
        let got = repo.planting_get(p.id).await.unwrap().unwrap();
        assert_eq!(got.status, PlantingStatus::Failed);
        assert_eq!(got.terminated_on, Some(d(2026, 6, 10)));
    }

    /// FR24: terminating is reversible, and reviving clears the date so the
    /// planting occupies its ground again.
    #[tokio::test]
    async fn reopening_a_terminated_planting_clears_its_termination_date() {
        let (repo, vid, lid, sid) = setup_annual().await;
        seed_defaults(&repo).await.unwrap();
        let p = create_annual_planting(
            &repo,
            AnnualPlantingRequest::from_sowing(vid, lid, sid, d(2026, 3, 1), dec!(20), 100),
            crate::test_helpers::no_cutoff_today(),
        )
        .await
        .unwrap();
        set_planting_status(&repo, p.id, PlantingStatus::Abandoned, Some(d(2026, 6, 10)))
            .await
            .unwrap();

        set_planting_status(&repo, p.id, PlantingStatus::Active, None)
            .await
            .unwrap();
        let got = repo.planting_get(p.id).await.unwrap().unwrap();
        assert_eq!(got.status, PlantingStatus::Active);
        assert_eq!(got.terminated_on, None, "revival frees the date too");
    }

    /// A terminal status without a date is refused — defaulting it would
    /// quietly falsify the capacity curve (FR15).
    #[tokio::test]
    async fn terminal_status_without_a_date_is_refused() {
        let (repo, vid, lid, sid) = setup_annual().await;
        seed_defaults(&repo).await.unwrap();
        let p = create_annual_planting(
            &repo,
            AnnualPlantingRequest::from_sowing(vid, lid, sid, d(2026, 3, 1), dec!(20), 100),
            crate::test_helpers::no_cutoff_today(),
        )
        .await
        .unwrap();
        let err = set_planting_status(&repo, p.id, PlantingStatus::Completed, None)
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::TerminationDateRequired));
        // …and nothing was written.
        let got = repo.planting_get(p.id).await.unwrap().unwrap();
        assert_eq!(got.status, PlantingStatus::Active);
    }

    /// A planting cannot end before it started — the domain invariant runs
    /// because the service goes through `Planting::terminate`.
    #[tokio::test]
    async fn termination_before_the_start_is_refused() {
        let (repo, vid, lid, sid) = setup_annual().await;
        seed_defaults(&repo).await.unwrap();
        let p = create_annual_planting(
            &repo,
            AnnualPlantingRequest::from_sowing(vid, lid, sid, d(2026, 3, 1), dec!(20), 100),
            crate::test_helpers::no_cutoff_today(),
        )
        .await
        .unwrap();
        let err = set_planting_status(&repo, p.id, PlantingStatus::Failed, Some(d(2020, 1, 1)))
            .await
            .unwrap_err();
        assert!(
            matches!(err, AppError::Domain(_)),
            "expected a domain invariant error, got {err:?}"
        );
    }
}
