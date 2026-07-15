//! Data migration between two [`Repository`] backends.
//!
//! `copy_all` reads every entity from a source repository and writes it
//! into a target repository in foreign-key-safe order. The target must be
//! empty of user data (the caller skips `seed_defaults` for that reason);
//! lookup tables (families, strata, location kinds) are copied verbatim
//! from the source rather than re-seeded so the user's edits survive the
//! migration.
//!
//! The function is intentionally synchronous-feeling — it walks the entity
//! graph once, sequential awaits, no parallelism. Pomone-sized catalogs
//! make the simple loop trivially fast; parallelism would buy nothing and
//! complicate FK ordering.
//!
//! Returned [`MigrationReport`] carries the count of records copied per
//! entity so the UI can display a one-line summary.

use crate::error::{AppError, AppResult};
use pomone_db::Repository;
use pomone_domain::{Location, LocationId};
use std::collections::{HashMap, HashSet};

/// Per-entity count of records copied during a [`copy_all`] run.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MigrationReport {
    pub families: usize,
    pub strata: usize,
    pub location_kinds: usize,
    pub locations: usize,
    pub crops: usize,
    pub varieties: usize,
    pub plantings: usize,
    pub yearly_harvests: usize,
    pub task_types: usize,
    pub task_methods: usize,
    pub task_implements: usize,
    pub task_series: usize,
    pub tasks: usize,
    pub treatments: usize,
    pub crop_plan_lines: usize,
    pub itk_templates: usize,
    pub itk_activities: usize,
    pub field_events: usize,
    /// Snapshot of the previous SQLite file taken by
    /// [`crate::App::swap_backend`] just before the swap; `None` when the
    /// previous backend was MariaDB or had no database file yet.
    pub pre_swap_backup: Option<std::path::PathBuf>,
}

