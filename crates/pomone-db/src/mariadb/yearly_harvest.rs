//! `YearlyHarvestRepo` implementation for MariaDB.

use crate::error::{DbError, DbResult};
use crate::mariadb::MariaDbRepository;
use crate::repository::YearlyHarvestRepo;
use async_trait::async_trait;
use pomone_domain::{PlantingId, YearlyHarvest};
use rust_decimal::Decimal;
use sqlx::Row;
use uuid::Uuid;

#[async_trait]
impl YearlyHarvestRepo for MariaDbRepository {
    async fn yearly_harvest_get(
        &self,
        planting_id: PlantingId,
        year: i32,
    ) -> DbResult<Option<YearlyHarvest>> {
        let row = sqlx::query(
            "SELECT planting_id, year, expected_yield_kg, actual_yield_kg, notes \
             FROM yearly_harvest WHERE planting_id = ? AND year = ?",
        )
        .bind(planting_id.as_uuid())
        .bind(year)
        .fetch_optional(&self.pool)
        .await?;
        row.map(row_to_harvest).transpose()
    }

    async fn yearly_harvest_list_for_planting(
        &self,
        planting_id: PlantingId,
    ) -> DbResult<Vec<YearlyHarvest>> {
        let rows = sqlx::query(
            "SELECT planting_id, year, expected_yield_kg, actual_yield_kg, notes \
             FROM yearly_harvest WHERE planting_id = ? ORDER BY year",
        )
        .bind(planting_id.as_uuid())
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(row_to_harvest).collect()
    }

    async fn yearly_harvest_upsert(&self, h: &YearlyHarvest) -> DbResult<()> {
        // MariaDB syntax: INSERT ... ON DUPLICATE KEY UPDATE
        sqlx::query(
            "INSERT INTO yearly_harvest (planting_id, year, expected_yield_kg, actual_yield_kg, notes) \
             VALUES (?, ?, ?, ?, ?) \
             ON DUPLICATE KEY UPDATE \
             expected_yield_kg = VALUES(expected_yield_kg), \
             actual_yield_kg = VALUES(actual_yield_kg), \
             notes = VALUES(notes)",
        )
        .bind(h.planting_id.as_uuid())
        .bind(h.year)
        .bind(h.expected_yield_kg)
        .bind(h.actual_yield_kg)
        .bind(h.notes.as_deref())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn yearly_harvest_delete(&self, planting_id: PlantingId, year: i32) -> DbResult<()> {
        let result = sqlx::query("DELETE FROM yearly_harvest WHERE planting_id = ? AND year = ?")
            .bind(planting_id.as_uuid())
            .bind(year)
            .execute(&self.pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(DbError::NotFound {
                kind: "yearly_harvest",
                id: format!("{planting_id}/{year}"),
            });
        }
        Ok(())
    }
}

fn row_to_harvest(row: sqlx::mysql::MySqlRow) -> DbResult<YearlyHarvest> {
    let planting_id: Uuid = row.try_get("planting_id")?;
    Ok(YearlyHarvest {
        planting_id: PlantingId::from(planting_id),
        year: row.try_get("year")?,
        expected_yield_kg: row.try_get::<Option<Decimal>, _>("expected_yield_kg")?,
        actual_yield_kg: row.try_get::<Option<Decimal>, _>("actual_yield_kg")?,
        notes: row.try_get("notes")?,
    })
}
