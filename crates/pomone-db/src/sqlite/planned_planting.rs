//! `PlannedPlantingRepo` implementation for SQLite.

use crate::codec::{decimal_from_text, decimal_to_text};
use crate::error::{DbError, DbResult};
use crate::repository::PlannedPlantingRepo;
use crate::sqlite::SqliteRepository;
use async_trait::async_trait;
use pomone_domain::{CropPlanLineId, PlannedPlanting, PlannedPlantingId, PlantingId, VarietyId};
use sqlx::Row;
use uuid::Uuid;

const COLUMNS: &str =
    "id, crop_plan_line_id, variety_id, series_index, planned_on, bed_meters, placed_planting_id";

#[async_trait]
impl PlannedPlantingRepo for SqliteRepository {
    async fn planned_planting_list_for_line(
        &self,
        line_id: CropPlanLineId,
    ) -> DbResult<Vec<PlannedPlanting>> {
        let rows = sqlx::query(&format!(
            "SELECT {COLUMNS} FROM planned_planting \
             WHERE crop_plan_line_id = ?1 ORDER BY series_index"
        ))
        .bind(line_id.as_uuid())
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(row_to_planned).collect()
    }

    async fn planned_planting_list_all(&self) -> DbResult<Vec<PlannedPlanting>> {
        let rows = sqlx::query(&format!(
            "SELECT {COLUMNS} FROM planned_planting ORDER BY crop_plan_line_id, series_index"
        ))
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(row_to_planned).collect()
    }

    async fn planned_planting_create(&self, pp: &PlannedPlanting) -> DbResult<()> {
        sqlx::query(&format!(
            "INSERT INTO planned_planting ({COLUMNS}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"
        ))
        .bind(pp.id.as_uuid())
        .bind(pp.crop_plan_line_id.as_uuid())
        .bind(pp.variety_id.as_uuid())
        .bind(i64::from(pp.series_index))
        .bind(pp.planned_on)
        .bind(decimal_to_text(pp.bed_meters))
        .bind(pp.placed_planting_id.map(PlantingId::as_uuid))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn planned_planting_update(&self, pp: &PlannedPlanting) -> DbResult<()> {
        let result = sqlx::query(
            "UPDATE planned_planting SET crop_plan_line_id = ?2, variety_id = ?3, \
             series_index = ?4, planned_on = ?5, bed_meters = ?6, placed_planting_id = ?7 \
             WHERE id = ?1",
        )
        .bind(pp.id.as_uuid())
        .bind(pp.crop_plan_line_id.as_uuid())
        .bind(pp.variety_id.as_uuid())
        .bind(i64::from(pp.series_index))
        .bind(pp.planned_on)
        .bind(decimal_to_text(pp.bed_meters))
        .bind(pp.placed_planting_id.map(PlantingId::as_uuid))
        .execute(&self.pool)
        .await?;
        if result.rows_affected() == 0 {
            return Err(DbError::NotFound {
                kind: "planned_planting",
                id: pp.id.to_string(),
            });
        }
        Ok(())
    }

    async fn planned_planting_delete(&self, id: PlannedPlantingId) -> DbResult<()> {
        let result = sqlx::query("DELETE FROM planned_planting WHERE id = ?1")
            .bind(id.as_uuid())
            .execute(&self.pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(DbError::NotFound {
                kind: "planned_planting",
                id: id.to_string(),
            });
        }
        Ok(())
    }
}

fn row_to_planned(row: sqlx::sqlite::SqliteRow) -> DbResult<PlannedPlanting> {
    let id: Uuid = row.try_get("id")?;
    let line_id: Uuid = row.try_get("crop_plan_line_id")?;
    let variety_id: Uuid = row.try_get("variety_id")?;
    let series_index: i64 = row.try_get("series_index")?;
    let bed_meters_text: String = row.try_get("bed_meters")?;
    let placed_planting_id: Option<Uuid> = row.try_get("placed_planting_id")?;
    Ok(PlannedPlanting {
        id: PlannedPlantingId::from(id),
        crop_plan_line_id: CropPlanLineId::from(line_id),
        variety_id: VarietyId::from(variety_id),
        series_index: u32::try_from(series_index).map_err(|_| {
            DbError::Malformed(format!("series_index out of u32 range: {series_index}"))
        })?,
        planned_on: row.try_get("planned_on")?,
        bed_meters: decimal_from_text(&bed_meters_text)?,
        placed_planting_id: placed_planting_id.map(PlantingId::from),
    })
}
