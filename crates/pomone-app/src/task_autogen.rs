//! Generate the standard task set for a planting.
//!
//! When the UI creates a new planting (annual `Cycle` or `Perennial`), the
//! services layer calls [`generate_tasks_for_planting`] to populate the
//! corresponding operational tasks: a Sow on the sowing date, a Transplant
//! on the transplant date, a Harvest on the first-harvest date for cycles.
//! Perennials get no auto task at establishment (bought stock → nothing to
//! transplant). The user can
//! mark them complete from the task calendar (PR E UI) and add ad-hoc
//! tasks later.
//!
//! The generator is intentionally permissive: missing optional dates just
//! skip the corresponding task, and a missing default `TaskType` (the
//! seed was deleted) is logged as a warning instead of failing — the
//! planting was already created, we don't want to roll it back over a
//! taxonomy quibble.
//!
//! The generator is idempotent (issue #69): before creating a task it
//! checks the planting's existing tasks and skips any (type, date) pair
//! that is already there, so calling it twice — or on a planting the user
//! already gave tasks — never creates duplicates.

use crate::error::AppResult;
use chrono::NaiveDate;
use pomone_db::Repository;
use pomone_domain::ids::TaskTypeId;
use pomone_domain::{
    date_calc, ItkActivity, Planting, PlantingSchedule, Task, TaskCategory, TaskType,
};

/// Generate and persist the operational tasks for a planting, returning what
/// was created. Best-effort: a missing seed type / an out-of-range date is
/// logged and skipped rather than aborting.
///
/// **Two paths (story 3.3):** when the planting's crop has an ITK template with
/// activities, those activities *are* the tasks (each `task_type_id` at
/// `establishment + offset_days`, incl. J-negative preparation). A crop without
/// an ITK keeps the shipped variety-profile auto-gen (Sow / Transplant / Plant
/// / Harvest) — the fallback below, unchanged.
pub async fn generate_tasks_for_planting(
    repo: &dyn Repository,
    planting: &Planting,
) -> AppResult<Vec<Task>> {
    let planting_tasks = repo.task_list_for_planting(planting.id).await?;
    // Idempotency guard (issue #69): a (type, date) pair the planting already
    // carries is not re-created. Shared by both paths.
    let existing: std::collections::HashSet<_> = planting_tasks
        .iter()
        .map(|t| (t.task_type_id, t.planned_on))
        .collect();

    // ITK path if the crop has a template with activities.
    if let Some(activities) = itk_activities(repo, planting).await? {
        return generate_from_itk(repo, planting, &activities, &planting_tasks, &existing).await;
    }
    // Fallback: variety-profile auto-gen (unchanged from before story 3.3).
    generate_from_profile(repo, planting, &planting_tasks, &existing).await
}

/// The crop's ITK activities, if it has a non-empty template — else `None`
/// (→ profile fallback). Resolves `planting → variety → crop → template`.
async fn itk_activities(
    repo: &dyn Repository,
    planting: &Planting,
) -> AppResult<Option<Vec<ItkActivity>>> {
    let Some(variety) = repo.variety_get(planting.variety_id).await? else {
        return Ok(None); // variety vanished — fall back defensively.
    };
    let Some(template) = repo.itk_template_get_for_crop(variety.crop_id).await? else {
        return Ok(None);
    };
    let activities = repo.itk_activity_list_for_template(template.id).await?;
    Ok(if activities.is_empty() {
        None
    } else {
        Some(activities)
    })
}

/// The establishment anchor the ITK offsets are measured from — the bed's
/// establishment date: transplant → sow → first-harvest (cycle), or
/// `established_on` (perennial). Same rule as `capacity::occupancy_window`'s
/// start and `phase_dates`, so the whole codebase agrees on "establishment".
fn establishment_anchor(planting: &Planting) -> NaiveDate {
    match planting.schedule {
        PlantingSchedule::Cycle {
            sown_on,
            transplanted_on,
            first_harvest_on,
            ..
        } => transplanted_on.or(sown_on).unwrap_or(first_harvest_on),
        PlantingSchedule::Perennial { established_on, .. } => established_on,
    }
}