/// Copy every record from `source` into `target`. Honours FK order:
/// families/strata/location_kinds/task lookups → locations (roots before
/// children) → crops → varieties → plantings → yearly_harvests/treatments
/// → task_series → tasks.
///
/// The target must be a fresh schema with **no** seeded defaults — IDs of
/// the source's lookup tables (Family/Strata/LocationKind/TaskType…) are
/// reused verbatim, and the underlying `_create` calls would otherwise fail
/// on a duplicate primary key.
pub async fn copy_all(
    source: &dyn Repository,
    target: &dyn Repository,
) -> AppResult<MigrationReport> {
    // Guard: the copy reuses the source's primary keys verbatim and runs
    // without an enclosing transaction, so a non-empty target would collide
    // mid-write and be left half-populated. Refuse before touching it — the
    // caller's source DB stays the untouched source of truth either way.
    if target_has_data(target).await? {
        return Err(AppError::MigrationTargetNotEmpty);
    }

    let mut report = MigrationReport::default();

    // 1. Independent lookup tables.
    for f in source.family_list().await? {
        target.family_create(&f).await?;
        report.families += 1;
    }
    for s in source.strata_list().await? {
        target.strata_create(&s).await?;
        report.strata += 1;
    }
    for k in source.location_kind_list().await? {
        target.location_kind_create(&k).await?;
        report.location_kinds += 1;
    }
    for tt in source.task_type_list().await? {
        target.task_type_create(&tt).await?;
        report.task_types += 1;
    }
    for m in source.task_method_list().await? {
        target.task_method_create(&m).await?;
        report.task_methods += 1;
    }
    for i in source.task_implement_list().await? {
        target.task_implement_create(&i).await?;
        report.task_implements += 1;
    }

    // 2. Locations: pre-order walk so each parent lands before its children
    //    (the FK on `parent_id` would otherwise reject the insert).
    let locations = source.location_list().await?;
    let by_id: HashMap<LocationId, Location> =
        locations.iter().map(|l| (l.id, l.clone())).collect();
    let mut ordered: Vec<Location> = Vec::with_capacity(locations.len());
    let mut emitted: HashSet<LocationId> = HashSet::new();
    for l in &locations {
        push_with_ancestors(l, &by_id, &mut ordered, &mut emitted);
    }
    for l in &ordered {
        target.location_create(l).await?;
        report.locations += 1;
    }

    // 3. Crops, varieties, plantings, yearly harvests.
    for c in source.crop_list().await? {
        target.crop_create(&c).await?;
        report.crops += 1;
    }
    for v in source.variety_list().await? {
        target.variety_create(&v).await?;
        report.varieties += 1;
    }
    let plantings = source.planting_list().await?;
    for p in &plantings {
        target.planting_create(p).await?;
        report.plantings += 1;
    }
    for p in &plantings {
        for h in source.yearly_harvest_list_for_planting(p.id).await? {
            target.yearly_harvest_upsert(&h).await?;
            report.yearly_harvests += 1;
        }
        for t in source.treatment_list_for_planting(p.id).await? {
            target.treatment_create(&t).await?;
            report.treatments += 1;
        }
    }

    // 3b. Crop-plan lines (story 2.1) — reference varieties, so after them.
    for line in source.crop_plan_line_list().await? {
        target.crop_plan_line_create(&line).await?;
        report.crop_plan_lines += 1;
    }

    // 3c. ITK templates + activities (story 2.2) — templates reference crops,
    //     activities reference their template + task lookups (all copied above).
    for crop in source.crop_list().await? {
        if let Some(template) = source.itk_template_get_for_crop(crop.id).await? {
            target.itk_template_create(&template).await?;
            report.itk_templates += 1;
            for activity in source.itk_activity_list_for_template(template.id).await? {
                target.itk_activity_create(&activity).await?;
                report.itk_activities += 1;
            }
        }
    }

    // 4. Work history: recurring series first (tasks reference them), then
    //    the tasks themselves (issue #102 — these were silently dropped).
    for s in source.task_series_list().await? {
        target.task_series_create(&s).await?;
        report.task_series += 1;
    }
    for t in source.task_list().await? {
        target.task_create(&t).await?;
        report.tasks += 1;
    }

    // 5. The append-only field-event journal. Copied last: events reference
    //    tasks/plantings by a non-FK `target_id`, so ordering is not enforced,
    //    but copying after the targets keeps the intent clear. The create is
    //    idempotent (conflict-no-op), so a re-run never duplicates.
    for e in source.field_event_list_all().await? {
        target.field_event_create(&e).await?;
        report.field_events += 1;
    }

    Ok(report)
}

/// True if `target` already holds any record across the catalog or data
/// tables — i.e. it is not a fresh, unseeded schema. One cheap read per table;
/// short-circuits on the first non-empty one.
async fn target_has_data(target: &dyn Repository) -> AppResult<bool> {
    Ok(!target.family_list().await?.is_empty()
        || !target.strata_list().await?.is_empty()
        || !target.location_kind_list().await?.is_empty()
        || !target.task_type_list().await?.is_empty()
        || !target.location_list().await?.is_empty()
        || !target.crop_list().await?.is_empty()
        || !target.variety_list().await?.is_empty()
        || !target.planting_list().await?.is_empty()
        || !target.task_list().await?.is_empty()
        || !target.crop_plan_line_list().await?.is_empty()
        || target_has_itk(target).await?
        || !target.field_event_list_all().await?.is_empty())
}

/// True if any crop in `target` already carries an ITK template. Separate helper
/// because ITK is only reachable per-crop (no flat list).
async fn target_has_itk(target: &dyn Repository) -> AppResult<bool> {
    for crop in target.crop_list().await? {
        if target.itk_template_get_for_crop(crop.id).await?.is_some() {
            return Ok(true);
        }
    }
    Ok(false)
}

