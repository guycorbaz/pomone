//! Presentation-layer helper for the Agenda screen — a global "what needs
//! doing" view that buckets tasks into **overdue**, **today**, and
//! **upcoming** (the next few days).
//!
//! Where the Task Calendar answers "what's planned this month", the Agenda
//! answers "what should I act on now". It reuses the same decoration as
//! [`crate::task_calendar_view`] (type color + a planting-aware label) but
//! pre-sorts and groups by urgency so the UI just renders three lists.

use crate::error::AppResult;
use chrono::{Duration, NaiveDate};
use pomone_db::Repository;
use std::collections::HashMap;

/// One task ready to render in an agenda list. Mirrors the calendar row but
/// keeps the date as a preformatted ISO string the UI forwards as-is.
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
}

/// The three urgency buckets the Agenda screen renders, in display order.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Agenda {
    /// Pending tasks planned before `today`, oldest first.
    pub overdue: Vec<AgendaRow>,
    /// Tasks planned exactly on `today` (pending first, then completed).
    pub today: Vec<AgendaRow>,
    /// Pending tasks in `(today, today + upcoming_days]`, soonest first.
    pub upcoming: Vec<AgendaRow>,
}

/// Build the agenda relative to `today`. `upcoming_days` sets the look-ahead
/// horizon for the "upcoming" bucket (e.g. `7` for the coming week).
///
/// Completed tasks only ever appear in the `today` bucket (so the day's done
/// work stays visible); overdue and upcoming list pending work exclusively.
/// One DB read per lookup table — same shape as the calendar view.
pub async fn list_agenda(
    repo: &dyn Repository,
    today: NaiveDate,
    upcoming_days: i64,
) -> AppResult<Agenda> {
    let tasks = repo.task_list().await?;
    let types = repo.task_type_list().await?;
    let plantings = repo.planting_list().await?;
    let varieties = repo.variety_list().await?;
    let crops = repo.crop_list().await?;

    let types_by_id: HashMap<_, _> = types.iter().map(|t| (t.id, t)).collect();
    let plant_by_id: HashMap<_, _> = plantings.iter().map(|p| (p.id, p)).collect();
    let var_by_id: HashMap<_, _> = varieties.iter().map(|v| (v.id, v)).collect();
    let crop_by_id: HashMap<_, _> = crops.iter().map(|c| (c.id, c)).collect();

    let horizon = today + Duration::days(upcoming_days);
    let mut agenda = Agenda::default();

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
        let row = AgendaRow {
            task_id: task.id.to_string(),
            planned_on: task.planned_on.format("%Y-%m-%d").to_string(),
            label,
            color: tt.color.clone(),
            completed: task.completed_on.is_some(),
        };

        let completed = task.completed_on.is_some();
        if task.planned_on == today {
            agenda.today.push(row);
        } else if !completed && task.planned_on < today {
            agenda.overdue.push(row);
        } else if !completed && task.planned_on > today && task.planned_on <= horizon {
            agenda.upcoming.push(row);
        }
    }

    // Overdue + upcoming by ascending date (most pressing first).
    agenda
        .overdue
        .sort_by(|a, b| a.planned_on.cmp(&b.planned_on));
    agenda
        .upcoming
        .sort_by(|a, b| a.planned_on.cmp(&b.planned_on));
    // Today: pending before done, then by label for a stable order.
    agenda
        .today
        .sort_by(|a, b| a.completed.cmp(&b.completed).then(a.label.cmp(&b.label)));

    Ok(agenda)
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
    async fn buckets_split_by_urgency_relative_to_today() {
        let repo = SqliteRepository::in_memory().await.unwrap();
        seed_defaults(&repo).await.unwrap();
        let today = d(2026, 5, 20);

        // Overdue (pending, past).
        add_task(&repo, TaskCategory::Weeding, d(2026, 5, 15), None).await;
        // Past but completed → excluded from overdue.
        add_task(
            &repo,
            TaskCategory::Irrigation,
            d(2026, 5, 16),
            Some(d(2026, 5, 16)),
        )
        .await;
        // Today, pending.
        add_task(&repo, TaskCategory::Harvest, today, None).await;
        // Today, completed → stays in today bucket.
        add_task(&repo, TaskCategory::Sow, today, Some(today)).await;
        // Upcoming within horizon.
        add_task(&repo, TaskCategory::Treatment, d(2026, 5, 25), None).await;
        // Beyond the 7-day horizon → excluded everywhere.
        add_task(&repo, TaskCategory::Tillage, d(2026, 6, 30), None).await;

        let agenda = list_agenda(&repo, today, 7).await.unwrap();

        assert_eq!(agenda.overdue.len(), 1);
        assert_eq!(agenda.overdue[0].planned_on, "2026-05-15");
        assert!(!agenda.overdue[0].completed);

        assert_eq!(agenda.today.len(), 2);
        // Pending sorts before completed.
        assert!(!agenda.today[0].completed);
        assert!(agenda.today[1].completed);

        assert_eq!(agenda.upcoming.len(), 1);
        assert_eq!(agenda.upcoming[0].planned_on, "2026-05-25");
    }

    #[tokio::test]
    async fn overdue_sorted_oldest_first() {
        let repo = SqliteRepository::in_memory().await.unwrap();
        seed_defaults(&repo).await.unwrap();
        let today = d(2026, 5, 20);
        add_task(&repo, TaskCategory::Weeding, d(2026, 5, 18), None).await;
        add_task(&repo, TaskCategory::Weeding, d(2026, 5, 10), None).await;
        add_task(&repo, TaskCategory::Weeding, d(2026, 5, 14), None).await;

        let agenda = list_agenda(&repo, today, 7).await.unwrap();
        let dates: Vec<_> = agenda
            .overdue
            .iter()
            .map(|r| r.planned_on.clone())
            .collect();
        assert_eq!(dates, ["2026-05-10", "2026-05-14", "2026-05-18"]);
    }

    #[tokio::test]
    async fn empty_when_no_tasks() {
        let repo = SqliteRepository::in_memory().await.unwrap();
        seed_defaults(&repo).await.unwrap();
        let agenda = list_agenda(&repo, d(2026, 5, 20), 7).await.unwrap();
        assert!(agenda.overdue.is_empty());
        assert!(agenda.today.is_empty());
        assert!(agenda.upcoming.is_empty());
    }
}