/// Combine an activity's label + notes into the task's notes field.
fn itk_notes(activity: &ItkActivity) -> Option<String> {
    match (&activity.label, &activity.notes) {
        (Some(l), Some(n)) => Some(format!("{l} — {n}")),
        (Some(l), None) => Some(l.clone()),
        (None, n) => n.clone(),
    }
}

/// Generate tasks from the crop's ITK activities.
///
/// Skip-aware guard (story 1.3), keyed for ITK on **`task_type_id`** (not
/// category): the ITK names an explicit type per activity, so a settled (done
/// or skipped) type is never regenerated at a shifted date on re-generation.
/// Assumes at most one activity per `task_type_id` per planting (mirrors the
/// profile path's "one per category" assumption); a future ITK with two
/// activities of the same type would need a `task.itk_activity_id` link.
///
/// Note: this is re-generation on the **same** planting id. Un-placing a
/// planting (story 3.2) deletes it and its tasks; re-placing creates a *new*
/// planting that legitimately gets fresh tasks — the prior placement, and its
/// skip decisions, were explicitly undone. That is not a resurrection.
async fn generate_from_itk(
    repo: &dyn Repository,
    planting: &Planting,
    activities: &[ItkActivity],
    planting_tasks: &[Task],
    existing: &std::collections::HashSet<(TaskTypeId, NaiveDate)>,
) -> AppResult<Vec<Task>> {
    let anchor = establishment_anchor(planting);
    let settled_types: std::collections::HashSet<TaskTypeId> = planting_tasks
        .iter()
        .filter(|t| t.is_settled())
        .map(|t| t.task_type_id)
        .collect();

    let mut created = Vec::with_capacity(activities.len());
    for activity in activities {
        // Signed offset from establishment; an out-of-range offset is skipped,
        // not fatal (best-effort contract).
        let date = match date_calc::offset_days(anchor, activity.offset_days) {
            Ok(d) => d,
            Err(e) => {
                tracing::warn!(
                    error = %e, planting_id = %planting.id, offset = activity.offset_days,
                    "ITK activity offset out of date range — skipping",
                );
                continue;
            }
        };
        if existing.contains(&(activity.task_type_id, date))
            || settled_types.contains(&activity.task_type_id)
        {
            tracing::debug!(
                planting_id = %planting.id, task_type_id = %activity.task_type_id,
                "ITK task already exists or its type is settled — skipping",
            );
            continue;
        }
        let task = Task::new(
            Some(planting.id),
            Some(planting.location_id),
            activity.task_type_id,
            activity.method_id,
            activity.implement_id,
            date,
            None,
            None,
            None,
            itk_notes(activity),
        );
        repo.task_create(&task).await?;
        created.push(task);
    }
    Ok(created)
}

/// Generate tasks from the variety profile (Sow / Transplant / Plant /
/// Harvest) — the fallback for a crop with no ITK. Unchanged from before
/// story 3.3.
async fn generate_from_profile(
    repo: &dyn Repository,
    planting: &Planting,
    planting_tasks: &[Task],
    existing: &std::collections::HashSet<(TaskTypeId, NaiveDate)>,
) -> AppResult<Vec<Task>> {
    // One DB round-trip to list types; index them by category for O(1) lookups.
    let types = repo.task_type_list().await?;
    let category_of: std::collections::HashMap<_, _> =
        types.iter().map(|t| (t.id, t.category)).collect();
    // Skip-aware guard (story 1.3): a phase already SETTLED (done or skipped)
    // for this planting must not be regenerated even at a shifted date after a
    // replan. Keyed on *category*, not type, because one agronomic slot can
    // resolve to two types: establishment is "Repiquage" (raised) or
    // "Plantation" (bought), both category `Transplant` — a replan that flips
    // the method must not resurrect a settled establishment.
    //
    // Assumes at most one task per category per planting (Sow / Transplant /
    // Harvest / Plant), which holds today; a future recurring-per-planting
    // generator would need an explicit campaign-window in the key.
    let settled_categories: std::collections::HashSet<_> = planting_tasks
        .iter()
        .filter(|t| t.is_settled())
        .filter_map(|t| category_of.get(&t.task_type_id).copied())
        .collect();
    let triggers = phase_dates(planting);
    let mut created = Vec::with_capacity(triggers.len());
    for (trigger, date) in triggers {
        match resolve_type(&types, trigger) {
            Some(tt)
                if existing.contains(&(tt.id, date))
                    || settled_categories.contains(&tt.category) =>
            {
                tracing::debug!(
                    ?trigger, planting_id = %planting.id,
                    "auto-generated task already exists or its phase is settled — skipping",
                );
            }
            Some(tt) => {
                let task = Task::new(
                    Some(planting.id),
                    Some(planting.location_id),
                    tt.id,
                    None,
                    None,
                    date,
                    None,
                    None,
                    None,
                    None,
                );
                repo.task_create(&task).await?;
                created.push(task);
            }
            None => {
                tracing::warn!(
                    ?trigger, planting_id = %planting.id,
                    "no TaskType available for trigger — skipping auto-generated task",
                );
            }
        }
    }
    Ok(created)
}