fn push_with_ancestors(
    loc: &Location,
    by_id: &HashMap<LocationId, Location>,
    ordered: &mut Vec<Location>,
    emitted: &mut HashSet<LocationId>,
) {
    if emitted.contains(&loc.id) {
        return;
    }
    if let Some(pid) = loc.parent_id {
        if let Some(parent) = by_id.get(&pid) {
            push_with_ancestors(parent, by_id, ordered, emitted);
        }
    }
    emitted.insert(loc.id);
    ordered.push(loc.clone());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::{
        create_annual_planting, create_perennial_planting, record_yearly_harvest,
        AnnualPlantingRequest, PerennialPlantingRequest, YearlyHarvestRequest,
    };
    use crate::test_helpers::seed_test_data;
    use chrono::NaiveDate;
    use pomone_db::{
        seed_defaults, CropRepo, FamilyRepo, LocationRepo, PlantingRepo, SqliteRepository,
        StrataRepo, VarietyRepo,
    };
    use rust_decimal_macros::dec;

    async fn seeded_repo() -> SqliteRepository {
        let repo = SqliteRepository::in_memory().await.unwrap();
        seed_defaults(&repo).await.unwrap();
        repo
    }

    #[tokio::test]
    async fn copy_all_round_trips_seeded_data() {
        let source = seeded_repo().await;
        seed_test_data(&source).await.unwrap();
        let target = SqliteRepository::in_memory().await.unwrap();

        let report = copy_all(&source, &target).await.unwrap();
        assert!(report.families > 0);
        assert!(report.strata > 0);
        assert!(report.location_kinds > 0);
        assert!(report.locations > 0);
        assert!(report.crops > 0);
        assert!(report.varieties >= 2);

        // Spot-check that the data is identical.
        assert_eq!(
            source.family_list().await.unwrap(),
            target.family_list().await.unwrap()
        );
        assert_eq!(
            source.location_list().await.unwrap(),
            target.location_list().await.unwrap()
        );
        assert_eq!(
            source.variety_list().await.unwrap(),
            target.variety_list().await.unwrap()
        );
    }

    #[tokio::test]
    async fn copy_all_preserves_plantings_and_harvests() {
        let source = seeded_repo().await;
        seed_test_data(&source).await.unwrap();
        let varieties = source.variety_list().await.unwrap();
        let locations = source.location_list().await.unwrap();
        let bed = locations.iter().find(|l| l.parent_id.is_some()).unwrap();
        let strata = source.strata_list().await.unwrap()[0].id;
        create_annual_planting(
            &source,
            AnnualPlantingRequest::from_sowing(
                varieties[0].id,
                bed.id,
                strata,
                NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
                dec!(20),
                100,
            ),
        )
        .await
        .unwrap();

        let target = SqliteRepository::in_memory().await.unwrap();
        let report = copy_all(&source, &target).await.unwrap();
        assert_eq!(report.plantings, 1);
        assert_eq!(report.yearly_harvests, 0);

        let target_plantings = target
            .planting_list()
            .await
            .unwrap()
            .into_iter()
            .map(|p| p.id)
            .collect::<Vec<_>>();
        let source_plantings = source
            .planting_list()
            .await
            .unwrap()
            .into_iter()
            .map(|p| p.id)
            .collect::<Vec<_>>();
        assert_eq!(target_plantings, source_plantings);
    }

    #[tokio::test]
    async fn copy_all_handles_perennial_with_harvests() {
        use pomone_db::{CropRepo, FamilyRepo, LocationKindRepo};
        use pomone_domain::{
            Crop, Family, Lifespan, Location, LocationKind, PluriannualProfile, PruningSeason,
            Strata, Variety, VarietyProfile,
        };
        let source = seeded_repo().await;
        // Build a perennial setup separately so we can record harvests.
        let f = Family::new("Rosaceae", None, None).unwrap();
        let s = Strata::new("Arborée", None, None, None, 500).unwrap();
        let k = LocationKind::new("Verger", None).unwrap();
        source.family_create(&f).await.unwrap();
        source.strata_create(&s).await.unwrap();
        source.location_kind_create(&k).await.unwrap();
        let crop = Crop::new(
            f.id,
            "Pommier",
            None,
            Lifespan::perennial(40, 3).unwrap(),
            PruningSeason::Winter,
        )
        .unwrap();
        source.crop_create(&crop).await.unwrap();
        let variety = Variety::new(
            crop.id,
            Lifespan::perennial(40, 3).unwrap(),
            "Reinette",
            None,
            VarietyProfile::Pluriannual(
                PluriannualProfile::new(Some(80), Some(120), 220, 280, None).unwrap(),
            ),
        )
        .unwrap();
        source.variety_create(&variety).await.unwrap();
        let location = Location::new(k.id, "Verger Est", dec!(20), dec!(5), None, None).unwrap();
        source.location_create(&location).await.unwrap();
        let planting = create_perennial_planting(
            &source,
            PerennialPlantingRequest::new(
                variety.id,
                location.id,
                s.id,
                NaiveDate::from_ymd_opt(2026, 3, 15).unwrap(),
                dec!(100),
                10,
            ),
        )
        .await
        .unwrap();
        record_yearly_harvest(
            &source,
            YearlyHarvestRequest::new(planting.id, 2030)
                .with_expected_yield(dec!(50))
                .with_actual_yield(dec!(48)),
        )
        .await
        .unwrap();

        let target = SqliteRepository::in_memory().await.unwrap();
        let report = copy_all(&source, &target).await.unwrap();
        assert!(report.plantings >= 1);
        assert_eq!(report.yearly_harvests, 1);
    }

    #[tokio::test]
    async fn copy_all_carries_tasks_and_treatments_over() {
        // Issue #102: work history and phytosanitary records used to be
        // silently dropped by backend migration.
        use crate::services::{record_treatment, TreatmentRequest};
        use pomone_db::{FieldEventRepo, TaskRepo, TaskTypeRepo, TreatmentRepo};
        use pomone_domain::{FactKind, FieldEvent};

        let source = seeded_repo().await;
        seed_test_data(&source).await.unwrap();
        let varieties = source.variety_list().await.unwrap();
        let locations = source.location_list().await.unwrap();
        let bed = locations.iter().find(|l| l.parent_id.is_some()).unwrap();
        let strata = source.strata_list().await.unwrap()[0].id;
        // Creating a planting auto-generates its sow/harvest tasks.
        let planting = create_annual_planting(
            &source,
            AnnualPlantingRequest::from_sowing(
                varieties[0].id,
                bed.id,
                strata,
                NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
                dec!(20),
                100,
            ),
        )
        .await
        .unwrap();
        record_treatment(
            &source,
            TreatmentRequest::new(
                planting.id,
                NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(),
                "cuivre",
                "Bouillie bordelaise",
                dec!(1.25),
                "kg/ha",
            ),
        )
        .await
        .unwrap();
        let source_tasks = source.task_list().await.unwrap();
        assert!(!source_tasks.is_empty(), "autogen should have made tasks");

        // A field event on the first task — the journal must survive migration.
        let first_task = source_tasks[0].id;
        let event = FieldEvent::new(
            FactKind::TaskDone,
            "task",
            first_task.as_uuid(),
            NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
            NaiveDate::from_ymd_opt(2026, 3, 2)
                .unwrap()
                .and_hms_opt(9, 0, 0)
                .unwrap(),
            "{}",
            None,
        )
        .unwrap();
        source.field_event_create(&event).await.unwrap();

        let target = SqliteRepository::in_memory().await.unwrap();
        let report = copy_all(&source, &target).await.unwrap();

        assert!(report.task_types > 0, "seeded task types must be copied");
        assert_eq!(report.tasks, source_tasks.len());
        assert_eq!(report.treatments, 1);
        assert_eq!(report.field_events, 1);
        assert_eq!(target.task_list().await.unwrap(), source_tasks);
        assert_eq!(
            target
                .treatment_list_for_planting(planting.id)
                .await
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            target.field_event_get(event.id).await.unwrap(),
            Some(event),
            "the field-event journal must migrate intact"
        );
        assert_eq!(
            source.task_type_list().await.unwrap(),
            target.task_type_list().await.unwrap()
        );
    }

    #[tokio::test]
    async fn copy_all_carries_crop_plan_lines_over() {
        use pomone_db::CropPlanLineRepo;
        use pomone_domain::CropPlanLine;

        let source = seeded_repo().await;
        seed_test_data(&source).await.unwrap();
        let variety = source.variety_list().await.unwrap()[0].id;
        // Two lines (one a draft) so the copy exercises more than a single row.
        let line1 =
            CropPlanLine::new(variety, 6, dec!(15), 14, true, Some("batavia".into())).unwrap();
        let line2 = CropPlanLine::new(variety, 3, dec!(20), 0, false, None).unwrap();
        source.crop_plan_line_create(&line1).await.unwrap();
        source.crop_plan_line_create(&line2).await.unwrap();

        let target = SqliteRepository::in_memory().await.unwrap();
        let report = copy_all(&source, &target).await.unwrap();

        assert_eq!(report.crop_plan_lines, 2);
        assert_eq!(
            target.crop_plan_line_get(line1.id).await.unwrap(),
            Some(line1),
            "the plan line must migrate intact"
        );
        assert_eq!(
            target.crop_plan_line_get(line2.id).await.unwrap(),
            Some(line2)
        );
    }

    #[tokio::test]
    async fn copy_all_carries_itk_templates_and_activities_over() {
        use pomone_db::{CropRepo, ItkRepo, TaskTypeRepo};
        use pomone_domain::{ItkActivity, ItkTemplate};

        let source = seeded_repo().await;
        seed_test_data(&source).await.unwrap();
        let crop = source.crop_list().await.unwrap()[0].id;
        let tt = source.task_type_list().await.unwrap()[0].id;
        let template = ItkTemplate::new(crop);
        source.itk_template_create(&template).await.unwrap();
        let a0 = ItkActivity::new(
            template.id,
            tt,
            -10,
            None,
            None,
            Some("prépa".into()),
            0,
            None,
        );
        let a1 = ItkActivity::new(template.id, tt, 20, None, None, None, 1, None);
        source.itk_activity_create(&a0).await.unwrap();
        source.itk_activity_create(&a1).await.unwrap();

        let target = SqliteRepository::in_memory().await.unwrap();
        let report = copy_all(&source, &target).await.unwrap();

        assert_eq!(report.itk_templates, 1);
        assert_eq!(report.itk_activities, 2);
        assert_eq!(
            target.itk_template_get_for_crop(crop).await.unwrap(),
            Some(template.clone())
        );
        assert_eq!(
            target
                .itk_activity_list_for_template(template.id)
                .await
                .unwrap(),
            vec![a0, a1],
            "activities migrate intact and stay position-ordered"
        );
    }

    #[tokio::test]
    async fn copy_all_refuses_a_non_empty_target() {
        let source = seeded_repo().await;
        seed_test_data(&source).await.unwrap();
        // A target that already holds data (here: seeded defaults) must be
        // rejected before any write, rather than colliding on duplicate PKs.
        let target = seeded_repo().await;

        let err = copy_all(&source, &target).await.unwrap_err();
        assert!(matches!(err, AppError::MigrationTargetNotEmpty));
        // Nothing from the source leaked in.
        assert_eq!(target.crop_list().await.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn copy_all_emits_locations_in_parent_first_order() {
        use pomone_db::{LocationKindRepo, StrataRepo};
        use pomone_domain::{LocationKind, Strata};
        // Build a 3-level location chain in the source.
        let source = SqliteRepository::in_memory().await.unwrap();
        let _ = source
            .strata_create(&Strata::new("X", None, None, None, 10).unwrap())
            .await;
        let k = LocationKind::new("Lieu", None).unwrap();
        source.location_kind_create(&k).await.unwrap();
        let farm =
            pomone_domain::Location::new(k.id, "Ferme", dec!(100), dec!(100), None, None).unwrap();
        source.location_create(&farm).await.unwrap();
        let bed =
            pomone_domain::Location::new(k.id, "Planche", dec!(5), dec!(1), Some(farm.id), None)
                .unwrap();
        source.location_create(&bed).await.unwrap();
        let row =
            pomone_domain::Location::new(k.id, "Rang", dec!(5), dec!(0.3), Some(bed.id), None)
                .unwrap();
        source.location_create(&row).await.unwrap();

        let target = SqliteRepository::in_memory().await.unwrap();
        let report = copy_all(&source, &target).await.unwrap();
        assert_eq!(report.locations, 3);
        let copied = target.location_list().await.unwrap();
        assert_eq!(copied.len(), 3);
    }
}
