//! Presentation-layer helpers for the Task Calendar screen.
//!
//! Produces a flat list of `TaskCalendarRow`s for a date range. Each row is a
//! single task ready to render as a pill in the corresponding day cell, with
//! the resolved type color and a human-friendly label that includes the
//! variety (and location) when the task is anchored to a planting.

use crate::error::AppResult;
use chrono::NaiveDate;
use pomone_db::Repository;
use pomone_domain::{TaskCategory, TaskId};
use std::collections::{HashMap, HashSet};

/// One task ready to render on a calendar day cell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskCalendarRow {
    pub task_id: TaskId,
    pub planted_on: NaiveDate,
    /// Human-friendly label, e.g. `"Tomate Marmande · Semis"` for a Sow task
    /// attached to a planting, or just `"Semis"` when neither planting nor
    /// location is set.
    pub label: String,
    /// Localized name of the task type (`"Semis"`, `"Récolte"`…).
    pub type_name: String,
    /// Stable enum so the UI can group/filter by category later without
    /// reparsing the name.
    pub category: TaskCategory,
    /// Hex color from the `TaskType` (e.g. `"#3C6E47"`).
    pub color: String,
    /// `true` if the task has a `completed_on` date set.
    pub completed: bool,
}

/// Fetch all tasks whose `planned_on` falls inside `[from, to]` and decorate
/// them with their type metadata and a human-friendly label.
///
/// When `categories` is `Some`, only tasks whose type's category is in the
/// set are returned — used by the calendar's per-category filter chips.
/// `None` returns everything (no filter); an empty set returns nothing
/// (the user explicitly deselected all categories).
///
/// One DB read per lookup table — same shape as `list_plantings` — which is
/// fine at Pomone's scale.
// `implicit_hasher` would have us thread a `BuildHasher` type param through
// the signature; the only callers (`pomone-ui` + tests) use the default
// `RandomState`, so the lint adds noise without buying flexibility.
#[allow(clippy::implicit_hasher)]
pub async fn list_task_calendar_rows(
    repo: &dyn Repository,
    from: NaiveDate,
    to: NaiveDate,
    categories: Option<&HashSet<TaskCategory>>,
) -> AppResult<Vec<TaskCalendarRow>> {
    let tasks = repo.task_list_in_range(from, to).await?;
    let types = repo.task_type_list().await?;
    let plantings = repo.planting_list().await?;
    let varieties = repo.variety_list().await?;
    let crops = repo.crop_list().await?;

    let types_by_id: HashMap<_, _> = types.iter().map(|t| (t.id, t)).collect();
    let plant_by_id: HashMap<_, _> = plantings.iter().map(|p| (p.id, p)).collect();
    let var_by_id: HashMap<_, _> = varieties.iter().map(|v| (v.id, v)).collect();
    let crop_by_id: HashMap<_, _> = crops.iter().map(|c| (c.id, c)).collect();

    let mut rows: Vec<TaskCalendarRow> = Vec::with_capacity(tasks.len());
    for task in tasks {
        let Some(tt) = types_by_id.get(&task.task_type_id) else {
            // Orphan task whose type was deleted — skip rather than crash.
            // (FK is RESTRICT so this shouldn't happen, but defensive.)
            continue;
        };
        if let Some(allowed) = categories {
            if !allowed.contains(&tt.category) {
                continue;
            }
        }
        let context = task
            .planting_id
            .and_then(|pid| plant_by_id.get(&pid))
            .and_then(|p| {
                let variety = var_by_id.get(&p.variety_id)?;
                let crop = crop_by_id.get(&variety.crop_id);
                let crop_name = crop.map_or("?", |c| c.name.as_str());
                Some(format!("{crop_name} · {}", variety.name))
            });
        let label = match context {
            Some(planting_label) => format!("{planting_label} · {}", tt.name),
            None => tt.name.clone(),
        };
        rows.push(TaskCalendarRow {
            task_id: task.id,
            planted_on: task.planned_on,
            label,
            type_name: tt.name.clone(),
            category: tt.category,
            color: tt.color.clone(),
            completed: task.completed_on.is_some(),
        });
    }
    rows.sort_by_key(|r| r.planted_on);
    Ok(rows)
}

