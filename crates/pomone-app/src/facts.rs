//! The single write path for field facts (story 1.2).
//!
//! Every settled-state change — a task done, skipped, or reopened — flows
//! through [`record_fact`], which appends the field event AND projects its
//! state onto the task in ONE transaction (via [`pomone_db::FactsRepo`]). So
//! "marked" always means "persisted", and replaying the same event is a
//! harmless no-op.
//!
//! No code below the UI/CLI reads the clock: `recorded_at` is injected by the
//! caller (story 1.3 tightens this).

use crate::error::AppResult;
use chrono::{NaiveDate, NaiveDateTime};
use pomone_db::{FactOutcome, Repository};
use pomone_domain::{skip_payload, FactKind, FieldEvent, FieldEventId, SkipReason, TaskId};

/// The `target_kind` carried by every task fact.
pub const TARGET_TASK: &str = "task";

/// A gesture to record through the single write path.
#[derive(Debug, Clone)]
pub enum Fact {
    /// Mark a task done, on `on`.
    Done { task_id: TaskId, on: NaiveDate },
    /// Skip a task with a reason (+ optional note), on `on`.
    Skipped {
        task_id: TaskId,
        on: NaiveDate,
        reason: SkipReason,
        note: Option<String>,
    },
    /// Reopen a settled task, on `on` — a correction that clears its state.
    Reopened { task_id: TaskId, on: NaiveDate },
}

/// Append the fact's event and project its state, atomically. `recorded_at` is
/// caller-injected. Returns the appended [`FieldEvent`] and whether it was
/// newly recorded (idempotent on a replayed event id).
pub async fn record_fact(
    repo: &dyn Repository,
    fact: Fact,
    recorded_at: NaiveDateTime,
) -> AppResult<(FieldEvent, FactOutcome)> {
    let (event, projection) = match fact {
        Fact::Done { task_id, on } => (
            FieldEvent::new(
                FactKind::TaskDone,
                TARGET_TASK,
                task_id.as_uuid(),
                on,
                recorded_at,
                "{}",
                None,
            )?,
            pomone_db::TaskProjection::Done { task_id, on },
        ),
        Fact::Skipped {
            task_id,
            on,
            reason,
            note,
        } => (
            FieldEvent::new(
                FactKind::TaskSkipped,
                TARGET_TASK,
                task_id.as_uuid(),
                on,
                recorded_at,
                skip_payload(reason, note.as_deref()),
                None,
            )?,
            pomone_db::TaskProjection::Skipped {
                task_id,
                on,
                reason,
                note,
            },
        ),
        Fact::Reopened { task_id, on } => {
            let corrects = latest_settling_event(repo, task_id).await?;
            (
                FieldEvent::new(
                    FactKind::TaskReopened,
                    TARGET_TASK,
                    task_id.as_uuid(),
                    on,
                    recorded_at,
                    "{}",
                    corrects,
                )?,
                pomone_db::TaskProjection::Reopen { task_id },
            )
        }
    };
    let outcome = repo.record_fact(&event, &projection).await?;
    Ok((event, outcome))
}

