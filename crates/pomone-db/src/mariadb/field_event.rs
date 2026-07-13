//! `FieldEventRepo` implementation for MariaDB.

use crate::codec::{decode_fact_kind, encode_fact_kind};
use crate::error::DbResult;
use crate::mariadb::MariaDbRepository;
use crate::repository::FieldEventRepo;
use async_trait::async_trait;
use pomone_domain::{FieldEvent, FieldEventId};
use sqlx::Row;
use uuid::Uuid;

const FIELD_EVENT_COLUMNS: &str =
    "id, kind, target_kind, target_id, occurred_at, recorded_at, payload, corrects";

#[async_trait]
impl FieldEventRepo for MariaDbRepository {
    async fn field_event_create(&self, e: &FieldEvent) -> DbResult<()> {
        // `ON DUPLICATE KEY UPDATE id = id` scopes the no-op to a primary-key
        // conflict only, mirroring SQLite's `ON CONFLICT(id) DO NOTHING`. (An
        // `INSERT IGNORE` would also silence truncation/other errors, diverging
        // from SQLite; this keeps the two backends behaviourally identical.)
        sqlx::query(&format!(
            "INSERT INTO field_event ({FIELD_EVENT_COLUMNS}) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?) \
             ON DUPLICATE KEY UPDATE id = id"
        ))
        .bind(e.id.as_uuid())
        .bind(encode_fact_kind(e.kind))
        .bind(&e.target_kind)
        .bind(e.target_id)
        .bind(e.occurred_at)
        .bind(e.recorded_at)
        .bind(&e.payload)
        .bind(e.corrects.map(FieldEventId::as_uuid))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn field_event_get(&self, id: FieldEventId) -> DbResult<Option<FieldEvent>> {
        let row = sqlx::query(&format!(
            "SELECT {FIELD_EVENT_COLUMNS} FROM field_event WHERE id = ?"
        ))
        .bind(id.as_uuid())
        .fetch_optional(&self.pool)
        .await?;
        row.map(row_to_field_event).transpose()
    }

    async fn field_event_list_for_target(
        &self,
        target_kind: &str,
        target_id: Uuid,
    ) -> DbResult<Vec<FieldEvent>> {
        let rows = sqlx::query(&format!(
            "SELECT {FIELD_EVENT_COLUMNS} FROM field_event \
             WHERE target_kind = ? AND target_id = ? ORDER BY recorded_at, id"
        ))
        .bind(target_kind)
        .bind(target_id)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(row_to_field_event).collect()
    }

    async fn field_event_list_all(&self) -> DbResult<Vec<FieldEvent>> {
        let rows = sqlx::query(&format!(
            "SELECT {FIELD_EVENT_COLUMNS} FROM field_event ORDER BY recorded_at, id"
        ))
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(row_to_field_event).collect()
    }
}

fn row_to_field_event(row: sqlx::mysql::MySqlRow) -> DbResult<FieldEvent> {
    let id: Uuid = row.try_get("id")?;
    let kind: String = row.try_get("kind")?;
    let target_id: Uuid = row.try_get("target_id")?;
    let corrects: Option<Uuid> = row.try_get("corrects")?;
    Ok(FieldEvent {
        id: FieldEventId::from(id),
        kind: decode_fact_kind(&kind)?,
        target_kind: row.try_get("target_kind")?,
        target_id,
        occurred_at: row.try_get("occurred_at")?,
        recorded_at: row.try_get("recorded_at")?,
        payload: row.try_get("payload")?,
        corrects: corrects.map(FieldEventId::from),
    })
}
