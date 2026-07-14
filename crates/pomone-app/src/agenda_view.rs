//! Presentation-layer helper for the Tasks screen — a single flat list of
//! every task, newest planned date first (reverse-chronological).
//!
//! Where the Calendar answers "what's planned this month" on a grid, this list
//! is the linear record the user scrolls. It reuses the same decoration as
//! [`crate::task_calendar_view`] (type color + a planting-aware label) and
//! flags overdue rows (pending and past) so the UI can tint their date.

use crate::error::AppResult;
use crate::facts::{record_fact, Fact};
use crate::i18n::I18n;
use chrono::{NaiveDate, NaiveDateTime};
use pomone_db::Repository;
use pomone_domain::field_event::SkipReason;
use pomone_domain::ids::TaskId;
use std::collections::HashMap;

/// One task ready to render in the flat list. Keeps the date as a preformatted
/// ISO string the UI forwards as-is.
// The four flags (completed / skipped / overdue / today) are independent
// render hints the UI reads directly; a sub-struct or enum would only add
// indirection to a flat presentation DTO.
#[allow(clippy::struct_excessive_bools)]
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
    /// `true` if the task was skipped — the UI strikes the row (story 1.5).
    pub skipped: bool,
    /// Localized skip-reason label (e.g. "météo"); empty unless `skipped`.
    pub skip_reason: String,
    /// `true` for a pending task whose planned date is in the past — the UI
    /// shows an "overdue" badge and tints the date. Never true for a settled
    /// (done or skipped) task.
    pub overdue: bool,
    /// `true` for a pending task planned for `today` — the UI shows a "today"
    /// badge. Never true for a settled task.
    pub today: bool,
}

/// Settled (done or skipped) tasks kept in the list (issue #69). Pending tasks
/// are never dropped — an overdue task must stay visible however old it is — but
/// the settled history is capped so years of records can't swamp the screen.
const SETTLED_HISTORY_CAP: usize = 500;

/// Build the flat task list relative to `today`, sorted newest planned date
/// first (ties broken by label). Includes every pending task and the
/// [`SETTLED_HISTORY_CAP`] most recent settled ones, so the list doubles as a
/// (bounded) history. `i18n` localizes the skip-reason badges. One DB read per
/// lookup table.
pub async fn list_agenda(
    repo: &dyn Repository,
    i18n: &I18n,
    today: NaiveDate,
) -> AppResult<Vec<AgendaRow>> {
    let tasks = repo.task_list().await?;
    let types = repo.task_type_list().await?;
    let plantings = repo.planting_list().await?;
    let varieties = repo.variety_list().await?;
    let crops = repo.crop_list().await?;

    let types_by_id: HashMap<_, _> = types.iter().map(|t| (t.id, t)).collect();
    let plant_by_id: HashMap<_, _> = plantings.iter().map(|p| (p.id, p)).collect();
    let var_by_id: HashMap<_, _> = varieties.iter().map(|v| (v.id, v)).collect();
    let crop_by_id: HashMap<_, _> = crops.iter().map(|c| (c.id, c)).collect();

    // Each entry pairs the render row with the date it was *settled* on (done
    // or skipped), or `None` while pending — used to cap the history by
    // recency-of-settling, not planned date (see the cap below).
    let mut rows: Vec<(AgendaRow, Option<NaiveDate>)> = Vec::with_capacity(tasks.len());
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
        let skipped = task.skipped_on.is_some();
        let settled = completed || skipped;
        let skip_reason = if skipped {
            task.skip_reason
                .map(|r| i18n.t(&format!("skip-reason-{}", r.as_str())))
                .unwrap_or_default()
        } else {
            String::new()
        };
        // The settling date is what a skip/done actually stamps; `None` while
        // pending. `completed_on` wins if somehow both are set (defensive).
        let settled_on = task.completed_on.or(task.skipped_on);
        rows.push((
            AgendaRow {
                task_id: task.id.to_string(),
                planned_on: task.planned_on.format("%Y-%m-%d").to_string(),
                label,
                color: tt.color.clone(),
                completed,
                skipped,
                skip_reason,
                overdue: !settled && task.planned_on < today,
                today: !settled && task.planned_on == today,
            },
            settled_on,
        ));
    }

    // Cap the settled (done + skipped) history by *recency of settling*, not by
    // planned date: skipping a long-overdue task (an old planned date, settled
    // today) must keep it visible and correctable, never drop it under the cap
    // (issue #69 / story 1.5). Pending rows are always kept.
    let mut settled: Vec<(usize, NaiveDate)> = rows
        .iter()
        .enumerate()
        .filter_map(|(i, (_, on))| on.map(|d| (i, d)))
        .collect();
    // Newest settled first; index tiebreak keeps the drop deterministic.
    settled.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let dropped: std::collections::HashSet<usize> = settled
        .into_iter()
        .skip(SETTLED_HISTORY_CAP)
        .map(|(i, _)| i)
        .collect();

    let mut out: Vec<AgendaRow> = rows
        .into_iter()
        .enumerate()
        .filter(|(i, _)| !dropped.contains(i))
        .map(|(_, (row, _))| row)
        .collect();

    // Newest planned date first; stable tiebreak on label.
    out.sort_by(|a, b| b.planned_on.cmp(&a.planned_on).then(a.label.cmp(&b.label)));

    Ok(out)
}