/// What kind of operation a planting's schedule implies, used to pick the
/// right task type. `Transplant` (raised seedlings) and `Plant` (bought
/// stock / perennial establishment) both map to the Transplant category but
/// to different task types so the calendar reads correctly (establishment
/// methods).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Trigger {
    Sow,
    Transplant,
    Plant,
    Harvest,
}

/// The (trigger, date) pairs we want to materialize for a given planting.
/// Pure — no DB access — so it's easy to unit-test the rules.
///
/// Methods are inferred from which dates the `Cycle` carries: a sowing date
/// implies `Sow`; a transplant date *with* a sowing date is a `Transplant`
/// (you raised the plants), *without* a sowing date is a `Plant` (bought
/// plants). Perennials are always `Plant` at establishment.
fn phase_dates(planting: &Planting) -> Vec<(Trigger, NaiveDate)> {
    let mut out: Vec<(Trigger, NaiveDate)> = Vec::new();
    match planting.schedule {
        PlantingSchedule::Cycle {
            sown_on,
            transplanted_on,
            first_harvest_on,
            ..
        } => {
            if let Some(d) = sown_on {
                out.push((Trigger::Sow, d));
            }
            if let Some(d) = transplanted_on {
                out.push((
                    if sown_on.is_some() {
                        Trigger::Transplant
                    } else {
                        Trigger::Plant
                    },
                    d,
                ));
            }
            out.push((Trigger::Harvest, first_harvest_on));
        }
        PlantingSchedule::Perennial { established_on, .. } => {
            // Bought stock put in the ground → a Plantation task (issue: a tree
            // is planted, not transplanted from a nursery you raised).
            out.push((Trigger::Plant, established_on));
        }
    }
    out
}

/// Resolve the task type for a trigger. Sow/Harvest/Transplant use the first
/// type of their category (as before). `Plant` prefers the "Plantation" type
/// (a Transplant-category type) and falls back to the first Transplant type
/// for databases seeded before it existed.
fn resolve_type(types: &[TaskType], trigger: Trigger) -> Option<&TaskType> {
    match trigger {
        Trigger::Sow => find_type(types, TaskCategory::Sow),
        Trigger::Harvest => find_type(types, TaskCategory::Harvest),
        Trigger::Transplant => types
            .iter()
            .find(|t| t.category == TaskCategory::Transplant && t.name != "Plantation")
            .or_else(|| find_type(types, TaskCategory::Transplant)),
        Trigger::Plant => types
            .iter()
            .find(|t| t.category == TaskCategory::Transplant && t.name == "Plantation")
            .or_else(|| find_type(types, TaskCategory::Transplant)),
    }
}