/// The id of the most recent settling event (done/skipped) for a task, if any —
/// what a reopen correction points back at.
async fn latest_settling_event(
    repo: &dyn Repository,
    task_id: TaskId,
) -> AppResult<Option<FieldEventId>> {
    let events = repo
        .field_event_list_for_target(TARGET_TASK, task_id.as_uuid())
        .await?;
    Ok(events
        .iter()
        .rev()
        .find(|e| matches!(e.kind, FactKind::TaskDone | FactKind::TaskSkipped))
        .map(|e| e.id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pomone_db::{FactsRepo, FieldEventRepo, SqliteRepository, TaskRepo, TaskTypeRepo};
    use pomone_domain::Task;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    fn dt() -> NaiveDateTime {
        d(2026, 3, 2).and_hms_opt(9, 0, 0).unwrap()
    }

    async fn setup_task() -> (SqliteRepository, TaskId) {
        let repo = SqliteRepository::in_memory().await.unwrap();
        pomone_db::seed_defaults(&repo).await.unwrap();
        let task_type = repo.task_type_list().await.unwrap()[0].id;
        let task = Task::new(
            None,
            None,
            task_type,
            None,
            None,
            d(2026, 3, 1),
            None,
            None,
            None,
            None,
        );
        repo.task_create(&task).await.unwrap();
        (repo, task.id)
    }

    #[tokio::test]
    async fn done_projects_completion_and_appends_one_event() {
        let (repo, task_id) = setup_task().await;
        let (event, outcome) = record_fact(
            &repo,
            Fact::Done {
                task_id,
                on: d(2026, 3, 2),
            },
            dt(),
        )
        .await
        .unwrap();
        assert_eq!(outcome, FactOutcome::Recorded);
        let task = repo.task_get(task_id).await.unwrap().unwrap();
        assert_eq!(task.completed_on, Some(d(2026, 3, 2)));
        assert!(task.skip_reason.is_none());
        // Exactly one event on the task.
        assert_eq!(
            repo.field_event_list_for_target(TARGET_TASK, task_id.as_uuid())
                .await
                .unwrap()
                .len(),
            1
        );
        assert_eq!(event.kind, FactKind::TaskDone);
    }

    #[tokio::test]
    async fn skip_projects_reason_and_note() {
        let (repo, task_id) = setup_task().await;
        record_fact(
            &repo,
            Fact::Skipped {
                task_id,
                on: d(2026, 3, 2),
                reason: SkipReason::Weather,
                note: Some("trop humide".into()),
            },
            dt(),
        )
        .await
        .unwrap();
        let task = repo.task_get(task_id).await.unwrap().unwrap();
        assert_eq!(task.skipped_on, Some(d(2026, 3, 2)));
        assert_eq!(task.skip_reason, Some(SkipReason::Weather));
        assert_eq!(task.skip_note.as_deref(), Some("trop humide"));
        assert!(task.completed_on.is_none());
    }

    #[tokio::test]
    async fn reopen_clears_state_and_points_at_the_settling_event() {
        let (repo, task_id) = setup_task().await;
        let (done, _) = record_fact(
            &repo,
            Fact::Done {
                task_id,
                on: d(2026, 3, 2),
            },
            dt(),
        )
        .await
        .unwrap();
        let (reopen, _) = record_fact(
            &repo,
            Fact::Reopened {
                task_id,
                on: d(2026, 3, 3),
            },
            dt(),
        )
        .await
        .unwrap();
        assert_eq!(reopen.corrects, Some(done.id));
        let task = repo.task_get(task_id).await.unwrap().unwrap();
        assert!(task.completed_on.is_none(), "reopen clears completion");
        // The original done event is untouched (append-only): 2 events total.
        assert_eq!(
            repo.field_event_list_for_target(TARGET_TASK, task_id.as_uuid())
                .await
                .unwrap()
                .len(),
            2
        );
    }

    #[tokio::test]
    async fn replaying_the_same_event_is_a_no_op() {
        let (repo, task_id) = setup_task().await;
        let event = FieldEvent::new(
            FactKind::TaskDone,
            TARGET_TASK,
            task_id.as_uuid(),
            d(2026, 3, 2),
            dt(),
            "{}",
            None,
        )
        .unwrap();
        let proj = pomone_db::TaskProjection::Done {
            task_id,
            on: d(2026, 3, 2),
        };
        assert_eq!(
            repo.record_fact(&event, &proj).await.unwrap(),
            FactOutcome::Recorded
        );
        // Same id again → AlreadyRecorded, nothing changes.
        assert_eq!(
            repo.record_fact(&event, &proj).await.unwrap(),
            FactOutcome::AlreadyRecorded
        );
        assert_eq!(
            repo.field_event_list_for_target(TARGET_TASK, task_id.as_uuid())
                .await
                .unwrap()
                .len(),
            1
        );
    }
}
