//! `VarietyRepo` implementation for SQLite.

use crate::codec::{
    decode_variety_profile, encode_variety_profile, opt_decimal_from_text, opt_decimal_to_text,
};
use crate::error::{DbError, DbResult};
use crate::repository::VarietyRepo;
use crate::sqlite::SqliteRepository;
use async_trait::async_trait;
use pomone_domain::{CropId, Variety, VarietyId};
use sqlx::Row;
use uuid::Uuid;

const COLUMNS: &str = "id, crop_id, name, description, profile_kind, \
                       days_to_transplant, days_to_maturity, harvest_window_days, \
                       bud_break_doy, flowering_doy, harvest_start_doy, harvest_end_doy, \
                       expected_yield_kg_per_plant";

#[async_trait]
impl VarietyRepo for SqliteRepository {
    async fn variety_get(&self, id: VarietyId) -> DbResult<Option<Variety>> {
        let sql = format!("SELECT {COLUMNS} FROM variety WHERE id = ?1");
        let row = sqlx::query(&sql)
            .bind(id.as_uuid())
            .fetch_optional(&self.pool)
            .await?;
        row.map(row_to_variety).transpose()
    }

    async fn variety_list(&self) -> DbResult<Vec<Variety>> {
        let sql = format!("SELECT {COLUMNS} FROM variety ORDER BY name COLLATE NOCASE");
        let rows = sqlx::query(&sql).fetch_all(&self.pool).await?;
        rows.into_iter().map(row_to_variety).collect()
    }

    async fn variety_list_for_crop(&self, crop_id: CropId) -> DbResult<Vec<Variety>> {
        let sql = format!(
            "SELECT {COLUMNS} FROM variety WHERE crop_id = ?1 ORDER BY name COLLATE NOCASE"
        );
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
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
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
        .bind(opt_decimal_to_text(p.expected_yield_kg_per_plant))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn variety_update(&self, v: &Variety) -> DbResult<()> {
        let p = encode_variety_profile(v.profile);
        let result = sqlx::query(
            "UPDATE variety SET crop_id = ?2, name = ?3, description = ?4, profile_kind = ?5, \
             days_to_transplant = ?6, days_to_maturity = ?7, harvest_window_days = ?8, \
             bud_break_doy = ?9, flowering_doy = ?10, harvest_start_doy = ?11, \
             harvest_end_doy = ?12, expected_yield_kg_per_plant = ?13 WHERE id = ?1",
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
        .bind(opt_decimal_to_text(p.expected_yield_kg_per_plant))
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
        let result = sqlx::query("DELETE FROM variety WHERE id = ?1")
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

fn row_to_variety(row: sqlx::sqlite::SqliteRow) -> DbResult<Variety> {
    let id: Uuid = row.try_get("id")?;
    let crop_id: Uuid = row.try_get("crop_id")?;
    let kind: String = row.try_get("profile_kind")?;
    let yield_text: Option<String> = row.try_get("expected_yield_kg_per_plant")?;
    let profile = decode_variety_profile(
        &kind,
        row.try_get("days_to_transplant")?,
        row.try_get("days_to_maturity")?,
        row.try_get("harvest_window_days")?,
        row.try_get("bud_break_doy")?,
        row.try_get("flowering_doy")?,
        row.try_get("harvest_start_doy")?,
        row.try_get("harvest_end_doy")?,
        opt_decimal_from_text(yield_text)?,
    )?;
    Ok(Variety {
        id: VarietyId::from(id),
        crop_id: CropId::from(crop_id),
        name: row.try_get("name")?,
        description: row.try_get("description")?,
        profile,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::{CropRepo, FamilyRepo, StrataRepo};
    use pomone_domain::{
        AnnualProfile, Crop, Family, Lifespan, PluriannualProfile, PruningSeason, Strata,
        VarietyProfile,
    };
    use rust_decimal_macros::dec;

    async fn setup_with_crop(lifespan: Lifespan) -> (SqliteRepository, CropId) {
        let repo = SqliteRepository::in_memory().await.unwrap();
        let f = Family::new("Test", None, None).unwrap();
        let s = Strata::new("Test", None, None, None, 0).unwrap();
        repo.family_create(&f).await.unwrap();
        repo.strata_create(&s).await.unwrap();
        let c = Crop::new(f.id, s.id, "Test", None, lifespan, PruningSeason::None).unwrap();
        repo.crop_create(&c).await.unwrap();
        (repo, c.id)
    }

    #[tokio::test]
    async fn annual_variety_roundtrip() {
        let (repo, crop_id) = setup_with_crop(Lifespan::Annual).await;
        let v = Variety::new(
            crop_id,
            Lifespan::Annual,
            "Marmande",
            None,
            VarietyProfile::Annual(AnnualProfile::new(Some(35), 70, 60).unwrap()),
        )
        .unwrap();
        repo.variety_create(&v).await.unwrap();
        let got = repo.variety_get(v.id).await.unwrap().unwrap();
        assert_eq!(got, v);
    }

    #[tokio::test]
    async fn pluriannual_variety_with_yield_roundtrip() {
        let lifespan = Lifespan::perennial(40, 3).unwrap();
        let (repo, crop_id) = setup_with_crop(lifespan).await;
        let v = Variety::new(
            crop_id,
            lifespan,
            "Reine des Reinettes",
            Some("pomme à couteau".into()),
            VarietyProfile::Pluriannual(
                PluriannualProfile::new(Some(80), Some(110), 220, 280, Some(dec!(15.5))).unwrap(),
            ),
        )
        .unwrap();
        repo.variety_create(&v).await.unwrap();
        let got = repo.variety_get(v.id).await.unwrap().unwrap();
        assert_eq!(got, v);
    }

    #[tokio::test]
    async fn variety_list_for_crop_filters_correctly() {
        let (repo, crop_a) = setup_with_crop(Lifespan::Annual).await;
        // A second crop in the same DB
        let f2 = repo.family_list().await.unwrap()[0].id;
        let s2 = repo.strata_list().await.unwrap()[0].id;
        let crop_b =
            Crop::new(f2, s2, "Other", None, Lifespan::Annual, PruningSeason::None).unwrap();
        repo.crop_create(&crop_b).await.unwrap();
        let crop_b_id = crop_b.id;

        for (name, crop_id) in [("V1", crop_a), ("V2", crop_a), ("V3", crop_b_id)] {
            let v = Variety::new(
                crop_id,
                Lifespan::Annual,
                name,
                None,
                VarietyProfile::Annual(AnnualProfile::new(None, 60, 30).unwrap()),
            )
            .unwrap();
            repo.variety_create(&v).await.unwrap();
        }

        let for_a = repo.variety_list_for_crop(crop_a).await.unwrap();
        assert_eq!(for_a.len(), 2);
        let for_b = repo.variety_list_for_crop(crop_b_id).await.unwrap();
        assert_eq!(for_b.len(), 1);
    }

    #[tokio::test]
    async fn deleting_crop_cascades_to_varieties() {
        let (repo, crop_id) = setup_with_crop(Lifespan::Annual).await;
        let v = Variety::new(
            crop_id,
            Lifespan::Annual,
            "Doomed",
            None,
            VarietyProfile::Annual(AnnualProfile::new(None, 60, 30).unwrap()),
        )
        .unwrap();
        repo.variety_create(&v).await.unwrap();
        repo.crop_delete(crop_id).await.unwrap();
        // ON DELETE CASCADE in the FK
        assert!(repo.variety_get(v.id).await.unwrap().is_none());
    }
}