fn find_type(types: &[TaskType], category: TaskCategory) -> Option<&TaskType> {
    types.iter().find(|t| t.category == category)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use pomone_domain::{Lifespan, LocationId, StrataId, VarietyId};
    use rust_decimal_macros::dec;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    fn cycle_planting(
        sown: Option<NaiveDate>,
        transplanted: Option<NaiveDate>,
        first_harvest: NaiveDate,
        last_harvest: NaiveDate,
    ) -> Planting {
        Planting::new(
            VarietyId::new(),
            LocationId::new(),
            StrataId::new(),
            Lifespan::Annual,
            dec!(10),
            10,
            PlantingSchedule::cycle(sown, transplanted, first_harvest, last_harvest).unwrap(),
            None,
            None,
        )
        .unwrap()
    }

    fn perennial_planting(established: NaiveDate) -> Planting {
        let lifespan = Lifespan::perennial(20, 3).unwrap();
        Planting::new(
            VarietyId::new(),
            LocationId::new(),
            StrataId::new(),
            lifespan,
            dec!(100),
            5,
            PlantingSchedule::perennial(established, None).unwrap(),
            None,
            None,
        )
        .unwrap()
    }

    #[test]
    fn cycle_with_sow_and_transplant_yields_three_triggers() {
        let p = cycle_planting(
            Some(d(2026, 3, 1)),
            Some(d(2026, 5, 1)),
            d(2026, 7, 1),
            d(2026, 8, 15),
        );
        let triggers = phase_dates(&p);
        assert_eq!(
            triggers,
            vec![
                (Trigger::Sow, d(2026, 3, 1)),
                (Trigger::Transplant, d(2026, 5, 1)),
                (Trigger::Harvest, d(2026, 7, 1)),
            ]
        );
    }

    #[test]
    fn direct_sow_cycle_skips_transplant() {
        let p = cycle_planting(Some(d(2026, 3, 1)), None, d(2026, 6, 1), d(2026, 7, 1));
        let triggers = phase_dates(&p);
        assert_eq!(
            triggers,
            vec![
                (Trigger::Sow, d(2026, 3, 1)),
                (Trigger::Harvest, d(2026, 6, 1)),
            ]
        );
    }

    #[test]
    fn bought_plants_cycle_yields_plant_then_harvest() {
        // No sowing tracked + a planting date → Plant (not Transplant).
        let p = cycle_planting(None, Some(d(2026, 5, 1)), d(2026, 6, 1), d(2026, 7, 1));
        assert_eq!(
            phase_dates(&p),
            vec![
                (Trigger::Plant, d(2026, 5, 1)),
                (Trigger::Harvest, d(2026, 6, 1)),
            ]
        );
    }

    #[test]
    fn perennial_yields_a_plant_trigger_at_establishment() {
        let p = perennial_planting(d(2026, 3, 15));
        assert_eq!(phase_dates(&p), vec![(Trigger::Plant, d(2026, 3, 15))]);
    }

    /// Story 2.2 clause 5: a crop with **no** ITK keeps the shipped
    /// variety-profile autogen (fallback). ITK persistence must not change
    /// generation for ITK-less crops — generation stays variety-profile-driven
    /// until story 2.6 wires ITKs in.
    #[tokio::test]
    async fn itk_less_crop_keeps_variety_profile_autogen() {
        use crate::services::{create_annual_planting, AnnualPlantingRequest};
        use crate::test_helpers::seed_test_data;
        use pomone_db::{
            seed_defaults, ItkRepo, LocationRepo, SqliteRepository, StrataRepo, TaskRepo,
            VarietyRepo,
        };

        let repo = SqliteRepository::in_memory().await.unwrap();
        seed_defaults(&repo).await.unwrap();
        seed_test_data(&repo).await.unwrap();
        let variety = repo.variety_list().await.unwrap()[0].clone();
        let locations = repo.location_list().await.unwrap();
        let bed = locations.iter().find(|l| l.parent_id.is_some()).unwrap();
        let strata = repo.strata_list().await.unwrap()[0].id;

        // The crop has no ITK template.
        assert!(
            repo.itk_template_get_for_crop(variety.crop_id)
                .await
                .unwrap()
                .is_none(),
            "precondition: crop has no ITK"
        );

        let planting = create_annual_planting(
            &repo,
            AnnualPlantingRequest::from_sowing(
                variety.id,
                bed.id,
                strata,
                d(2026, 3, 1),
                dec!(20),
                100,
            ),
        )
        .await
        .unwrap();

        // Variety-profile autogen still produced the cycle's tasks.
        let tasks = repo.task_list_for_planting(planting.id).await.unwrap();
        assert!(
            !tasks.is_empty(),
            "ITK-less crop must still autogen from the variety profile"
        );
    }

    #[tokio::test]
    async fn generating_twice_creates_no_duplicates() {
        use crate::services::{create_annual_planting, AnnualPlantingRequest};
        use crate::test_helpers::seed_test_data;
        use pomone_db::{
            seed_defaults, LocationRepo, PlantingRepo, SqliteRepository, StrataRepo, TaskRepo,
            VarietyRepo,
        };

        let repo = SqliteRepository::in_memory().await.unwrap();
        seed_defaults(&repo).await.unwrap();
        seed_test_data(&repo).await.unwrap();
        let varieties = repo.variety_list().await.unwrap();
        let locations = repo.location_list().await.unwrap();
        let bed = locations.iter().find(|l| l.parent_id.is_some()).unwrap();
        let strata = repo.strata_list().await.unwrap()[0].id;
        // Creation runs the generator once.
        let planting = create_annual_planting(
            &repo,
            AnnualPlantingRequest::from_sowing(
                varieties[0].id,
                bed.id,
                strata,
                d(2026, 3, 1),
                dec!(20),
                100,
            ),
        )
        .await
        .unwrap();
        let planting = repo.planting_get(planting.id).await.unwrap().unwrap();
        let before = repo.task_list_for_planting(planting.id).await.unwrap();
        assert!(!before.is_empty());

        // Second run must be a no-op (issue #69).
        let created = generate_tasks_for_planting(&repo, &planting).await.unwrap();
        assert!(created.is_empty(), "second run must not create tasks");
        let after = repo.task_list_for_planting(planting.id).await.unwrap();
        assert_eq!(after.len(), before.len());
    }

    #[tokio::test]
    async fn settled_task_is_not_resurrected_after_replan() {
        use crate::facts::{record_fact, Fact};
        use crate::services::{create_annual_planting, AnnualPlantingRequest};
        use crate::test_helpers::seed_test_data;
        use pomone_db::{
            seed_defaults, LocationRepo, PlantingRepo, SqliteRepository, StrataRepo, TaskRepo,
            VarietyRepo,
        };
        use pomone_domain::{Planting, PlantingSchedule, SkipReason};

        let repo = SqliteRepository::in_memory().await.unwrap();
        seed_defaults(&repo).await.unwrap();
        seed_test_data(&repo).await.unwrap();
        let varieties = repo.variety_list().await.unwrap();
        let bed = repo
            .location_list()
            .await
            .unwrap()
            .into_iter()
            .find(|l| l.parent_id.is_some())
            .unwrap();
        let strata = repo.strata_list().await.unwrap()[0].id;
        let planting = create_annual_planting(
            &repo,
            AnnualPlantingRequest::from_sowing(
                varieties[0].id,
                bed.id,
                strata,
                d(2026, 3, 1),
                dec!(20),
                100,
            ),
        )
        .await
        .unwrap();
        let planting = repo.planting_get(planting.id).await.unwrap().unwrap();

        // The grower skips the earliest auto-task (the sow).
        let tasks = repo.task_list_for_planting(planting.id).await.unwrap();
        let sow = tasks.iter().min_by_key(|t| t.planned_on).unwrap().clone();
        let sow_type = sow.task_type_id;
        record_fact(
            &repo,
            Fact::Skipped {
                task_id: sow.id,
                on: d(2026, 3, 1),
                reason: SkipReason::Weather,
                note: None,
            },
            d(2026, 3, 1).and_hms_opt(9, 0, 0).unwrap(),
        )
        .await
        .unwrap();

        // Replan: shift the whole cycle two weeks later and re-run autogen.
        let replanned = Planting {
            schedule: PlantingSchedule::cycle(
                Some(d(2026, 3, 15)),
                Some(d(2026, 4, 19)),
                d(2026, 6, 1),
                d(2026, 8, 1),
            )
            .unwrap(),
            ..planting
        };
        generate_tasks_for_planting(&repo, &replanned)
            .await
            .unwrap();

        // The skipped sow slot is NOT resurrected at the new date.
        let after = repo.task_list_for_planting(replanned.id).await.unwrap();
        let sow_count = after.iter().filter(|t| t.task_type_id == sow_type).count();
        assert_eq!(
            sow_count, 1,
            "a settled (skipped) task type must not be regenerated after a replan"
        );
    }

    /// Establishment is "Repiquage" (raised) or "Plantation" (bought) — two
    /// types, one category, one agronomic slot. A DONE Repiquage must not be
    /// resurrected as a Plantation when a replan drops the sow date (method
    /// flip). Also exercises the DONE branch of the settled guard.
    #[tokio::test]
    async fn settled_establishment_survives_method_flip_on_replan() {
        use crate::facts::{record_fact, Fact};
        use crate::services::{create_annual_planting, AnnualPlantingRequest};
        use crate::test_helpers::seed_test_data;
        use pomone_db::{
            seed_defaults, LocationRepo, PlantingRepo, SqliteRepository, StrataRepo, TaskRepo,
            TaskTypeRepo, VarietyRepo,
        };
        use pomone_domain::{Planting, PlantingSchedule};

        let repo = SqliteRepository::in_memory().await.unwrap();
        seed_defaults(&repo).await.unwrap();
        seed_test_data(&repo).await.unwrap();
        let varieties = repo.variety_list().await.unwrap();
        let bed = repo
            .location_list()
            .await
            .unwrap()
            .into_iter()
            .find(|l| l.parent_id.is_some())
            .unwrap();
        let strata = repo.strata_list().await.unwrap()[0].id;
        // Raised transplant: sow + transplant present → a Repiquage task.
        let planting = create_annual_planting(
            &repo,
            AnnualPlantingRequest::from_sowing(
                varieties[0].id,
                bed.id,
                strata,
                d(2026, 3, 1),
                dec!(20),
                100,
            ),
        )
        .await
        .unwrap();
        let planting = repo.planting_get(planting.id).await.unwrap().unwrap();

        let types = repo.task_type_list().await.unwrap();
        let cat_of = |id| types.iter().find(|t| t.id == id).map(|t| t.category);
        let tasks = repo.task_list_for_planting(planting.id).await.unwrap();
        let repiquage = tasks
            .iter()
            .find(|t| cat_of(t.task_type_id) == Some(TaskCategory::Transplant))
            .expect("raised transplant yields a Repiquage task");

        // Mark the Repiquage DONE.
        record_fact(
            &repo,
            Fact::Done {
                task_id: repiquage.id,
                on: d(2026, 4, 5),
            },
            d(2026, 4, 5).and_hms_opt(9, 0, 0).unwrap(),
        )
        .await
        .unwrap();

        // Replan as bought plants (no sow date) — the establishment trigger now
        // resolves to "Plantation", a *different* type but the same category.
        let replanned = Planting {
            schedule: PlantingSchedule::cycle(
                None,
                Some(d(2026, 3, 20)),
                d(2026, 6, 1),
                d(2026, 8, 1),
            )
            .unwrap(),
            ..planting
        };
        generate_tasks_for_planting(&repo, &replanned)
            .await
            .unwrap();

        // The done establishment slot is NOT resurrected as a Plantation: still
        // exactly one Transplant-category task.
        let after = repo.task_list_for_planting(replanned.id).await.unwrap();
        let establishment = after
            .iter()
            .filter(|t| cat_of(t.task_type_id) == Some(TaskCategory::Transplant))
            .count();
        assert_eq!(
            establishment, 1,
            "a settled establishment must not be regenerated across a method flip"
        );
    }

    // ---- ITK-driven generation (story 3.3) ---------------------------------

    /// Seed a catalogue + an annual crop whose ITK has two activities:
    /// `t_prep` at J-14 and `t_care` at J+20 (distinct task types). Returns the
    /// pieces a planting/placement needs.
    async fn repo_with_itk() -> (
        pomone_db::SqliteRepository,
        VarietyId,
        LocationId,
        StrataId,
        pomone_domain::ids::TaskTypeId,
        pomone_domain::ids::TaskTypeId,
    ) {
        use pomone_db::{
            seed_defaults, CropRepo, FamilyRepo, ItkRepo, LocationKindRepo, LocationRepo,
            SqliteRepository, StrataRepo, TaskTypeRepo, VarietyRepo,
        };
        use pomone_domain::{
            AnnualProfile, Crop, Family, ItkActivity, ItkTemplate, Location, LocationKind,
            PruningSeason, Strata, Variety, VarietyProfile,
        };

        let repo = SqliteRepository::in_memory().await.unwrap();
        seed_defaults(&repo).await.unwrap();
        let types = repo.task_type_list().await.unwrap();
        let (t_prep, t_care) = (types[0].id, types[1].id);

        let fam = Family::new("Solanaceae", None, None).unwrap();
        repo.family_create(&fam).await.unwrap();
        let strata = Strata::new("Herbacée", None, None, None, 40).unwrap();
        repo.strata_create(&strata).await.unwrap();
        let kind = LocationKind::new("Planche", None).unwrap();
        repo.location_kind_create(&kind).await.unwrap();
        let bed = Location::new(kind.id, "Planche A", dec!(25), dec!(0.8), None, None).unwrap();
        repo.location_create(&bed).await.unwrap();
        let crop = Crop::new(
            fam.id,
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
            VarietyProfile::Annual(AnnualProfile::new(Some(30), 60, 30).unwrap()),
        )
        .unwrap();
        repo.variety_create(&variety).await.unwrap();

        let tpl = ItkTemplate::new(crop.id);
        repo.itk_template_create(&tpl).await.unwrap();
        repo.itk_activity_create(&ItkActivity::new(
            tpl.id,
            t_prep,
            -14,
            None,
            None,
            Some("prépa planche".into()),
            0,
            None,
        ))
        .await
        .unwrap();
        repo.itk_activity_create(&ItkActivity::new(
            tpl.id, t_care, 20, None, None, None, 1, None,
        ))
        .await
        .unwrap();

        (repo, variety.id, bed.id, strata.id, t_prep, t_care)
    }

    #[tokio::test]
    async fn itk_activities_become_dated_tasks_at_anchor_plus_offset() {
        use crate::services::{create_annual_planting, AnnualPlantingRequest};
        use pomone_db::TaskRepo;

        let (repo, vid, lid, sid, t_prep, t_care) = repo_with_itk().await;
        let planting = create_annual_planting(
            &repo,
            AnnualPlantingRequest::from_sowing(vid, lid, sid, d(2026, 3, 1), dec!(15), 50),
        )
        .await
        .unwrap();

        // Anchor = transplant date = sown + DTT(30) = 2026-03-31.
        let PlantingSchedule::Cycle {
            transplanted_on: Some(anchor),
            ..
        } = planting.schedule
        else {
            panic!("expected a raised-transplant cycle");
        };

        let tasks = repo.task_list_for_planting(planting.id).await.unwrap();
        // ITK path REPLACES the profile path: exactly the two ITK tasks.
        assert_eq!(tasks.len(), 2, "only the ITK activities, no profile tasks");
        let prep = tasks.iter().find(|t| t.task_type_id == t_prep).unwrap();
        let care = tasks.iter().find(|t| t.task_type_id == t_care).unwrap();
        assert_eq!(
            prep.planned_on,
            date_calc::offset_days(anchor, -14).unwrap()
        );
        assert!(
            prep.planned_on < anchor,
            "J-negative prep lands before establishment"
        );
        assert_eq!(care.planned_on, date_calc::offset_days(anchor, 20).unwrap());
        assert_eq!(prep.notes.as_deref(), Some("prépa planche"));
    }

    #[tokio::test]
    async fn itk_task_carries_method_and_implement() {
        use crate::services::{create_annual_planting, AnnualPlantingRequest};
        use pomone_db::{
            ItkRepo, TaskImplementRepo, TaskMethodRepo, TaskRepo, TaskTypeRepo, VarietyRepo,
        };
        use pomone_domain::{ItkActivity, TaskImplement, TaskMethod};

        let (repo, vid, lid, sid, _t_prep, t_care) = repo_with_itk().await;
        // Add a method + implement and a third activity that references them.
        let method = TaskMethod::new("Manuel", None).unwrap();
        repo.task_method_create(&method).await.unwrap();
        let implement = TaskImplement::new("Houe", None).unwrap();
        repo.task_implement_create(&implement).await.unwrap();
        let tpl = repo
            .itk_template_get_for_crop(repo.variety_get(vid).await.unwrap().unwrap().crop_id)
            .await
            .unwrap()
            .unwrap();
        // Reuse t_care's slot at a distinct type would collide; use a fresh type.
        let extra_type = repo.task_type_list().await.unwrap()[2].id;
        repo.itk_activity_create(&ItkActivity::new(
            tpl.id,
            extra_type,
            5,
            Some(method.id),
            Some(implement.id),
            None,
            2,
            None,
        ))
        .await
        .unwrap();

        let planting = create_annual_planting(
            &repo,
            AnnualPlantingRequest::from_sowing(vid, lid, sid, d(2026, 3, 1), dec!(15), 50),
        )
        .await
        .unwrap();
        let tasks = repo.task_list_for_planting(planting.id).await.unwrap();
        let carried = tasks.iter().find(|t| t.task_type_id == extra_type).unwrap();
        assert_eq!(carried.task_method_id, Some(method.id));
        assert_eq!(carried.implement_id, Some(implement.id));
        // The other activities still generated too.
        assert!(tasks.iter().any(|t| t.task_type_id == t_care));
    }

    #[tokio::test]
    async fn settled_itk_task_not_resurrected_after_regeneration() {
        use crate::facts::{record_fact, Fact};
        use crate::services::{create_annual_planting, AnnualPlantingRequest};
        use pomone_db::{PlantingRepo, TaskRepo};
        use pomone_domain::{Planting, PlantingSchedule, SkipReason};

        let (repo, vid, lid, sid, _t_prep, t_care) = repo_with_itk().await;
        let planting = create_annual_planting(
            &repo,
            AnnualPlantingRequest::from_sowing(vid, lid, sid, d(2026, 3, 1), dec!(15), 50),
        )
        .await
        .unwrap();
        let planting = repo.planting_get(planting.id).await.unwrap().unwrap();

        // Skip the J+20 care task.
        let tasks = repo.task_list_for_planting(planting.id).await.unwrap();
        let care = tasks
            .iter()
            .find(|t| t.task_type_id == t_care)
            .unwrap()
            .clone();
        record_fact(
            &repo,
            Fact::Skipped {
                task_id: care.id,
                on: care.planned_on,
                reason: SkipReason::NoTime,
                note: None,
            },
            care.planned_on.and_hms_opt(9, 0, 0).unwrap(),
        )
        .await
        .unwrap();

        // Replan: shift the cycle two weeks and regenerate on the same planting.
        let replanned = Planting {
            schedule: PlantingSchedule::cycle(
                Some(d(2026, 3, 15)),
                Some(d(2026, 4, 14)),
                d(2026, 6, 1),
                d(2026, 8, 1),
            )
            .unwrap(),
            ..planting
        };
        generate_tasks_for_planting(&repo, &replanned)
            .await
            .unwrap();

        // The settled care type is NOT regenerated at the shifted date.
        let after = repo.task_list_for_planting(replanned.id).await.unwrap();
        assert_eq!(
            after.iter().filter(|t| t.task_type_id == t_care).count(),
            1,
            "a settled (skipped) ITK type must not be resurrected on re-generation"
        );
    }

    #[tokio::test]
    async fn placement_emits_itk_tasks_end_to_end() {
        use crate::services::{place_planned_planting, PlacementRequest};
        use pomone_db::{CropPlanLineRepo, PlannedPlantingRepo, TaskRepo};
        use pomone_domain::{CropPlanLine, PlannedPlanting};

        let (repo, vid, lid, sid, t_prep, t_care) = repo_with_itk().await;
        // A planned succession to place.
        let line =
            CropPlanLine::new(vid, 1, dec!(15), 14, Some(d(2026, 4, 1)), false, None).unwrap();
        repo.crop_plan_line_create(&line).await.unwrap();
        let pp = PlannedPlanting::new(line.id, vid, 0, d(2026, 4, 1), dec!(15)).unwrap();
        repo.planned_planting_create(&pp).await.unwrap();

        let planting = place_planned_planting(&repo, PlacementRequest::new(pp.id, lid, sid, 60))
            .await
            .unwrap();

        // Placement generated the crop's ITK tasks.
        let tasks = repo.task_list_for_planting(planting.id).await.unwrap();
        assert!(tasks.iter().any(|t| t.task_type_id == t_prep));
        assert!(tasks.iter().any(|t| t.task_type_id == t_care));
    }
}
