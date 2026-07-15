//! `CropPlanLineRepo` implementation for MariaDB.

use crate::error::{DbError, DbResult};
use crate::mariadb::MariaDbRepository;
use crate::repository::CropPlanLineRepo;
use async_trait::async_trait;
use pomone_domain::{CropPlanLine, CropPlanLineId, VarietyId};
use rust_decimal::Decimal;
use sqlx::Row;
use uuid::Uuid;

const CROP_PLAN_LINE_COLUMNS: &str =
    "id, variety_id, series, bed_meters, stagger_days, draft, notes";

#[async_trait]
impl CropPlanLineRepo for MariaDbRepository {
    async fn crop_plan_line_get(&self, id: CropPlanLineId) -> DbResult<Option<CropPlanLine>> {
        let row = sqlx::query(&format!(
            "SELECT {CROP_PLAN_LINE_COLUMNS} FROM crop_plan_line WHERE id = ?"
        ))
        .bind(id.as_uuid())
        .fetch_optional(&self.pool)
        .await?;
        row.map(row_to_crop_plan_line).transpose()
    }

    async fn crop_plan_line_list(&self) -> DbResult<Vec<CropPlanLine>> {
        let rows = sqlx::query(&format!(
            "SELECT {CROP_PLAN_LINE_COLUMNS} FROM crop_plan_line ORDER BY variety_id, id"
        ))
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(row_to_crop_plan_line).collect()
    }

    async fn crop_plan_line_create(&self, line: &CropPlanLine) -> DbResult<()> {
        sqlx::query(
            "INSERT INTO crop_plan_line \
             (id, variety_id, series, bed_meters, stagger_days, draft, notes) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(line.id.as_uuid())
        .bind(line.variety_id.as_uuid())
        .bind(i64::from(line.series))
        .bind(line.bed_meters)
        .bind(i64::from(line.stagger_days))
        .bind(line.draft)
        .bind(line.notes.as_deref())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn crop_plan_line_update(&self, line: &CropPlanLine) -> DbResult<()> {
        let result = sqlx::query(
            "UPDATE crop_plan_line SET variety_id = ?, series = ?, bed_meters = ?, \
             stagger_days = ?, draft = ?, notes = ? WHERE id = ?",
        )
        .bind(line.variety_id.as_uuid())
        .bind(i64::from(line.series))
        .bind(line.bed_meters)
        .bind(i64::from(line.stagger_days))
        .bind(line.draft)
        .bind(line.notes.as_deref())
        .bind(line.id.as_uuid())
        .execute(&self.pool)
        .await?;
        if result.rows_affected() == 0 {
            return Err(DbError::NotFound {
                kind: "crop_plan_line",
                id: line.id.to_string(),
            });
        }
        Ok(())
    }

    async fn crop_plan_line_delete(&self, id: CropPlanLineId) -> DbResult<()> {
        let result = sqlx::query("DELETE FROM crop_plan_line WHERE id = ?")
            .bind(id.as_uuid())
            .execute(&self.pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(DbError::NotFound {
                kind: "crop_plan_line",
                id: id.to_string(),
            });
        }
        Ok(())
    }
}

fn row_to_crop_plan_line(row: sqlx::mysql::MySqlRow) -> DbResult<CropPlanLine> {
    let id: Uuid = row.try_get("id")?;
    let variety_id: Uuid = row.try_get("variety_id")?;
    let series: i64 = row.try_get("series")?;
    let stagger_days: i64 = row.try_get("stagger_days")?;
    Ok(CropPlanLine {
        id: CropPlanLineId::from(id),
        variety_id: VarietyId::from(variety_id),
        series: u32::try_from(series)
            .map_err(|_| DbError::Malformed(format!("series out of u32 range: {series}")))?,
        bed_meters: row.try_get::<Decimal, _>("bed_meters")?,
        stagger_days: u32::try_from(stagger_days).map_err(|_| {
            DbError::Malformed(format!("stagger_days out of u32 range: {stagger_days}"))
        })?,
        draft: row.try_get("draft")?,
        notes: row.try_get("notes")?,
    })
}
