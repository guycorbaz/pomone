//! The single write path for settled task state — MariaDB (`FactsRepo`).
//!
//! Mirror of `sqlite/facts.rs` with `?` placeholders. The ONLY place that
//! issues `UPDATE task SET completed_on|skipped_on|skip_reason|skip_note`
//! (lint-enforced, story 1.2).

use crate::codec::{encode_fact_kind, encode_skip_reason};
use crate::error::DbResult;
use crate::mariadb::MariaDbRepository;
use crate::repository::{FactOutcome, FactsRepo, TaskProjection};
use async_trait::async_trait;
use pomone_domain::{FieldEvent, FieldEventId};

#[async_trait]
impl FactsRepo for MariaDbRepository {
    async fn record_fact(
        &self,
        event: &FieldEvent,
        projection: &TaskProjection,
    ) -> DbResult<FactOutcome> {
        let mut tx = self.pool.begin().await?;

        let existing: Option<i64> = sqlx::query_scalar("SELECT 1 FROM field_event WHERE id = ?")
            .bind(event.id.as_uuid())
            .fetch_optional(&mut *tx)
            .await?;
        if existing.is_some() {
            return Ok(FactOutcome::AlreadyRecorded);
        }

        sqlx::query(
            "INSERT INTO field_event \
             (id, kind, target_kind, target_id, occurred_at, recorded_at, payload, corrects) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
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

        apply_projection(&mut tx, projection).await?;

        tx.commit().await?;
        Ok(FactOutcome::Recorded)
    }
}

async fn apply_projection(
    tx: &mut sqlx::Transaction<'_, sqlx::MySql>,
    projection: &TaskProjection,
) -> DbResult<()> {
    match projection {
        TaskProjection::Done { task_id, on } => {
            sqlx::query(
                "UPDATE task SET completed_on = ?, skipped_on = NULL, \
                 skip_reason = NULL, skip_note = NULL WHERE id = ?",
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
                "UPDATE task SET skipped_on = ?, skip_reason = ?, skip_note = ?, \
                 completed_on = NULL WHERE id = ?",
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
                 skip_reason = NULL, skip_note = NULL WHERE id = ?",
            )
            .bind(task_id.as_uuid())
            .execute(&mut **tx)
            .await?;
        }
    }
    Ok(())
}
