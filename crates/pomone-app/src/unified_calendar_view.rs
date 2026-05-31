//! Presentation-layer helper for the **unified** calendar (issue #47).
//!
//! Merges the two families of dated entries that used to live on separate
//! screens into one list, ready to render on a single monthly grid:
//!
//! * **Tasks** — operational work (`Task`): editable, completable, draggable.
//!   Reuses [`list_task_calendar_rows`] so labels, colors and the per-category
//!   filter behave exactly like the standalone task calendar did.
//! * **Milestones** — crop-cycle events derived from planting schedules
//!   (sowing, harvest, perennial phenology…). Read-only; the UI routes to the
//!   planting on click.
//!
//! ## De-duplication at the source
//!
//! Creating a planting auto-generates Sow/Transplant/Harvest tasks
//! ([`generate_tasks_for_planting`](crate::task_autogen::generate_tasks_for_planting)),
//! which fall on the very same dates as the corresponding planning
//! milestones. To avoid showing the same event twice, we **drop the milestone
//! kinds that already exist as a task** rather than reconciling them at render
//! time — see [`milestone_is_redundant`]. The kept milestones are therefore
//! only the ones with no task equivalent (end-of-harvest, perennial removal,
//! bud-break, flowering, and perennial productive-year harvest windows).
//!
//! The task-category filter (`categories`) applies to tasks only: milestones
//! are a separate family, toggled independently by the UI's master switch.

use crate::calendar_view::{list_events_in_range, CalendarEventKind};
use crate::error::AppResult;
use crate::task_calendar_view::list_task_calendar_rows;
use chrono::NaiveDate;
use pomone_db::Repository;
use pomone_domain::{PlantingSchedule, TaskCategory, TaskId};
use std::collections::{HashMap, HashSet};

/// Which family an entry belongs to — drives styling and interactions in the
/// UI (tasks are editable/draggable; milestones are read-only).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalendarEntryKind {
    Task,
    Milestone,
}

/// One dated entry on the unified calendar. The `task_*` fields are populated
/// for [`CalendarEntryKind::Task`] entries and the `planting_id`/
/// `milestone_kind` fields for [`CalendarEntryKind::Milestone`] ones.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalendarEntry {
    pub date: NaiveDate,
    /// Human-friendly label (`"Tomate · Marmande · Semis"` for a task,
    /// `"Tomate · Marmande"` for a milestone).
    pub label: String,
    pub kind: CalendarEntryKind,

    // --- Task-only ---
    pub task_id: Option<TaskId>,
    /// Resolved hex color of the task's `TaskType` (e.g. `"#3C6E47"`).
    pub task_color: Option<String>,
    pub category: Option<TaskCategory>,
    /// `true` when the task carries a `completed_on` date. Always `false` for
    /// milestones.
    pub completed: bool,

    // --- Milestone-only ---
    /// Stringified `PlantingId` so the UI can route to the planting on click,
    /// matching the standalone calendar's existing navigation.
    pub planting_id: Option<String>,
    pub milestone_kind: Option<CalendarEventKind>,
}

/// Build the merged task + milestone list for `[from, to]`, de-duplicated at
/// the source and sorted by date.
///
/// `categories` filters **tasks** the same way [`list_task_calendar_rows`]
/// does (`None` = all, empty set = none). `show_milestones` is the master
/// toggle for the crop-cycle milestone family: when `false`, no milestones are
/// emitted (the two filter groups are independent).
// The only callers (`pomone-ui` + tests) use the default `RandomState`, so
// threading a `BuildHasher` param would add noise without buying flexibility.
#[allow(clippy::implicit_hasher)]
pub async fn list_calendar_entries(
    repo: &dyn Repository,
    from: NaiveDate,
    to: NaiveDate,
    categories: Option<&HashSet<TaskCategory>>,
    show_milestones: bool,
) -> AppResult<Vec<CalendarEntry>> {
    // Tasks — reuse the existing helper for labels, colors and filtering.
    let task_rows = list_task_calendar_rows(repo, from, to, categories).await?;
    let mut entries: Vec<CalendarEntry> = task_rows
        .into_iter()
        .map(|r| CalendarEntry {
            date: r.planted_on,
            label: r.label,
            kind: CalendarEntryKind::Task,
            task_id: Some(r.task_id),
            task_color: Some(r.color),
            category: Some(r.category),
            completed: r.completed,
            planting_id: None,
            milestone_kind: None,
        })
        .collect();

    // Milestones — master-toggled off entirely, else curated so we never
    // duplicate an auto-generated task. The cycle-vs-perennial distinction
    // matters for `HarvestStart`, so classify each planting once.
    if show_milestones {
        push_milestones(repo, from, to, &mut entries).await?;
    }

    entries.sort_by_key(|e| e.date);
    Ok(entries)
}

