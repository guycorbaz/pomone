//! `LocationRepo` implementation for MariaDB.

use crate::error::{DbError, DbResult};
use crate::mariadb::MariaDbRepository;
use crate::repository::LocationRepo;
use async_trait::async_trait;
use pomone_domain::{Location, LocationId, LocationKindId};
use rust_decimal::Decimal;
use sqlx::Row;
use uuid::Uuid;

const COLUMNS: &str = "id, parent_id, kind_id, name, length_m, width_m, notes";

#[async_trait]
impl LocationRepo for MariaDbRepository {
    async fn location_get(&self, id: LocationId) -> DbResult<Option<Location>> {
        let sql = format!("SELECT {COLUMNS} FROM location WHERE id = ?");
        let row = sqlx::query(&sql)
            .bind(id.as_uuid())
            .fetch_optional(&self.pool)
            .await?;
        row.map(row_to_location).transpose()
    }

    async fn location_list(&self) -> DbResult<Vec<Location>> {
        let sql = format!("SELECT {COLUMNS} FROM location ORDER BY name");
        let rows = sqlx::query(&sql).fetch_all(&self.pool).await?;
        rows.into_iter().map(row_to_location).collect()
    }

    async fn location_list_roots(&self) -> DbResult<Vec<Location>> {
        let sql = format!("SELECT {COLUMNS} FROM location WHERE parent_id IS NULL ORDER BY name");
        let rows = sqlx::query(&sql).fetch_all(&self.pool).await?;
        rows.into_iter().map(row_to_location).collect()
    }

    async fn location_list_children(&self, parent_id: LocationId) -> DbResult<Vec<Location>> {
        let sql = format!("SELECT {COLUMNS} FROM location WHERE parent_id = ? ORDER BY name");
        let rows = sqlx::query(&sql)
            .bind(parent_id.as_uuid())
            .fetch_all(&self.pool)
            .await?;
        rows.into_iter().map(row_to_location).collect()
    }

    async fn location_create(&self, l: &Location) -> DbResult<()> {
        sqlx::query(
            "INSERT INTO location (id, parent_id, kind_id, name, length_m, width_m, notes) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(l.id.as_uuid())
        .bind(l.parent_id.map(LocationId::as_uuid))
        .bind(l.kind_id.as_uuid())
        .bind(&l.name)
        .bind(l.length_m)
        .bind(l.width_m)
        .bind(l.notes.as_deref())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn location_update(&self, l: &Location) -> DbResult<()> {
        if let Some(parent) = l.parent_id {
            check_no_cycle(&self.pool, l.id, parent).await?;
        }
        let result = sqlx::query(
            "UPDATE location SET parent_id = ?, kind_id = ?, name = ?, \
             length_m = ?, width_m = ?, notes = ? WHERE id = ?",
        )
        .bind(l.parent_id.map(LocationId::as_uuid))
        .bind(l.kind_id.as_uuid())
        .bind(&l.name)
        .bind(l.length_m)
        .bind(l.width_m)
        .bind(l.notes.as_deref())
        .bind(l.id.as_uuid())
        .execute(&self.pool)
        .await?;
        if result.rows_affected() == 0 {
            return Err(DbError::NotFound {
                kind: "location",
                id: l.id.to_string(),
            });
        }
        Ok(())
    }

    async fn location_delete(&self, id: LocationId) -> DbResult<()> {
        let result = sqlx::query("DELETE FROM location WHERE id = ?")
            .bind(id.as_uuid())
            .execute(&self.pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(DbError::NotFound {
                kind: "location",
                id: id.to_string(),
            });
        }
        Ok(())
    }
}

async fn check_no_cycle(
    pool: &sqlx::MySqlPool,
    self_id: LocationId,
    proposed_parent: LocationId,
) -> DbResult<()> {
    let mut current = Some(proposed_parent);
    let mut steps = 0_u32;
    while let Some(id) = current {
        if id == self_id {
            return Err(DbError::HierarchyCycle);
        }
        steps += 1;
        if steps > 10_000 {
            return Err(DbError::HierarchyCycle);
        }
        let row = sqlx::query("SELECT parent_id FROM location WHERE id = ?")
            .bind(id.as_uuid())
            .fetch_optional(pool)
            .await?;
        current = match row {
            Some(r) => r
                .try_get::<Option<Uuid>, _>("parent_id")?
                .map(LocationId::from),
            None => None,
        };
    }
    Ok(())
}

fn row_to_location(row: sqlx::mysql::MySqlRow) -> DbResult<Location> {
    let id: Uuid = row.try_get("id")?;
    let parent_id: Option<Uuid> = row.try_get("parent_id")?;
    let kind_id: Uuid = row.try_get("kind_id")?;
    Ok(Location {
        id: LocationId::from(id),
        parent_id: parent_id.map(LocationId::from),
        kind_id: LocationKindId::from(kind_id),
        name: row.try_get("name")?,
        length_m: row.try_get::<Decimal, _>("length_m")?,
        width_m: row.try_get::<Decimal, _>("width_m")?,
        notes: row.try_get("notes")?,
    })
}
