//! The single write path for settled task state — SQLite (`FactsRepo`).
//!
//! `record_fact` inserts the field event AND applies its projection in ONE
//! transaction. This file is the ONLY place that issues `UPDATE task SET
//! completed_on|skipped_on|skip_reason|skip_note` — enforced by the lint test
//! `crates/pomone-db/tests/facts_write_path.rs` (story 1.2).

use crate::codec::{encode_fact_kind, encode_skip_reason};
use crate::error::{DbError, DbResult};
use crate::repository::{FactOutcome, FactsRepo, TaskProjection};
use crate::sqlite::SqliteRepository;
use async_trait::async_trait;
use pomone_domain::{FieldEvent, FieldEventId};

#[async_trait]
impl FactsRepo for SqliteRepository {
    async fn record_fact(
        &self,
        event: &FieldEvent,
        projection: &TaskProjection,
    ) -> DbResult<FactOutcome> {
        let mut tx = self.pool.begin().await?;

        // 1. Append the event. The conflict-no-op is the idempotency point AND
        //    is race-safe (unlike a separate SELECT-then-INSERT): a replayed id
        //    inserts 0 rows, so we change nothing and roll back on tx drop.
        let inserted = sqlx::query(
            "INSERT INTO field_event \
             (id, kind, target_kind, target_id, occurred_at, recorded_at, payload, corrects) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8) \
             ON CONFLICT(id) DO NOTHING",
        )
        .bind(event.id.as_uuid())
        .bind(encode_fact_kind(event.kind))
        .bind(&event.target_kind)
        .bind(event.target_id)
        .bind(event.occurred_at)
        .bind(event.recorded_at)
        .bind(&event.payload)
        .bind(event.corrects.map(FieldEventId::as_uuid))
        .execute(&mut *tx)
        .await?;
        if inserted.rows_affected() == 0 {
            return Ok(FactOutcome::AlreadyRecorded);
        }

        // 2. Project the settled state onto the task (the guarded writes). A
        //    0-row projection means the target task is gone — reject the whole
        //    fact (tx drop rolls back the just-inserted event) so a fact never
        //    outlives the task it asserts.
        if apply_projection(&mut tx, projection).await? == 0 {
            return Err(DbError::NotFound {
                kind: "task",
                id: projection.task_id().to_string(),
            });
        }

        tx.commit().await?;
        Ok(FactOutcome::Recorded)
    }
}

/// Apply the projection; returns the number of task rows affected (0 = the
/// target task does not exist).
async fn apply_projection(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    projection: &TaskProjection,
) -> DbResult<u64> {
    let result = match projection {
        TaskProjection::Done { task_id, on } => {
            sqlx::query(
                "UPDATE task SET completed_on = ?1, skipped_on = NULL, \
                 skip_reason = NULL, skip_note = NULL WHERE id = ?2",
            )
            .bind(on)
            .bind(task_id.as_uuid())
            .execute(&mut **tx)
            .await?
        }
        TaskProjection::Skipped {
            task_id,
            on,
            reason,
            note,
        } => {
            sqlx::query(
                "UPDATE task SET skipped_on = ?1, skip_reason = ?2, skip_note = ?3, \
                 completed_on = NULL WHERE id = ?4",
            )
            .bind(on)
            .bind(encode_skip_reason(*reason))
            .bind(note.as_deref())
            .bind(task_id.as_uuid())
            .execute(&mut **tx)
            .await?
        }
        TaskProjection::Reopen { task_id } => {
            sqlx::query(
                "UPDATE task SET completed_on = NULL, skipped_on = NULL, \
                 skip_reason = NULL, skip_note = NULL WHERE id = ?1",
            )
            .bind(task_id.as_uuid())
            .execute(&mut **tx)
            .await?
        }
    };
    Ok(result.rows_affected())
}
