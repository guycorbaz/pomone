//! The single write path for settled task state — SQLite (`FactsRepo`).
//!
//! `record_fact` inserts the field event AND applies its projection in ONE
//! transaction. This file is the ONLY place that issues `UPDATE task SET
//! completed_on|skipped_on|skip_reason|skip_note` — enforced by the lint test
//! `crates/pomone-db/tests/facts_write_path.rs` (story 1.2).

use crate::codec::{encode_fact_kind, encode_skip_reason};
use crate::error::DbResult;
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

        // Idempotent replay: if this event id is already recorded, change
        // nothing (do not re-project). Dropping the tx rolls back the read-only
        // work.
        let existing: Option<i64> = sqlx::query_scalar("SELECT 1 FROM field_event WHERE id = ?1")
            .bind(event.id.as_uuid())
            .fetch_optional(&mut *tx)
            .await?;
        if existing.is_some() {
            return Ok(FactOutcome::AlreadyRecorded);
        }

        // 1. Append the event (identical shape to `field_event_create`).
        sqlx::query(
            "INSERT INTO field_event \
             (id, kind, target_kind, target_id, occurred_at, recorded_at, payload, corrects) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
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

        // 2. Project the settled state onto the task (the guarded writes).
        apply_projection(&mut tx, projection).await?;

        tx.commit().await?;
        Ok(FactOutcome::Recorded)
    }
}

async fn apply_projection(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    projection: &TaskProjection,
) -> DbResult<()> {
    match projection {
        TaskProjection::Done { task_id, on } => {
            sqlx::query(
                "UPDATE task SET completed_on = ?1, skipped_on = NULL, \
                 skip_reason = NULL, skip_note = NULL WHERE id = ?2",
            )
            .bind(on)
            .bind(task_id.as_uuid())
            .execute(&mut **tx)
            .await?;
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
            .await?;
        }
        TaskProjection::Reopen { task_id } => {
            sqlx::query(
                "UPDATE task SET completed_on = NULL, skipped_on = NULL, \
                 skip_reason = NULL, skip_note = NULL WHERE id = ?1",
            )
            .bind(task_id.as_uuid())
            .execute(&mut **tx)
            .await?;
        }
    }
    Ok(())
}
