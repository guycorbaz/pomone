//! `CropPlanLineRepo` implementation for SQLite.

use crate::codec::{decimal_from_text, decimal_to_text};
use crate::error::{DbError, DbResult};
use crate::repository::CropPlanLineRepo;
use crate::sqlite::SqliteRepository;
use async_trait::async_trait;
use pomone_domain::{CropPlanLine, CropPlanLineId, VarietyId};
use sqlx::Row;
use uuid::Uuid;

const CROP_PLAN_LINE_COLUMNS: &str =
    "id, variety_id, series, bed_meters, stagger_days, first_on, draft, notes";

#[async_trait]
impl CropPlanLineRepo for SqliteRepository {
    async fn crop_plan_line_get(&self, id: CropPlanLineId) -> DbResult<Option<CropPlanLine>> {
        let row = sqlx::query(&format!(
            "SELECT {CROP_PLAN_LINE_COLUMNS} FROM crop_plan_line WHERE id = ?1"
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
             (id, variety_id, series, bed_meters, stagger_days, first_on, draft, notes) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        )
        .bind(line.id.as_uuid())
        .bind(line.variety_id.as_uuid())
        .bind(i64::from(line.series))
        .bind(decimal_to_text(line.bed_meters))
        .bind(i64::from(line.stagger_days))
        .bind(line.first_on)
        .bind(line.draft)
        .bind(line.notes.as_deref())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn crop_plan_line_update(&self, line: &CropPlanLine) -> DbResult<()> {
        let result = sqlx::query(
            "UPDATE crop_plan_line SET variety_id = ?2, series = ?3, bed_meters = ?4, \
             stagger_days = ?5, first_on = ?6, draft = ?7, notes = ?8 WHERE id = ?1",
        )
        .bind(line.id.as_uuid())
        .bind(line.variety_id.as_uuid())
        .bind(i64::from(line.series))
        .bind(decimal_to_text(line.bed_meters))
        .bind(i64::from(line.stagger_days))
        .bind(line.first_on)
        .bind(line.draft)
        .bind(line.notes.as_deref())
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
        let result = sqlx::query("DELETE FROM crop_plan_line WHERE id = ?1")
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

fn row_to_crop_plan_line(row: sqlx::sqlite::SqliteRow) -> DbResult<CropPlanLine> {
    let id: Uuid = row.try_get("id")?;
    let variety_id: Uuid = row.try_get("variety_id")?;
    let series: i64 = row.try_get("series")?;
    let stagger_days: i64 = row.try_get("stagger_days")?;
    let bed_meters_text: String = row.try_get("bed_meters")?;
    Ok(CropPlanLine {
        id: CropPlanLineId::from(id),
        variety_id: VarietyId::from(variety_id),
        series: u32::try_from(series)
            .map_err(|_| DbError::Malformed(format!("series out of u32 range: {series}")))?,
        bed_meters: decimal_from_text(&bed_meters_text)?,
        stagger_days: u32::try_from(stagger_days).map_err(|_| {
            DbError::Malformed(format!("stagger_days out of u32 range: {stagger_days}"))
        })?,
        first_on: row.try_get("first_on")?,
        draft: row.try_get("draft")?,
        notes: row.try_get("notes")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::{CropRepo, FamilyRepo, VarietyRepo};
    use pomone_domain::{
        AnnualProfile, Crop, Family, Lifespan, PruningSeason, Variety, VarietyProfile,
    };
    use rust_decimal_macros::dec;

    async fn repo_with_variety() -> (SqliteRepository, VarietyId) {
        let repo = SqliteRepository::in_memory().await.unwrap();
        let f = Family::new("Asteraceae", None, None).unwrap();
        repo.family_create(&f).await.unwrap();
        let crop = Crop::new(f.id, "Laitue", None, Lifespan::Annual, PruningSeason::None).unwrap();
        repo.crop_create(&crop).await.unwrap();
        let v = Variety::new(
            crop.id,
            Lifespan::Annual,
            "Batavia",
            None,
            VarietyProfile::Annual(AnnualProfile::new(Some(20), 45, 30).unwrap()),
        )
        .unwrap();
        repo.variety_create(&v).await.unwrap();
        (repo, v.id)
    }

    fn sample(vid: VarietyId, series: u32, draft: bool) -> CropPlanLine {
        CropPlanLine::new(
            vid,
            series,
            dec!(15),
            14,
            chrono::NaiveDate::from_ymd_opt(2026, 4, 1),
            draft,
            Some("batavia".into()),
        )
        .unwrap()
    }

    #[tokio::test]
    async fn create_then_get_roundtrips_all_fields() {
        let (repo, vid) = repo_with_variety().await;
        let line = sample(vid, 6, true);
        repo.crop_plan_line_create(&line).await.unwrap();
        let got = repo.crop_plan_line_get(line.id).await.unwrap().unwrap();
        assert_eq!(got, line);
    }

    #[tokio::test]
    async fn update_changes_fields_keeps_id_and_missing_is_not_found() {
        let (repo, vid) = repo_with_variety().await;
        let line = sample(vid, 3, true);
        repo.crop_plan_line_create(&line).await.unwrap();
        let promoted = line
            .clone()
            .with_updates(vid, 8, dec!(20), 7, None, false, None)
            .unwrap();
        repo.crop_plan_line_update(&promoted).await.unwrap();
        let got = repo.crop_plan_line_get(line.id).await.unwrap().unwrap();
        assert_eq!(got.series, 8);
        assert_eq!(got.bed_meters, dec!(20));
        assert!(!got.draft);
        assert_eq!(got.notes, None);

        // Updating an absent line is NotFound.
        repo.crop_plan_line_delete(line.id).await.unwrap();
        let err = repo.crop_plan_line_update(&promoted).await.unwrap_err();
        assert!(matches!(
            err,
            DbError::NotFound {
                kind: "crop_plan_line",
                ..
            }
        ));
    }

    #[tokio::test]
    async fn list_returns_all_lines() {
        let (repo, vid) = repo_with_variety().await;
        for s in [1, 2, 3] {
            repo.crop_plan_line_create(&sample(vid, s, false))
                .await
                .unwrap();
        }
        assert_eq!(repo.crop_plan_line_list().await.unwrap().len(), 3);
    }

    #[tokio::test]
    async fn delete_removes_row_and_missing_is_not_found() {
        let (repo, vid) = repo_with_variety().await;
        let line = sample(vid, 2, false);
        repo.crop_plan_line_create(&line).await.unwrap();
        repo.crop_plan_line_delete(line.id).await.unwrap();
        assert!(repo.crop_plan_line_get(line.id).await.unwrap().is_none());
        let err = repo.crop_plan_line_delete(line.id).await.unwrap_err();
        assert!(matches!(
            err,
            DbError::NotFound {
                kind: "crop_plan_line",
                ..
            }
        ));
    }

    #[tokio::test]
    async fn variety_in_use_by_a_line_cannot_be_deleted() {
        let (repo, vid) = repo_with_variety().await;
        repo.crop_plan_line_create(&sample(vid, 2, false))
            .await
            .unwrap();
        // ON DELETE RESTRICT: the FK refuses to orphan a planned line.
        assert!(repo.variety_delete(vid).await.is_err());
    }
}
