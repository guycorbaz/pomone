//! `TreatmentRepo` implementation for SQLite.

use crate::codec::{decimal_from_text, decimal_to_text};
use crate::error::{DbError, DbResult};
use crate::repository::TreatmentRepo;
use crate::sqlite::SqliteRepository;
use async_trait::async_trait;
use pomone_domain::{PlantingId, Treatment, TreatmentId};
use sqlx::Row;
use uuid::Uuid;

const TREATMENT_COLUMNS: &str =
    "id, planting_id, applied_on, active_substance, product_name, dose, dose_unit, notes";

#[async_trait]
impl TreatmentRepo for SqliteRepository {
    async fn treatment_get(&self, id: TreatmentId) -> DbResult<Option<Treatment>> {
        let row = sqlx::query(&format!(
            "SELECT {TREATMENT_COLUMNS} FROM treatment WHERE id = ?1"
        ))
        .bind(id.as_uuid())
        .fetch_optional(&self.pool)
        .await?;
        row.map(row_to_treatment).transpose()
    }

    async fn treatment_list_for_planting(
        &self,
        planting_id: PlantingId,
    ) -> DbResult<Vec<Treatment>> {
        let rows = sqlx::query(&format!(
            "SELECT {TREATMENT_COLUMNS} FROM treatment \
             WHERE planting_id = ?1 ORDER BY applied_on DESC"
        ))
        .bind(planting_id.as_uuid())
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(row_to_treatment).collect()
    }

    async fn treatment_create(&self, t: &Treatment) -> DbResult<()> {
        sqlx::query(
            "INSERT INTO treatment \
             (id, planting_id, applied_on, active_substance, product_name, dose, dose_unit, notes) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        )
        .bind(t.id.as_uuid())
        .bind(t.planting_id.as_uuid())
        .bind(t.applied_on)
        .bind(&t.active_substance)
        .bind(&t.product_name)
        .bind(decimal_to_text(t.dose))
        .bind(&t.dose_unit)
        .bind(t.notes.as_deref())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn treatment_delete(&self, id: TreatmentId) -> DbResult<()> {
        let result = sqlx::query("DELETE FROM treatment WHERE id = ?1")
            .bind(id.as_uuid())
            .execute(&self.pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(DbError::NotFound {
                kind: "treatment",
                id: id.to_string(),
            });
        }
        Ok(())
    }
}

fn row_to_treatment(row: sqlx::sqlite::SqliteRow) -> DbResult<Treatment> {
    let id: Uuid = row.try_get("id")?;
    let planting_id: Uuid = row.try_get("planting_id")?;
    let dose_text: String = row.try_get("dose")?;
    Ok(Treatment {
        id: TreatmentId::from(id),
        planting_id: PlantingId::from(planting_id),
        applied_on: row.try_get("applied_on")?,
        active_substance: row.try_get("active_substance")?,
        product_name: row.try_get("product_name")?,
        dose: decimal_from_text(&dose_text)?,
        dose_unit: row.try_get("dose_unit")?,
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
        AnnualProfile, Crop, Family, Lifespan, Location, LocationKind, Planting, PlantingSchedule,
        PruningSeason, Strata, Variety, VarietyProfile,
    };
    use rust_decimal_macros::dec;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    async fn setup_with_annual_planting() -> (SqliteRepository, PlantingId) {
        let repo = SqliteRepository::in_memory().await.unwrap();
        let f = Family::new("Solanaceae", None, None).unwrap();
        let s = Strata::new("Herbacée", None, None, None, 10).unwrap();
        let k = LocationKind::new("Planche", None).unwrap();
        repo.family_create(&f).await.unwrap();
        repo.strata_create(&s).await.unwrap();
        repo.location_kind_create(&k).await.unwrap();
        let crop = Crop::new(f.id, "Tomate", None, Lifespan::Annual, PruningSeason::None).unwrap();
        repo.crop_create(&crop).await.unwrap();
        let v = Variety::new(
            crop.id,
            Lifespan::Annual,
            "Rose de Berne",
            None,
            VarietyProfile::Annual(AnnualProfile::new(Some(35), 70, 60).unwrap()),
        )
        .unwrap();
        repo.variety_create(&v).await.unwrap();
        let loc = Location::new(k.id, "Planche 1", dec!(20), dec!(1), None, None).unwrap();
        repo.location_create(&loc).await.unwrap();
        let p = Planting::new(
            v.id,
            loc.id,
            s.id,
            Lifespan::Annual,
            dec!(20),
            50,
            PlantingSchedule::cycle(Some(d(2026, 3, 1)), None, d(2026, 7, 1), d(2026, 9, 30))
                .unwrap(),
            None,
            None,
        )
        .unwrap();
        repo.planting_create(&p).await.unwrap();
        (repo, p.id)
    }

    fn sample_treatment(pid: PlantingId, applied_on: NaiveDate) -> Treatment {
        Treatment::new(
            pid,
            applied_on,
            "cuivre",
            "Bouillie bordelaise",
            dec!(1.25),
            "kg/ha",
            Some("avant pluie".into()),
        )
        .unwrap()
    }

    #[tokio::test]
    async fn create_then_get_roundtrips() {
        let (repo, pid) = setup_with_annual_planting().await;
        let t = sample_treatment(pid, d(2026, 6, 10));
        repo.treatment_create(&t).await.unwrap();
        let got = repo.treatment_get(t.id).await.unwrap().unwrap();
        assert_eq!(got, t);
    }

    #[tokio::test]
    async fn list_for_planting_orders_newest_first() {
        let (repo, pid) = setup_with_annual_planting().await;
        for date in [d(2026, 5, 1), d(2026, 7, 1), d(2026, 6, 1)] {
            repo.treatment_create(&sample_treatment(pid, date))
                .await
                .unwrap();
        }
        let dates: Vec<_> = repo
            .treatment_list_for_planting(pid)
            .await
            .unwrap()
            .into_iter()
            .map(|t| t.applied_on)
            .collect();
        assert_eq!(dates, vec![d(2026, 7, 1), d(2026, 6, 1), d(2026, 5, 1)]);
    }

    #[tokio::test]
    async fn delete_removes_row_and_missing_is_not_found() {
        let (repo, pid) = setup_with_annual_planting().await;
        let t = sample_treatment(pid, d(2026, 6, 10));
        repo.treatment_create(&t).await.unwrap();
        repo.treatment_delete(t.id).await.unwrap();
        assert!(repo.treatment_get(t.id).await.unwrap().is_none());
        let err = repo.treatment_delete(t.id).await.unwrap_err();
        assert!(matches!(
            err,
            DbError::NotFound {
                kind: "treatment",
                ..
            }
        ));
    }

    #[tokio::test]
    async fn deleting_planting_cascades_to_treatments() {
        let (repo, pid) = setup_with_annual_planting().await;
        let t = sample_treatment(pid, d(2026, 6, 10));
        repo.treatment_create(&t).await.unwrap();
        repo.planting_delete(pid).await.unwrap();
        assert!(repo.treatment_get(t.id).await.unwrap().is_none());
    }
}