/// Mark a task done, on `on`, from the tasks screen — records a `Done` fact
/// through the single write path (story 1.2). `on` is the agronomic date the
/// gesture is about; `recorded_at` is injected by the caller (no clock read
/// below the UI/CLI, story 1.3).
pub async fn mark_task_done(
    repo: &dyn Repository,
    task_id: TaskId,
    on: NaiveDate,
    recorded_at: NaiveDateTime,
) -> AppResult<()> {
    record_fact(repo, Fact::Done { task_id, on }, recorded_at).await?;
    Ok(())
}

/// Skip a task from the tasks screen — records a `Skipped` fact with a
/// closed-set `reason` and an optional free-text `note`. A skip is a deliberate
/// decision, never a silent drop: it strikes the row, keeps a reason badge, and
/// leaves the task out of future-facing debt (`is_overdue` never fires on it,
/// story 1.5). Same single write path as every other settle gesture.
pub async fn skip_task(
    repo: &dyn Repository,
    task_id: TaskId,
    on: NaiveDate,
    reason: SkipReason,
    note: Option<String>,
    recorded_at: NaiveDateTime,
) -> AppResult<()> {
    record_fact(
        repo,
        Fact::Skipped {
            task_id,
            on,
            reason,
            note,
        },
        recorded_at,
    )
    .await?;
    Ok(())
}

