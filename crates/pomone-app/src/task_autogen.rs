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
use pomone_domain::{Planting, PlantingSchedule, Task, TaskCategory, TaskType};

/// Generate and persist the standard tasks for a planting, returning what
/// was created. Best-effort: a missing seed type for a category is logged
/// and skipped rather than aborting.
pub async fn generate_tasks_for_planting(
    repo: &dyn Repository,
    planting: &Planting,
) -> AppResult<Vec<Task>> {
    // One DB round-trip to list types; index them by category for O(1)
    // lookups (a future fix-or-create flow could insert missing categories
    // on the fly — for now, missing = skip).
    let types = repo.task_type_list().await?;
    let category_of: std::collections::HashMap<_, _> =
        types.iter().map(|t| (t.id, t.category)).collect();
    let planting_tasks = repo.task_list_for_planting(planting.id).await?;
    // Idempotency guard (issue #69): a (type, date) pair the planting already
    // carries is not re-created.
    let existing: std::collections::HashSet<_> = planting_tasks
        .iter()
        .map(|t| (t.task_type_id, t.planned_on))
        .collect();
    // Skip-aware guard (story 1.3): a phase that is already SETTLED (done or
    // skipped) for this planting must not be regenerated even at a shifted date
    // after a replan — a deliberate decision is never resurrected. Keyed on the
    // task's *category*, not its type, because one agronomic slot can resolve
    // to two types: establishment is "Repiquage" (raised) or "Plantation"
    // (bought), both category `Transplant` — a replan that flips the method
    // must not resurrect a settled establishment.
    //
    // This assumes the generator emits at most one task per category per
    // planting (Sow / Transplant / Harvest / Plant), which holds today; a
    // future recurring-per-planting generator would need an explicit
    // campaign-window in the key.
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
}
