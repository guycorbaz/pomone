//! `VarietyRepo` implementation for MariaDB.

use crate::codec::{decode_variety_profile, encode_variety_profile};
use crate::error::{DbError, DbResult};
use crate::mariadb::MariaDbRepository;
use crate::repository::VarietyRepo;
use async_trait::async_trait;
use pomone_domain::{CropId, Variety, VarietyId};
use rust_decimal::Decimal;
use sqlx::Row;
use uuid::Uuid;

const COLUMNS: &str = "id, crop_id, name, description, profile_kind, \
                       days_to_transplant, days_to_maturity, harvest_window_days, \
                       bud_break_doy, flowering_doy, harvest_start_doy, harvest_end_doy, \
                       expected_yield_kg_per_plant";

#[async_trait]
impl VarietyRepo for MariaDbRepository {
    async fn variety_get(&self, id: VarietyId) -> DbResult<Option<Variety>> {
        let sql = format!("SELECT {COLUMNS} FROM variety WHERE id = ?");
        let row = sqlx::query(&sql)
            .bind(id.as_uuid())
            .fetch_optional(&self.pool)
            .await?;
        row.map(row_to_variety).transpose()
    }

    async fn variety_list(&self) -> DbResult<Vec<Variety>> {
        let sql = format!("SELECT {COLUMNS} FROM variety ORDER BY name");
        let rows = sqlx::query(&sql).fetch_all(&self.pool).await?;
        rows.into_iter().map(row_to_variety).collect()
    }

    async fn variety_list_for_crop(&self, crop_id: CropId) -> DbResult<Vec<Variety>> {
        let sql = format!("SELECT {COLUMNS} FROM variety WHERE crop_id = ? ORDER BY name");
        let rows = sqlx::query(&sql)
            .bind(crop_id.as_uuid())
            .fetch_all(&self.pool)
            .await?;
        rows.into_iter().map(row_to_variety).collect()
    }

    async fn variety_create(&self, v: &Variety) -> DbResult<()> {
        let p = encode_variety_profile(v.profile);
        sqlx::query(
            "INSERT INTO variety (id, crop_id, name, description, profile_kind, \
             days_to_transplant, days_to_maturity, harvest_window_days, \
             bud_break_doy, flowering_doy, harvest_start_doy, harvest_end_doy, \
             expected_yield_kg_per_plant) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(v.id.as_uuid())
        .bind(v.crop_id.as_uuid())
        .bind(&v.name)
        .bind(v.description.as_deref())
        .bind(p.kind)
        .bind(p.days_to_transplant)
        .bind(p.days_to_maturity)
        .bind(p.harvest_window_days)
        .bind(p.bud_break_doy)
        .bind(p.flowering_doy)
        .bind(p.harvest_start_doy)
        .bind(p.harvest_end_doy)
        .bind(p.expected_yield_kg_per_plant)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn variety_update(&self, v: &Variety) -> DbResult<()> {
        let p = encode_variety_profile(v.profile);
        let result = sqlx::query(
            "UPDATE variety SET crop_id = ?, name = ?, description = ?, profile_kind = ?, \
             days_to_transplant = ?, days_to_maturity = ?, harvest_window_days = ?, \
             bud_break_doy = ?, flowering_doy = ?, harvest_start_doy = ?, \
             harvest_end_doy = ?, expected_yield_kg_per_plant = ? WHERE id = ?",
        )
        .bind(v.crop_id.as_uuid())
        .bind(&v.name)
        .bind(v.description.as_deref())
        .bind(p.kind)
        .bind(p.days_to_transplant)
        .bind(p.days_to_maturity)
        .bind(p.harvest_window_days)
        .bind(p.bud_break_doy)
        .bind(p.flowering_doy)
        .bind(p.harvest_start_doy)
        .bind(p.harvest_end_doy)
        .bind(p.expected_yield_kg_per_plant)
        .bind(v.id.as_uuid())
        .execute(&self.pool)
        .await?;
        if result.rows_affected() == 0 {
            return Err(DbError::NotFound {
                kind: "variety",
                id: v.id.to_string(),
            });
        }
        Ok(())
    }

    async fn variety_delete(&self, id: VarietyId) -> DbResult<()> {
        let result = sqlx::query("DELETE FROM variety WHERE id = ?")
            .bind(id.as_uuid())
            .execute(&self.pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(DbError::NotFound {
                kind: "variety",
                id: id.to_string(),
            });
        }
        Ok(())
    }
}

fn row_to_variety(row: sqlx::mysql::MySqlRow) -> DbResult<Variety> {
    let id: Uuid = row.try_get("id")?;
    let crop_id: Uuid = row.try_get("crop_id")?;
    let kind: String = row.try_get("profile_kind")?;
    let yield_decimal: Option<Decimal> = row.try_get("expected_yield_kg_per_plant")?;
    let profile = decode_variety_profile(
        &kind,
        row.try_get("days_to_transplant")?,
        row.try_get("days_to_maturity")?,
        row.try_get("harvest_window_days")?,
        row.try_get("bud_break_doy")?,
        row.try_get("flowering_doy")?,
        row.try_get("harvest_start_doy")?,
        row.try_get("harvest_end_doy")?,
        yield_decimal,
    )?;
    Ok(Variety {
        id: VarietyId::from(id),
        crop_id: CropId::from(crop_id),
        name: row.try_get("name")?,
        description: row.try_get("description")?,
        profile,
    })
}