/// Reopen (correct) a settled task from the tasks screen — records a `Reopened`
/// fact that clears its done/skipped state. Any settled state is correctable
/// from this view alone, and the correction is explicit (a fact), never a
/// silent edit (story 1.5).
pub async fn reopen_task(
    repo: &dyn Repository,
    task_id: TaskId,
    on: NaiveDate,
    recorded_at: NaiveDateTime,
) -> AppResult<()> {
    record_fact(repo, Fact::Reopened { task_id, on }, recorded_at).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::Lang;
    use pomone_db::{seed_defaults, SqliteRepository, TaskRepo, TaskTypeRepo};
    use pomone_domain::field_event::SkipReason;
    use pomone_domain::{Task, TaskCategory};

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    fn i18n() -> I18n {
        I18n::new(Lang::Fr).unwrap()
    }

    fn at(y: i32, m: u32, day: u32) -> NaiveDateTime {
        d(y, m, day).and_hms_opt(9, 0, 0).unwrap()
    }

    /// Create a free-standing task of the given category, optionally completed.
    /// Returns the new task's id so callers can settle it afterwards.
    async fn add_task(
        repo: &SqliteRepository,
        category: TaskCategory,
        planned_on: NaiveDate,
        completed_on: Option<NaiveDate>,
    ) -> pomone_domain::ids::TaskId {
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
        let id = task.id;
        repo.task_create(&task).await.unwrap();
        id
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

        let rows = list_agenda(&repo, &i18n(), today).await.unwrap();
        let dates: Vec<_> = rows.iter().map(|r| r.planned_on.clone()).collect();
        // Reverse-chronological: newest planned date first.
        assert_eq!(
            dates,
            ["2026-05-25", "2026-05-20", "2026-05-10", "2026-05-08"]
        );
    }

    #[tokio::test]
    async fn overdue_and_today_flags_only_for_pending_tasks() {
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
        add_task(&repo, TaskCategory::Harvest, today, None).await; // pending today
        add_task(&repo, TaskCategory::Sow, today, Some(today)).await; // completed today
        add_task(&repo, TaskCategory::Treatment, d(2026, 5, 25), None).await; // future

        let rows = list_agenda(&repo, &i18n(), today).await.unwrap();

        let overdue: Vec<_> = rows
            .iter()
            .filter(|r| r.overdue)
            .map(|r| r.planned_on.clone())
            .collect();
        assert_eq!(overdue, ["2026-05-15"]);

        // Exactly one "today" flag: the pending today task, not the completed one.
        assert_eq!(rows.iter().filter(|r| r.today).count(), 1);
        assert!(rows.iter().all(|r| !(r.today && r.overdue))); // never both
    }

    #[tokio::test]
    async fn completed_history_is_capped_but_pending_never_dropped() {
        let repo = SqliteRepository::in_memory().await.unwrap();
        seed_defaults(&repo).await.unwrap();
        let today = d(2026, 5, 20);

        // A very old pending task — must survive the cap however old.
        add_task(&repo, TaskCategory::Weeding, d(2020, 1, 1), None).await;
        // One more completed task than the cap allows.
        let base = d(2024, 1, 1);
        for i in 0..=SETTLED_HISTORY_CAP {
            let day = base + chrono::Duration::days(i64::try_from(i).unwrap());
            add_task(&repo, TaskCategory::Irrigation, day, Some(day)).await;
        }

        let rows = list_agenda(&repo, &i18n(), today).await.unwrap();
        let completed = rows.iter().filter(|r| r.completed).count();
        assert_eq!(completed, SETTLED_HISTORY_CAP);
        // The dropped one is the oldest completed; the old pending stays.
        assert!(rows.iter().any(|r| r.planned_on == "2020-01-01"));
        assert!(!rows
            .iter()
            .any(|r| r.completed && r.planned_on == "2024-01-01"));
    }

    #[tokio::test]
    async fn empty_when_no_tasks() {
        let repo = SqliteRepository::in_memory().await.unwrap();
        seed_defaults(&repo).await.unwrap();
        let rows = list_agenda(&repo, &i18n(), d(2026, 5, 20)).await.unwrap();
        assert!(rows.is_empty());
    }

    #[tokio::test]
    async fn skipped_task_is_flagged_reasoned_and_never_overdue() {
        let repo = SqliteRepository::in_memory().await.unwrap();
        seed_defaults(&repo).await.unwrap();
        let today = d(2026, 5, 20);

        // A past task, then skipped — must show as skipped with a localized
        // reason and must NOT be flagged overdue (story 1.5).
        let id = add_task(&repo, TaskCategory::Weeding, d(2026, 5, 10), None).await;
        skip_task(
            &repo,
            id,
            d(2026, 5, 15),
            SkipReason::Weather,
            None,
            at(2026, 5, 15),
        )
        .await
        .unwrap();

        let rows = list_agenda(&repo, &i18n(), today).await.unwrap();
        let row = rows.iter().find(|r| r.task_id == id.to_string()).unwrap();
        assert!(row.skipped);
        assert!(!row.completed);
        assert!(!row.overdue, "a skipped task is a decision, not a debt");
        assert!(!row.today);
        // Localized (fr) skip-reason label — resolved, not the raw key.
        assert!(!row.skip_reason.is_empty());
        assert_ne!(row.skip_reason, "skip-reason-weather");
    }

    /// Regression (review 1.5): skipping a long-overdue task must keep it
    /// visible for correction even when the settled history is already full —
    /// the cap is by recency of settling, not by planned date.
    #[tokio::test]
    async fn freshly_skipped_old_task_survives_the_settled_cap() {
        let repo = SqliteRepository::in_memory().await.unwrap();
        seed_defaults(&repo).await.unwrap();
        let today = d(2026, 5, 20);

        // Fill the cap with recent completed tasks (all settled in 2024).
        let base = d(2024, 1, 1);
        for i in 0..SETTLED_HISTORY_CAP {
            let day = base + chrono::Duration::days(i64::try_from(i).unwrap());
            add_task(&repo, TaskCategory::Irrigation, day, Some(day)).await;
        }
        // A long-overdue task (old planned date), skipped *today*.
        let id = add_task(&repo, TaskCategory::Weeding, d(2020, 1, 1), None).await;
        skip_task(&repo, id, today, SkipReason::NoTime, None, at(2026, 5, 20))
            .await
            .unwrap();

        let rows = list_agenda(&repo, &i18n(), today).await.unwrap();
        let row = rows.iter().find(|r| r.task_id == id.to_string());
        assert!(
            row.is_some_and(|r| r.skipped),
            "a freshly-skipped task must stay visible and correctable"
        );
    }

    /// «Corriger» reopens any settled state — done *or* skipped — clearing it so
    /// the task is pending again and re-enters the future-facing lists (AC-b).
    #[tokio::test]
    async fn reopen_clears_both_done_and_skipped_states() {
        let repo = SqliteRepository::in_memory().await.unwrap();
        seed_defaults(&repo).await.unwrap();
        let today = d(2026, 5, 20);

        // (1) a skipped past task, then corrected → pending & overdue again.
        let skipped = add_task(&repo, TaskCategory::Weeding, d(2026, 5, 10), None).await;
        skip_task(
            &repo,
            skipped,
            d(2026, 5, 15),
            SkipReason::NoTime,
            None,
            at(2026, 5, 15),
        )
        .await
        .unwrap();
        reopen_task(&repo, skipped, today, at(2026, 5, 20))
            .await
            .unwrap();

        // (2) a done task, then corrected → pending again.
        let done = add_task(&repo, TaskCategory::Harvest, d(2026, 5, 12), None).await;
        mark_task_done(&repo, done, d(2026, 5, 12), at(2026, 5, 12))
            .await
            .unwrap();
        reopen_task(&repo, done, today, at(2026, 5, 20))
            .await
            .unwrap();

        let rows = list_agenda(&repo, &i18n(), today).await.unwrap();
        let s = rows
            .iter()
            .find(|r| r.task_id == skipped.to_string())
            .unwrap();
        assert!(!s.skipped && !s.completed, "skip cleared by correction");
        assert!(s.overdue, "reopened past task is a debt again");
        assert!(s.skip_reason.is_empty());
        let dn = rows.iter().find(|r| r.task_id == done.to_string()).unwrap();
        assert!(!dn.completed, "done cleared by correction");
    }
}
