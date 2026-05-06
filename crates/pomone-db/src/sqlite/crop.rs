//! `CropRepo` implementation for SQLite.

use crate::error::{DbError, DbResult};
use crate::repository::CropRepo;
use crate::sqlite::codec::{decode_lifespan, decode_pruning, encode_lifespan, encode_pruning};
use crate::sqlite::SqliteRepository;
use async_trait::async_trait;
use pomone_domain::{Crop, CropId, FamilyId, StrataId};
use sqlx::Row;
use uuid::Uuid;

const COLUMNS: &str = "id, family_id, strata_id, name, latin_name, pruning_season, \
                       lifespan_kind, lifespan_years, productive_pattern, years_to_first_yield";

#[async_trait]
impl CropRepo for SqliteRepository {
    async fn crop_get(&self, id: CropId) -> DbResult<Option<Crop>> {
        let sql = format!("SELECT {COLUMNS} FROM crop WHERE id = ?1");
        let row = sqlx::query(&sql)
            .bind(id.as_uuid())
            .fetch_optional(&self.pool)
            .await?;
        row.map(row_to_crop).transpose()
    }

    async fn crop_list(&self) -> DbResult<Vec<Crop>> {
        let sql = format!("SELECT {COLUMNS} FROM crop ORDER BY name COLLATE NOCASE");
        let rows = sqlx::query(&sql).fetch_all(&self.pool).await?;
        rows.into_iter().map(row_to_crop).collect()
    }

    async fn crop_create(&self, c: &Crop) -> DbResult<()> {
        let life = encode_lifespan(c.lifespan);
        sqlx::query(
            "INSERT INTO crop (id, family_id, strata_id, name, latin_name, pruning_season, \
             lifespan_kind, lifespan_years, productive_pattern, years_to_first_yield) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
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
            "UPDATE crop SET family_id = ?2, strata_id = ?3, name = ?4, latin_name = ?5, \
             pruning_season = ?6, lifespan_kind = ?7, lifespan_years = ?8, \
             productive_pattern = ?9, years_to_first_yield = ?10 WHERE id = ?1",
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
        if result.rows_affected() == 0 {
            return Err(DbError::NotFound {
                kind: "crop",
                id: c.id.to_string(),
            });
        }
        Ok(())
    }

    async fn crop_delete(&self, id: CropId) -> DbResult<()> {
        let result = sqlx::query("DELETE FROM crop WHERE id = ?1")
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

fn row_to_crop(row: sqlx::sqlite::SqliteRow) -> DbResult<Crop> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::{FamilyRepo, StrataRepo};
    use pomone_domain::{Family, Lifespan, PruningSeason, Strata};

    async fn fresh_with_lookups() -> (SqliteRepository, FamilyId, StrataId) {
        let repo = SqliteRepository::in_memory().await.unwrap();
        let f = Family::new("Solanaceae", None, None).unwrap();
        let s = Strata::new("Herbacée", None, None, None, 40).unwrap();
        repo.family_create(&f).await.unwrap();
        repo.strata_create(&s).await.unwrap();
        (repo, f.id, s.id)
    }

    #[tokio::test]
    async fn annual_crop_roundtrip() {
        let (repo, fam, strata) = fresh_with_lookups().await;
        let c = Crop::new(
            fam,
            strata,
            "Tomate",
            Some("Solanum lycopersicum".into()),
            Lifespan::Annual,
            PruningSeason::None,
        )
        .unwrap();
        repo.crop_create(&c).await.unwrap();
        let got = repo.crop_get(c.id).await.unwrap().unwrap();
        assert_eq!(got, c);
    }

    #[tokio::test]
    async fn perennial_crop_roundtrip() {
        let (repo, fam, strata) = fresh_with_lookups().await;
        let c = Crop::new(
            fam,
            strata,
            "Pommier",
            Some("Malus domestica".into()),
            Lifespan::perennial(40, 3).unwrap(),
            PruningSeason::Winter,
        )
        .unwrap();
        repo.crop_create(&c).await.unwrap();
        let got = repo.crop_get(c.id).await.unwrap().unwrap();
        assert_eq!(got, c);
        assert!(got.lifespan.is_recurring());
    }

    #[tokio::test]
    async fn biennial_crop_roundtrip() {
        let (repo, fam, strata) = fresh_with_lookups().await;
        let c = Crop::new(
            fam,
            strata,
            "Carotte porte-graine",
            None,
            Lifespan::biennial().unwrap(),
            PruningSeason::None,
        )
        .unwrap();
        repo.crop_create(&c).await.unwrap();
        let got = repo.crop_get(c.id).await.unwrap().unwrap();
        assert_eq!(got, c);
    }

    #[tokio::test]
    async fn list_orders_alphabetically() {
        let (repo, fam, strata) = fresh_with_lookups().await;
        for n in ["Pommier", "Aubergine", "Carotte"] {
            repo.crop_create(
                &Crop::new(fam, strata, n, None, Lifespan::Annual, PruningSeason::None).unwrap(),
            )
            .await
            .unwrap();
        }
        let names: Vec<_> = repo
            .crop_list()
            .await
            .unwrap()
            .into_iter()
            .map(|c| c.name)
            .collect();
        assert_eq!(names, vec!["Aubergine", "Carotte", "Pommier"]);
    }

    #[tokio::test]
    async fn update_changes_lifespan() {
        let (repo, fam, strata) = fresh_with_lookups().await;
        let c = Crop::new(
            fam,
            strata,
            "Plant",
            None,
            Lifespan::Annual,
            PruningSeason::None,
        )
        .unwrap();
        repo.crop_create(&c).await.unwrap();
        let updated = Crop {
            lifespan: Lifespan::perennial(20, 2).unwrap(),
            pruning_season: PruningSeason::Both,
            ..c.clone()
        };
        repo.crop_update(&updated).await.unwrap();
        let got = repo.crop_get(c.id).await.unwrap().unwrap();
        assert!(got.lifespan.is_recurring());
        assert_eq!(got.pruning_season, PruningSeason::Both);
    }
}
