//! Presentation-layer helper for the Tasks screen — a single flat list of
//! every task, newest planned date first (reverse-chronological).
//!
//! Where the Calendar answers "what's planned this month" on a grid, this list
//! is the linear record the user scrolls. It reuses the same decoration as
//! [`crate::task_calendar_view`] (type color + a planting-aware label) and
//! flags overdue rows (pending and past) so the UI can tint their date.

use crate::error::AppResult;
use chrono::NaiveDate;
use pomone_db::Repository;
use std::collections::HashMap;

/// One task ready to render in the flat list. Keeps the date as a preformatted
/// ISO string the UI forwards as-is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgendaRow {
    pub task_id: String,
    /// ISO-8601 (`YYYY-MM-DD`) planned date.
    pub planned_on: String,
    /// Human-friendly label, e.g. `"Tomate · Désherbage"` for a task attached
    /// to a planting, or just the type name when free-standing.
    pub label: String,
    /// Hex color from the `TaskType` (e.g. `"#3C6E47"`), for the row's dot.
    pub color: String,
    /// `true` if the task has a `completed_on` date set.
    pub completed: bool,
    /// `true` for a pending task whose planned date is in the past — the UI
    /// tints the date to flag it.
    pub overdue: bool,
}

/// Build the flat task list relative to `today`, sorted newest planned date
/// first (ties broken by label). Includes every task — pending and completed —
/// so the list doubles as a history. One DB read per lookup table.
pub async fn list_agenda(repo: &dyn Repository, today: NaiveDate) -> AppResult<Vec<AgendaRow>> {
    let tasks = repo.task_list().await?;
    let types = repo.task_type_list().await?;
    let plantings = repo.planting_list().await?;
    let varieties = repo.variety_list().await?;
    let crops = repo.crop_list().await?;

    let types_by_id: HashMap<_, _> = types.iter().map(|t| (t.id, t)).collect();
    let plant_by_id: HashMap<_, _> = plantings.iter().map(|p| (p.id, p)).collect();
    let var_by_id: HashMap<_, _> = varieties.iter().map(|v| (v.id, v)).collect();
    let crop_by_id: HashMap<_, _> = crops.iter().map(|c| (c.id, c)).collect();

    let mut rows: Vec<AgendaRow> = Vec::with_capacity(tasks.len());
    for task in &tasks {
        // Orphan task whose type was deleted — skip rather than crash.
        let Some(tt) = types_by_id.get(&task.task_type_id) else {
            continue;
        };
        let context = task
            .planting_id
            .and_then(|pid| plant_by_id.get(&pid))
            .and_then(|p| {
                let variety = var_by_id.get(&p.variety_id)?;
                let crop_name = crop_by_id
                    .get(&variety.crop_id)
                    .map_or("?", |c| c.name.as_str());
                Some(format!("{crop_name} · {}", variety.name))
            });
        let label = match context {
            Some(planting_label) => format!("{planting_label} · {}", tt.name),
            None => tt.name.clone(),
        };
        let completed = task.completed_on.is_some();
        rows.push(AgendaRow {
            task_id: task.id.to_string(),
            planned_on: task.planned_on.format("%Y-%m-%d").to_string(),
            label,
            color: tt.color.clone(),
            completed,
            overdue: !completed && task.planned_on < today,
        });
    }

    // Newest planned date first; stable tiebreak on label.
    rows.sort_by(|a, b| b.planned_on.cmp(&a.planned_on).then(a.label.cmp(&b.label)));

    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pomone_db::{seed_defaults, SqliteRepository, TaskRepo, TaskTypeRepo};
    use pomone_domain::{Task, TaskCategory};

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    /// Create a free-standing task of the given category, optionally completed.
    async fn add_task(
        repo: &SqliteRepository,
        category: TaskCategory,
        planned_on: NaiveDate,
        completed_on: Option<NaiveDate>,
    ) {
        let tt = repo
            .task_type_list()
            .await
            .unwrap()
            .into_iter()
            .find(|t| t.category == category)
            .unwrap();
        let task = Task::new(
            None,
            None,
            tt.id,
            None,
            None,
            planned_on,
            completed_on,
            None,
            None,
            None,
        );
        repo.task_create(&task).await.unwrap();
    }

    #[tokio::test]
    async fn lists_every_task_newest_first() {
        let repo = SqliteRepository::in_memory().await.unwrap();
        seed_defaults(&repo).await.unwrap();
        let today = d(2026, 5, 20);

        add_task(&repo, TaskCategory::Weeding, d(2026, 5, 10), None).await;
        add_task(&repo, TaskCategory::Harvest, today, None).await;
        add_task(&repo, TaskCategory::Treatment, d(2026, 5, 25), None).await;
        // A completed past task is kept (the list doubles as history).
        add_task(
            &repo,
            TaskCategory::Irrigation,
            d(2026, 5, 8),
            Some(d(2026, 5, 8)),
        )
        .await;

        let rows = list_agenda(&repo, today).await.unwrap();
        let dates: Vec<_> = rows.iter().map(|r| r.planned_on.clone()).collect();
        // Reverse-chronological: newest planned date first.
        assert_eq!(
            dates,
            ["2026-05-25", "2026-05-20", "2026-05-10", "2026-05-08"]
        );
    }

    #[tokio::test]
    async fn overdue_flag_set_only_for_pending_past_tasks() {
        let repo = SqliteRepository::in_memory().await.unwrap();
        seed_defaults(&repo).await.unwrap();
        let today = d(2026, 5, 20);

        add_task(&repo, TaskCategory::Weeding, d(2026, 5, 15), None).await; // pending past
        add_task(
            &repo,
            TaskCategory::Irrigation,
            d(2026, 5, 16),
            Some(d(2026, 5, 16)),
        )
        .await; // completed past
        add_task(&repo, TaskCategory::Treatment, d(2026, 5, 25), None).await; // future

        let rows = list_agenda(&repo, today).await.unwrap();
        let overdue: Vec<_> = rows
            .iter()
            .filter(|r| r.overdue)
            .map(|r| r.planned_on.clone())
            .collect();
        assert_eq!(overdue, ["2026-05-15"]);
    }

    #[tokio::test]
    async fn empty_when_no_tasks() {
        let repo = SqliteRepository::in_memory().await.unwrap();
        seed_defaults(&repo).await.unwrap();
        let rows = list_agenda(&repo, d(2026, 5, 20)).await.unwrap();
        assert!(rows.is_empty());
    }
}
