//! `YearlyHarvestRepo` implementation for SQLite.

use crate::codec::{opt_decimal_from_text, opt_decimal_to_text};
use crate::error::{DbError, DbResult};
use crate::repository::YearlyHarvestRepo;
use crate::sqlite::SqliteRepository;
use async_trait::async_trait;
use pomone_domain::{PlantingId, YearlyHarvest};
use sqlx::Row;
use uuid::Uuid;

#[async_trait]
impl YearlyHarvestRepo for SqliteRepository {
    async fn yearly_harvest_get(
        &self,
        planting_id: PlantingId,
        year: i32,
    ) -> DbResult<Option<YearlyHarvest>> {
        let row = sqlx::query(
            "SELECT planting_id, year, expected_yield_kg, actual_yield_kg, notes \
             FROM yearly_harvest WHERE planting_id = ?1 AND year = ?2",
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
             FROM yearly_harvest WHERE planting_id = ?1 ORDER BY year",
        )
        .bind(planting_id.as_uuid())
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(row_to_harvest).collect()
    }

    async fn yearly_harvest_upsert(&self, h: &YearlyHarvest) -> DbResult<()> {
        // SQLite supports ON CONFLICT (since 3.24); both fields make up the PK.
        sqlx::query(
            "INSERT INTO yearly_harvest (planting_id, year, expected_yield_kg, actual_yield_kg, notes) \
             VALUES (?1, ?2, ?3, ?4, ?5) \
             ON CONFLICT(planting_id, year) DO UPDATE SET \
             expected_yield_kg = excluded.expected_yield_kg, \
             actual_yield_kg = excluded.actual_yield_kg, \
             notes = excluded.notes",
        )
        .bind(h.planting_id.as_uuid())
        .bind(h.year)
        .bind(opt_decimal_to_text(h.expected_yield_kg))
        .bind(opt_decimal_to_text(h.actual_yield_kg))
        .bind(h.notes.as_deref())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn yearly_harvest_delete(&self, planting_id: PlantingId, year: i32) -> DbResult<()> {
        let result = sqlx::query("DELETE FROM yearly_harvest WHERE planting_id = ?1 AND year = ?2")
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

fn row_to_harvest(row: sqlx::sqlite::SqliteRow) -> DbResult<YearlyHarvest> {
    let planting_id: Uuid = row.try_get("planting_id")?;
    let expected_text: Option<String> = row.try_get("expected_yield_kg")?;
    let actual_text: Option<String> = row.try_get("actual_yield_kg")?;
    Ok(YearlyHarvest {
        planting_id: PlantingId::from(planting_id),
        year: row.try_get("year")?,
        expected_yield_kg: opt_decimal_from_text(expected_text)?,
        actual_yield_kg: opt_decimal_from_text(actual_text)?,
        notes: row.try_get("notes")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::{
        CropRepo, FamilyRepo, LocationKindRepo, LocationRepo, PlantingRepo, StrataRepo, VarietyRepo,
    };
    use chrono::NaiveDate;
    use pomone_domain::{
        Crop, Family, Lifespan, Location, LocationKind, Planting, PlantingSchedule,
        PluriannualProfile, PruningSeason, Strata, Variety, VarietyProfile,
    };
    use rust_decimal_macros::dec;

    async fn setup_with_perennial_planting() -> (SqliteRepository, PlantingId) {
        let repo = SqliteRepository::in_memory().await.unwrap();
        let f = Family::new("Rosaceae", None, None).unwrap();
        let s = Strata::new("Sous-étage", None, None, None, 20).unwrap();
        let k = LocationKind::new("Verger", None).unwrap();
        repo.family_create(&f).await.unwrap();
        repo.strata_create(&s).await.unwrap();
        repo.location_kind_create(&k).await.unwrap();
        let lifespan = Lifespan::perennial(40, 3).unwrap();
        let crop = Crop::new(f.id, "Pommier", None, lifespan, PruningSeason::Winter).unwrap();
        repo.crop_create(&crop).await.unwrap();
        let v = Variety::new(
            crop.id,
            lifespan,
            "Reinette",
            None,
            VarietyProfile::Pluriannual(
                PluriannualProfile::new(None, None, 220, 280, None).unwrap(),
            ),
        )
        .unwrap();
        repo.variety_create(&v).await.unwrap();
        let loc = Location::new(k.id, "Verger nord", dec!(50), dec!(40), None, None).unwrap();
        repo.location_create(&loc).await.unwrap();
        let p = Planting::new(
            v.id,
            loc.id,
            s.id,
            lifespan,
            dec!(2000),
            50,
            PlantingSchedule::perennial(NaiveDate::from_ymd_opt(2026, 3, 15).unwrap(), None)
                .unwrap(),
            Some("Verger nord".into()),
            None,
        )
        .unwrap();
        repo.planting_create(&p).await.unwrap();
        (repo, p.id)
    }

    #[tokio::test]
    async fn upsert_inserts_then_updates() {
        let (repo, pid) = setup_with_perennial_planting().await;
        let h = YearlyHarvest::new(pid, 2030, Some(dec!(100)), None, None).unwrap();
        repo.yearly_harvest_upsert(&h).await.unwrap();
        // Now update the same (planting_id, year) with actual yield
        let h2 = YearlyHarvest::new(
            pid,
            2030,
            Some(dec!(100)),
            Some(dec!(85)),
            Some("frost".into()),
        )
        .unwrap();
        repo.yearly_harvest_upsert(&h2).await.unwrap();

        let got = repo.yearly_harvest_get(pid, 2030).await.unwrap().unwrap();
        assert_eq!(got.actual_yield_kg, Some(dec!(85)));
        assert_eq!(got.notes.as_deref(), Some("frost"));
    }

    #[tokio::test]
    async fn list_for_planting_orders_by_year() {
        let (repo, pid) = setup_with_perennial_planting().await;
        for y in [2032, 2030, 2031] {
            let h = YearlyHarvest::new(pid, y, Some(dec!(50)), None, None).unwrap();
            repo.yearly_harvest_upsert(&h).await.unwrap();
        }
        let years: Vec<_> = repo
            .yearly_harvest_list_for_planting(pid)
            .await
            .unwrap()
            .into_iter()
            .map(|h| h.year)
            .collect();
        assert_eq!(years, vec![2030, 2031, 2032]);
    }

    #[tokio::test]
    async fn delete_removes_row() {
        let (repo, pid) = setup_with_perennial_planting().await;
        let h = YearlyHarvest::new(pid, 2030, Some(dec!(50)), None, None).unwrap();
        repo.yearly_harvest_upsert(&h).await.unwrap();
        repo.yearly_harvest_delete(pid, 2030).await.unwrap();
        assert!(repo.yearly_harvest_get(pid, 2030).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn delete_missing_returns_not_found() {
        let (repo, pid) = setup_with_perennial_planting().await;
        let err = repo.yearly_harvest_delete(pid, 2030).await.unwrap_err();
        assert!(matches!(
            err,
            DbError::NotFound {
                kind: "yearly_harvest",
                ..
            }
        ));
    }

    #[tokio::test]
    async fn deleting_planting_cascades_to_harvests() {
        let (repo, pid) = setup_with_perennial_planting().await;
        let h = YearlyHarvest::new(pid, 2030, Some(dec!(50)), None, None).unwrap();
        repo.yearly_harvest_upsert(&h).await.unwrap();
        repo.planting_delete(pid).await.unwrap();
        assert!(repo.yearly_harvest_get(pid, 2030).await.unwrap().is_none());
    }
}
