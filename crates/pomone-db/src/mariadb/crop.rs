//! `CropRepo` implementation for MariaDB.

use crate::codec::{decode_lifespan, decode_pruning, encode_lifespan, encode_pruning};
use crate::error::{DbError, DbResult};
use crate::mariadb::MariaDbRepository;
use crate::repository::CropRepo;
use async_trait::async_trait;
use pomone_domain::{Crop, CropId, FamilyId, StrataId};
use sqlx::Row;
use uuid::Uuid;

const COLUMNS: &str = "id, family_id, strata_id, name, latin_name, pruning_season, \
                       lifespan_kind, lifespan_years, productive_pattern, years_to_first_yield";

#[async_trait]
impl CropRepo for MariaDbRepository {
    async fn crop_get(&self, id: CropId) -> DbResult<Option<Crop>> {
        let sql = format!("SELECT {COLUMNS} FROM crop WHERE id = ?");
        let row = sqlx::query(&sql)
            .bind(id.as_uuid())
            .fetch_optional(&self.pool)
            .await?;
        row.map(row_to_crop).transpose()
    }

    async fn crop_list(&self) -> DbResult<Vec<Crop>> {
        let sql = format!("SELECT {COLUMNS} FROM crop ORDER BY name");
        let rows = sqlx::query(&sql).fetch_all(&self.pool).await?;
        rows.into_iter().map(row_to_crop).collect()
    }

    async fn crop_create(&self, c: &Crop) -> DbResult<()> {
        let life = encode_lifespan(c.lifespan);
        sqlx::query(
            "INSERT INTO crop (id, family_id, strata_id, name, latin_name, pruning_season, \
             lifespan_kind, lifespan_years, productive_pattern, years_to_first_yield) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(c.id.as_uuid())
        .bind(c.family_id.as_uuid())
        .bind(c.strata_id.as_uuid())
        .bind(&c.name)
        .bind(c.latin_name.as_deref())
        .bind(encode_pruning(c.pruning_season))
        .bind(life.kind)
        .bind(life.lifespan_years)
        .bind(life.pattern)
        .bind(life.years_to_first_yield)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn crop_update(&self, c: &Crop) -> DbResult<()> {
        let life = encode_lifespan(c.lifespan);
        let result = sqlx::query(
            "UPDATE crop SET family_id = ?, strata_id = ?, name = ?, latin_name = ?, \
             pruning_season = ?, lifespan_kind = ?, lifespan_years = ?, \
             productive_pattern = ?, years_to_first_yield = ? WHERE id = ?",
        )
        .bind(c.family_id.as_uuid())
        .bind(c.strata_id.as_uuid())
        .bind(&c.name)
        .bind(c.latin_name.as_deref())
        .bind(encode_pruning(c.pruning_season))
        .bind(life.kind)
        .bind(life.lifespan_years)
        .bind(life.pattern)
        .bind(life.years_to_first_yield)
        .bind(c.id.as_uuid())
        .execute(&self.pool)
        .await?;
        if result.rows_affected() == 0 {
            return Err(DbError::NotFound {
                kind: "crop",
                id: c.id.to_string(),
            });
        }
        Ok(())
    }

    async fn crop_delete(&self, id: CropId) -> DbResult<()> {
        let result = sqlx::query("DELETE FROM crop WHERE id = ?")
            .bind(id.as_uuid())
            .execute(&self.pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(DbError::NotFound {
                kind: "crop",
                id: id.to_string(),
            });
        }
        Ok(())
    }
}

fn row_to_crop(row: sqlx::mysql::MySqlRow) -> DbResult<Crop> {
    let id: Uuid = row.try_get("id")?;
    let family_id: Uuid = row.try_get("family_id")?;
    let strata_id: Uuid = row.try_get("strata_id")?;
    let pruning: String = row.try_get("pruning_season")?;
    let lifespan_kind: String = row.try_get("lifespan_kind")?;
    let lifespan_years: Option<i64> = row.try_get("lifespan_years")?;
    let pattern: Option<String> = row.try_get("productive_pattern")?;
    let yfty: Option<i64> = row.try_get("years_to_first_yield")?;
    Ok(Crop {
        id: CropId::from(id),
        family_id: FamilyId::from(family_id),
        strata_id: StrataId::from(strata_id),
        name: row.try_get("name")?,
        latin_name: row.try_get("latin_name")?,
        pruning_season: decode_pruning(&pruning)?,
        lifespan: decode_lifespan(&lifespan_kind, lifespan_years, pattern.as_deref(), yfty)?,
    })
}
