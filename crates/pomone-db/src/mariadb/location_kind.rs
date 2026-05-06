//! `LocationKindRepo` implementation for MariaDB.

use crate::error::{DbError, DbResult};
use crate::mariadb::MariaDbRepository;
use crate::repository::LocationKindRepo;
use async_trait::async_trait;
use pomone_domain::{LocationKind, LocationKindId};
use sqlx::Row;
use uuid::Uuid;

#[async_trait]
impl LocationKindRepo for MariaDbRepository {
    async fn location_kind_get(&self, id: LocationKindId) -> DbResult<Option<LocationKind>> {
        let row = sqlx::query("SELECT id, name, description FROM location_kind WHERE id = ?")
            .bind(id.as_uuid())
            .fetch_optional(&self.pool)
            .await?;
        row.map(row_to_kind).transpose()
    }

    async fn location_kind_list(&self) -> DbResult<Vec<LocationKind>> {
        let rows = sqlx::query("SELECT id, name, description FROM location_kind ORDER BY name")
            .fetch_all(&self.pool)
            .await?;
        rows.into_iter().map(row_to_kind).collect()
    }

    async fn location_kind_create(&self, k: &LocationKind) -> DbResult<()> {
        sqlx::query("INSERT INTO location_kind (id, name, description) VALUES (?, ?, ?)")
            .bind(k.id.as_uuid())
            .bind(&k.name)
            .bind(k.description.as_deref())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn location_kind_update(&self, k: &LocationKind) -> DbResult<()> {
        let result = sqlx::query("UPDATE location_kind SET name = ?, description = ? WHERE id = ?")
            .bind(&k.name)
            .bind(k.description.as_deref())
            .bind(k.id.as_uuid())
            .execute(&self.pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(DbError::NotFound {
                kind: "location_kind",
                id: k.id.to_string(),
            });
        }
        Ok(())
    }

    async fn location_kind_delete(&self, id: LocationKindId) -> DbResult<()> {
        let result = sqlx::query("DELETE FROM location_kind WHERE id = ?")
            .bind(id.as_uuid())
            .execute(&self.pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(DbError::NotFound {
                kind: "location_kind",
                id: id.to_string(),
            });
        }
        Ok(())
    }
}

fn row_to_kind(row: sqlx::mysql::MySqlRow) -> DbResult<LocationKind> {
    let id: Uuid = row.try_get("id")?;
    Ok(LocationKind {
        id: LocationKindId::from(id),
        name: row.try_get("name")?,
        description: row.try_get("description")?,
    })
}