/// Toggle a task's completion state: if it has no `completed_on`, set it to
/// `on`; if it already has one, clear it. Returns the new state (`true` =
/// now completed). Useful for the calendar's click-to-complete.
pub async fn toggle_task_completion(
    repo: &dyn Repository,
    task_id: TaskId,
    on: NaiveDate,
) -> AppResult<bool> {
    let task = repo
        .task_get(task_id)
        .await?
        .ok_or_else(|| crate::error::AppError::NotFound {
            kind: "task",
            id: task_id.to_string(),
        })?;
    let now_completed = task.completed_on.is_none();
    let updated = if now_completed {
        task.mark_completed(on)
    } else {
        pomone_domain::Task {
            completed_on: None,
            ..task
        }
    };
    repo.task_update(&updated).await?;
    Ok(now_completed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pomone_db::{seed_defaults, SqliteRepository};

    #[tokio::test]
    async fn empty_range_returns_no_rows() {
        let repo = SqliteRepository::in_memory().await.unwrap();
        seed_defaults(&repo).await.unwrap();
        let rows = list_task_calendar_rows(
            &repo,
            NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2026, 12, 31).unwrap(),
            None,
        )
        .await
        .unwrap();
        assert!(rows.is_empty());
    }

    #[tokio::test]
    async fn rows_carry_type_color_and_category() {
        use pomone_db::{LocationKindRepo, LocationRepo, TaskRepo, TaskTypeRepo};
        use pomone_domain::{LocationKind, Task};
        use rust_decimal_macros::dec;
        let repo = SqliteRepository::in_memory().await.unwrap();
        seed_defaults(&repo).await.unwrap();
        let k = LocationKind::new("Planche", None).unwrap();
        repo.location_kind_create(&k).await.unwrap();
        let loc =
            pomone_domain::Location::new(k.id, "Planche A", dec!(10), dec!(1), None, None).unwrap();
        repo.location_create(&loc).await.unwrap();

        let types = repo.task_type_list().await.unwrap();
        let weeding = types
            .iter()
            .find(|t| t.category == TaskCategory::Weeding)
            .unwrap();
        let task = Task::new(
            None,
            Some(loc.id),
            weeding.id,
            None,
            None,
            NaiveDate::from_ymd_opt(2026, 5, 15).unwrap(),
            None,
            Some(30),
            None,
            None,
        );
        repo.task_create(&task).await.unwrap();

        let rows = list_task_calendar_rows(
            &repo,
            NaiveDate::from_ymd_opt(2026, 5, 1).unwrap(),
            NaiveDate::from_ymd_opt(2026, 5, 31).unwrap(),
            None,
        )
        .await
        .unwrap();
        assert_eq!(rows.len(), 1);
        let row = &rows[0];
        assert_eq!(row.category, TaskCategory::Weeding);
        assert_eq!(row.type_name, "Désherbage");
        assert!(row.color.starts_with('#'));
        // No planting → label falls back to the type name.
        assert_eq!(row.label, "Désherbage");
        assert!(!row.completed);
    }

    #[tokio::test]
    async fn category_filter_keeps_only_matching_rows() {
        use pomone_db::{TaskRepo, TaskTypeRepo};
        use pomone_domain::Task;
        let repo = SqliteRepository::in_memory().await.unwrap();
        seed_defaults(&repo).await.unwrap();
        let types = repo.task_type_list().await.unwrap();
        let sow = types
            .iter()
            .find(|t| t.category == TaskCategory::Sow)
            .unwrap();
        let harvest = types
            .iter()
            .find(|t| t.category == TaskCategory::Harvest)
            .unwrap();
        let weeding = types
            .iter()
            .find(|t| t.category == TaskCategory::Weeding)
            .unwrap();
        for tt in [sow, harvest, weeding] {
            let t = Task::new(
                None,
                None,
                tt.id,
                None,
                None,
                NaiveDate::from_ymd_opt(2026, 5, 10).unwrap(),
                None,
                None,
                None,
                None,
            );
            repo.task_create(&t).await.unwrap();
        }

        let from = NaiveDate::from_ymd_opt(2026, 5, 1).unwrap();
        let to = NaiveDate::from_ymd_opt(2026, 5, 31).unwrap();

        // None → all rows.
        let all = list_task_calendar_rows(&repo, from, to, None)
            .await
            .unwrap();
        assert_eq!(all.len(), 3);

        // Filter to harvest+weeding.
        let mut wanted = HashSet::new();
        wanted.insert(TaskCategory::Harvest);
        wanted.insert(TaskCategory::Weeding);
        let filtered = list_task_calendar_rows(&repo, from, to, Some(&wanted))
            .await
            .unwrap();
        assert_eq!(filtered.len(), 2);
        assert!(filtered
            .iter()
            .all(|r| r.category == TaskCategory::Harvest || r.category == TaskCategory::Weeding));

        // Empty set → nothing.
        let none = list_task_calendar_rows(&repo, from, to, Some(&HashSet::new()))
            .await
            .unwrap();
        assert!(none.is_empty());
    }

    #[tokio::test]
    async fn toggle_marks_then_clears_completion() {
        use pomone_db::{TaskRepo, TaskTypeRepo};
        use pomone_domain::Task;
        let repo = SqliteRepository::in_memory().await.unwrap();
        seed_defaults(&repo).await.unwrap();
        let tt = repo
            .task_type_list()
            .await
            .unwrap()
            .into_iter()
            .next()
            .unwrap();
        let task = Task::new(
            None,
            None,
            tt.id,
            None,
            None,
            NaiveDate::from_ymd_opt(2026, 5, 10).unwrap(),
            None,
            None,
            None,
            None,
        );
        repo.task_create(&task).await.unwrap();
        let now = NaiveDate::from_ymd_opt(2026, 5, 11).unwrap();
        assert!(toggle_task_completion(&repo, task.id, now).await.unwrap());
        let got = repo.task_get(task.id).await.unwrap().unwrap();
        assert_eq!(got.completed_on, Some(now));
        // Toggle back.
        assert!(!toggle_task_completion(&repo, task.id, now).await.unwrap());
        let got2 = repo.task_get(task.id).await.unwrap().unwrap();
        assert_eq!(got2.completed_on, None);
    }
}
