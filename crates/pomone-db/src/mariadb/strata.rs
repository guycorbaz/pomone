//! `StrataRepo` implementation for MariaDB.

use crate::error::{DbError, DbResult};
use crate::mariadb::MariaDbRepository;
use crate::repository::StrataRepo;
use async_trait::async_trait;
use pomone_domain::{Strata, StrataId};
use rust_decimal::Decimal;
use sqlx::Row;
use uuid::Uuid;

const COLUMNS: &str = "id, name, description, min_height_m, max_height_m, sort_order";

#[async_trait]
impl StrataRepo for MariaDbRepository {
    async fn strata_get(&self, id: StrataId) -> DbResult<Option<Strata>> {
        let sql = format!("SELECT {COLUMNS} FROM strata WHERE id = ?");
        let row = sqlx::query(&sql)
            .bind(id.as_uuid())
            .fetch_optional(&self.pool)
            .await?;
        row.map(row_to_strata).transpose()
    }

    async fn strata_list(&self) -> DbResult<Vec<Strata>> {
        let sql = format!("SELECT {COLUMNS} FROM strata ORDER BY sort_order, name");
        let rows = sqlx::query(&sql).fetch_all(&self.pool).await?;
        rows.into_iter().map(row_to_strata).collect()
    }

    async fn strata_create(&self, s: &Strata) -> DbResult<()> {
        sqlx::query(
            "INSERT INTO strata (id, name, description, min_height_m, max_height_m, sort_order) \
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(s.id.as_uuid())
        .bind(&s.name)
        .bind(s.description.as_deref())
        .bind(s.min_height_m)
        .bind(s.max_height_m)
        .bind(s.sort_order)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn strata_update(&self, s: &Strata) -> DbResult<()> {
        let result = sqlx::query(
            "UPDATE strata SET name = ?, description = ?, min_height_m = ?, \
             max_height_m = ?, sort_order = ? WHERE id = ?",
        )
        .bind(&s.name)
        .bind(s.description.as_deref())
        .bind(s.min_height_m)
        .bind(s.max_height_m)
        .bind(s.sort_order)
        .bind(s.id.as_uuid())
        .execute(&self.pool)
        .await?;
        if result.rows_affected() == 0 {
            return Err(DbError::NotFound {
                kind: "strata",
                id: s.id.to_string(),
            });
        }
        Ok(())
    }

    async fn strata_delete(&self, id: StrataId) -> DbResult<()> {
        let result = sqlx::query("DELETE FROM strata WHERE id = ?")
            .bind(id.as_uuid())
            .execute(&self.pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(DbError::NotFound {
                kind: "strata",
                id: id.to_string(),
            });
        }
        Ok(())
    }
}

fn row_to_strata(row: sqlx::mysql::MySqlRow) -> DbResult<Strata> {
    let id: Uuid = row.try_get("id")?;
    Ok(Strata {
        id: StrataId::from(id),
        name: row.try_get("name")?,
        description: row.try_get("description")?,
        min_height_m: row.try_get::<Option<Decimal>, _>("min_height_m")?,
        max_height_m: row.try_get::<Option<Decimal>, _>("max_height_m")?,
        sort_order: row.try_get("sort_order")?,
    })
}