/// Append the curated, de-duplicated crop-cycle milestones for `[from, to]`.
async fn push_milestones(
    repo: &dyn Repository,
    from: NaiveDate,
    to: NaiveDate,
    entries: &mut Vec<CalendarEntry>,
) -> AppResult<()> {
    let events = list_events_in_range(repo, from, to).await?;
    let is_cycle: HashMap<String, bool> = repo
        .planting_list()
        .await?
        .iter()
        .map(|p| {
            (
                p.id.to_string(),
                matches!(p.schedule, PlantingSchedule::Cycle { .. }),
            )
        })
        .collect();

    for e in events {
        let planting_is_cycle = is_cycle.get(&e.planting_id).copied().unwrap_or(false);
        if milestone_is_redundant(e.kind, planting_is_cycle) {
            continue;
        }
        entries.push(CalendarEntry {
            date: e.date,
            label: e.label,
            kind: CalendarEntryKind::Milestone,
            task_id: None,
            task_color: None,
            category: None,
            completed: false,
            planting_id: Some(e.planting_id),
            milestone_kind: Some(e.kind),
        });
    }
    Ok(())
}

/// True when a milestone of `kind` is already materialized as an
/// auto-generated task and must be dropped to avoid a duplicate.
///
/// Mirrors [`crate::task_autogen`]'s rules: cycles get Sow/Transplant/Harvest
/// tasks, perennials get a single Transplant at establishment. Everything else
/// has no task and is kept as an informational milestone.
fn milestone_is_redundant(kind: CalendarEventKind, planting_is_cycle: bool) -> bool {
    use CalendarEventKind::{
        BudBreak, Establishment, Flowering, HarvestEnd, HarvestStart, Removal, Sowing,
        Transplanting,
    };
    match kind {
        // Auto-generated as tasks for every schedule (Sow/Transplant for
        // cycles, Transplant-at-establishment for perennials).
        Sowing | Transplanting | Establishment => true,
        // A cycle's first harvest is the `Harvest` task; a perennial's
        // productive-year harvest windows have no task, so keep those.
        HarvestStart => planting_is_cycle,
        // No task equivalent — always kept.
        HarvestEnd | Removal | BudBreak | Flowering => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::create_annual_planting_from_sowing;
    use crate::test_helpers::seed_test_data;
    use pomone_db::{seed_defaults, LocationRepo, SqliteRepository, VarietyRepo};
    use rust_decimal_macros::dec;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    async fn fresh_repo() -> SqliteRepository {
        let repo = SqliteRepository::in_memory().await.unwrap();
        seed_defaults(&repo).await.unwrap();
        repo
    }

    #[test]
    fn redundancy_rule_matches_autogen() {
        use CalendarEventKind::*;
        // Always tasks → redundant for both schedule kinds.
        for &k in &[Sowing, Transplanting, Establishment] {
            assert!(milestone_is_redundant(k, true), "{k:?} cycle");
            assert!(milestone_is_redundant(k, false), "{k:?} perennial");
        }
        // HarvestStart: task for a cycle (redundant), milestone for a perennial.
        assert!(milestone_is_redundant(HarvestStart, true));
        assert!(!milestone_is_redundant(HarvestStart, false));
        // No task equivalent → always kept.
        for &k in &[HarvestEnd, Removal, BudBreak, Flowering] {
            assert!(!milestone_is_redundant(k, true), "{k:?} cycle");
            assert!(!milestone_is_redundant(k, false), "{k:?} perennial");
        }
    }

    #[tokio::test]
    async fn empty_repo_yields_no_entries() {
        let repo = fresh_repo().await;
        seed_test_data(&repo).await.unwrap();
        let entries = list_calendar_entries(&repo, d(2026, 1, 1), d(2026, 12, 31), None, true)
            .await
            .unwrap();
        assert!(entries.is_empty());
    }

    #[tokio::test]
    async fn annual_planting_keeps_tasks_and_only_non_redundant_milestones() {
        let repo = fresh_repo().await;
        seed_test_data(&repo).await.unwrap();

        let varieties = repo.variety_list().await.unwrap();
        let locations = repo.location_list().await.unwrap();
        let bed = locations.iter().find(|l| l.parent_id.is_some()).unwrap();
        create_annual_planting_from_sowing(
            &repo,
            varieties[0].id,
            bed.id,
            d(2026, 3, 1),
            dec!(20),
            100,
            Some("démo".to_owned()),
            None,
        )
        .await
        .unwrap();

        let entries = list_calendar_entries(&repo, d(2026, 1, 1), d(2026, 12, 31), None, true)
            .await
            .unwrap();

        // Auto-generated tasks survive (at least Sow + Harvest for a sowing).
        assert!(entries
            .iter()
            .any(|e| e.category == Some(TaskCategory::Sow)));
        assert!(entries
            .iter()
            .any(|e| e.category == Some(TaskCategory::Harvest)));

        // The only milestones left for a cycle are end-of-harvest: sowing,
        // transplanting and harvest-start are dropped (they exist as tasks).
        let milestones: Vec<_> = entries
            .iter()
            .filter(|e| e.kind == CalendarEntryKind::Milestone)
            .collect();
        assert!(
            !milestones.is_empty(),
            "expected at least the HarvestEnd milestone"
        );
        assert!(milestones
            .iter()
            .all(|m| m.milestone_kind == Some(CalendarEventKind::HarvestEnd)));

        // De-dup invariant: no date carries both a Sow/Harvest task and a
        // redundant milestone of the same nature.
        assert!(!entries
            .iter()
            .any(|e| e.milestone_kind == Some(CalendarEventKind::Sowing)
                || e.milestone_kind == Some(CalendarEventKind::Transplanting)
                || e.milestone_kind == Some(CalendarEventKind::HarvestStart)));

        // Sorted by date.
        assert!(entries.windows(2).all(|w| w[0].date <= w[1].date));
    }

    #[tokio::test]
    async fn category_filter_touches_tasks_only_not_milestones() {
        let repo = fresh_repo().await;
        seed_test_data(&repo).await.unwrap();

        let varieties = repo.variety_list().await.unwrap();
        let locations = repo.location_list().await.unwrap();
        let bed = locations.iter().find(|l| l.parent_id.is_some()).unwrap();
        create_annual_planting_from_sowing(
            &repo,
            varieties[0].id,
            bed.id,
            d(2026, 3, 1),
            dec!(20),
            100,
            None,
            None,
        )
        .await
        .unwrap();

        // Filter tasks down to Sow only.
        let mut only_sow = HashSet::new();
        only_sow.insert(TaskCategory::Sow);
        let entries =
            list_calendar_entries(&repo, d(2026, 1, 1), d(2026, 12, 31), Some(&only_sow), true)
                .await
                .unwrap();

        // Harvest task filtered out…
        assert!(entries
            .iter()
            .all(|e| e.category != Some(TaskCategory::Harvest)));
        assert!(entries
            .iter()
            .any(|e| e.category == Some(TaskCategory::Sow)));
        // …but the HarvestEnd milestone is unaffected by the task filter.
        assert!(entries
            .iter()
            .any(|e| e.milestone_kind == Some(CalendarEventKind::HarvestEnd)));
    }

    #[tokio::test]
    async fn milestone_master_toggle_off_drops_all_milestones() {
        let repo = fresh_repo().await;
        seed_test_data(&repo).await.unwrap();

        let varieties = repo.variety_list().await.unwrap();
        let locations = repo.location_list().await.unwrap();
        let bed = locations.iter().find(|l| l.parent_id.is_some()).unwrap();
        create_annual_planting_from_sowing(
            &repo,
            varieties[0].id,
            bed.id,
            d(2026, 3, 1),
            dec!(20),
            100,
            None,
            None,
        )
        .await
        .unwrap();

        let entries = list_calendar_entries(&repo, d(2026, 1, 1), d(2026, 12, 31), None, false)
            .await
            .unwrap();
        // Tasks remain; not a single milestone is emitted.
        assert!(entries.iter().all(|e| e.kind == CalendarEntryKind::Task));
        assert!(entries
            .iter()
            .any(|e| e.category == Some(TaskCategory::Sow)));
    }
}
