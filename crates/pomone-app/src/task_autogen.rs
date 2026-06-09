//! Generate the standard task set for a planting.
//!
//! When the UI creates a new planting (annual `Cycle` or `Perennial`), the
//! services layer calls [`generate_tasks_for_planting`] to populate the
//! corresponding operational tasks: a Sow on the sowing date, a Transplant
//! on the transplant date, a Harvest on the first-harvest date for cycles;
//! a Transplant on the establishment date for perennials. The user can
//! mark them complete from the task calendar (PR E UI) and add ad-hoc
//! tasks later.
//!
//! The generator is intentionally permissive: missing optional dates just
//! skip the corresponding task, and a missing default `TaskType` (the
//! seed was deleted) is logged as a warning instead of failing — the
//! planting was already created, we don't want to roll it back over a
//! taxonomy quibble.
//!
//! This module is idempotent in spirit: callers should only invoke it
//! once per planting (typically right after `planting_create`). It does
//! not check whether tasks already exist; running it twice would create
//! duplicates.

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
    let triggers = phase_dates(planting);
    let mut created = Vec::with_capacity(triggers.len());
    for (category, date) in triggers {
        match find_type(&types, category) {
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
                    ?category, planting_id = %planting.id,
                    "no TaskType seeded for category — skipping auto-generated task",
                );
            }
        }
    }
    Ok(created)
}

/// The (category, date) pairs we want to materialize for a given planting.
/// Pure — no DB access — so it's easy to unit-test the rules.
fn phase_dates(planting: &Planting) -> Vec<(TaskCategory, NaiveDate)> {
    let mut out: Vec<(TaskCategory, NaiveDate)> = Vec::new();
    match planting.schedule {
        PlantingSchedule::Cycle {
            sown_on,
            transplanted_on,
            first_harvest_on,
            ..
        } => {
            if let Some(d) = sown_on {
                out.push((TaskCategory::Sow, d));
            }
            if let Some(d) = transplanted_on {
                out.push((TaskCategory::Transplant, d));
            }
            out.push((TaskCategory::Harvest, first_harvest_on));
        }
        PlantingSchedule::Perennial { established_on, .. } => {
            // "Établissement" of a perennial = putting it in the ground.
            // We file it under Transplant (the closest semantic match)
            // rather than coining a new category just for this.
            out.push((TaskCategory::Transplant, established_on));
        }
    }
    out
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
                (TaskCategory::Sow, d(2026, 3, 1)),
                (TaskCategory::Transplant, d(2026, 5, 1)),
                (TaskCategory::Harvest, d(2026, 7, 1)),
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
                (TaskCategory::Sow, d(2026, 3, 1)),
                (TaskCategory::Harvest, d(2026, 6, 1)),
            ]
        );
    }

    #[test]
    fn cycle_with_only_harvest_dates_yields_a_single_harvest_trigger() {
        // E.g. a bought-as-plants planting where no sowing was tracked.
        let p = cycle_planting(None, None, d(2026, 6, 1), d(2026, 7, 1));
        assert_eq!(
            phase_dates(&p),
            vec![(TaskCategory::Harvest, d(2026, 6, 1))]
        );
    }

    #[test]
    fn perennial_yields_one_transplant_trigger() {
        let p = perennial_planting(d(2026, 3, 15));
        assert_eq!(
            phase_dates(&p),
            vec![(TaskCategory::Transplant, d(2026, 3, 15))]
        );
    }
}
