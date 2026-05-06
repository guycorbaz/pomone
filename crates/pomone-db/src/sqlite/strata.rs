//! `StrataRepo` implementation for SQLite.

use crate::error::{DbError, DbResult};
use crate::repository::StrataRepo;
use crate::sqlite::codec::{opt_decimal_from_text, opt_decimal_to_text};
use crate::sqlite::SqliteRepository;
use async_trait::async_trait;
use pomone_domain::{Strata, StrataId};
use sqlx::Row;
use uuid::Uuid;

#[async_trait]
impl StrataRepo for SqliteRepository {
    async fn strata_get(&self, id: StrataId) -> DbResult<Option<Strata>> {
        let row = sqlx::query(
            "SELECT id, name, description, min_height_m, max_height_m, sort_order \
             FROM strata WHERE id = ?1",
        )
        .bind(id.as_uuid())
        .fetch_optional(&self.pool)
        .await?;
        row.map(row_to_strata).transpose()
    }

    async fn strata_list(&self) -> DbResult<Vec<Strata>> {
        let rows = sqlx::query(
            "SELECT id, name, description, min_height_m, max_height_m, sort_order \
             FROM strata ORDER BY sort_order, name COLLATE NOCASE",
        )
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(row_to_strata).collect()
    }

    async fn strata_create(&self, s: &Strata) -> DbResult<()> {
        sqlx::query(
            "INSERT INTO strata (id, name, description, min_height_m, max_height_m, sort_order) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )
        .bind(s.id.as_uuid())
        .bind(&s.name)
        .bind(s.description.as_deref())
        .bind(opt_decimal_to_text(s.min_height_m))
        .bind(opt_decimal_to_text(s.max_height_m))
        .bind(s.sort_order)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn strata_update(&self, s: &Strata) -> DbResult<()> {
        let result = sqlx::query(
            "UPDATE strata SET name = ?2, description = ?3, min_height_m = ?4, \
             max_height_m = ?5, sort_order = ?6 WHERE id = ?1",
        )
        .bind(s.id.as_uuid())
        .bind(&s.name)
        .bind(s.description.as_deref())
        .bind(opt_decimal_to_text(s.min_height_m))
        .bind(opt_decimal_to_text(s.max_height_m))
        .bind(s.sort_order)
        .execute(&self.pool)
        .await?;
        if result.rows_affected() == 0 {
            return Err(DbError::NotFound {
                kind: "strata",
                id: s.id.to_string(),
            });
        }
        Ok(())
    }

    async fn strata_delete(&self, id: StrataId) -> DbResult<()> {
        let result = sqlx::query("DELETE FROM strata WHERE id = ?1")
            .bind(id.as_uuid())
            .execute(&self.pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(DbError::NotFound {
                kind: "strata",
                id: id.to_string(),
            });
        }
        Ok(())
    }
}

fn row_to_strata(row: sqlx::sqlite::SqliteRow) -> DbResult<Strata> {
    let id: Uuid = row.try_get("id")?;
    let min_text: Option<String> = row.try_get("min_height_m")?;
    let max_text: Option<String> = row.try_get("max_height_m")?;
    Ok(Strata {
        id: StrataId::from(id),
        name: row.try_get("name")?,
        description: row.try_get("description")?,
        min_height_m: opt_decimal_from_text(min_text)?,
        max_height_m: opt_decimal_from_text(max_text)?,
        sort_order: row.try_get("sort_order")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    async fn fresh() -> SqliteRepository {
        SqliteRepository::in_memory().await.unwrap()
    }

    #[tokio::test]
    async fn create_get_with_decimals() {
        let repo = fresh().await;
        let s = Strata::new("Canopée", None, Some(dec!(6.0)), Some(dec!(40.0)), 10).unwrap();
        repo.strata_create(&s).await.unwrap();
        let got = repo.strata_get(s.id).await.unwrap().unwrap();
        assert_eq!(got, s);
    }

    #[tokio::test]
    async fn list_ordered_by_sort_then_name() {
        let repo = fresh().await;
        for (n, ord) in [("B", 1), ("A", 1), ("C", 0)] {
            repo.strata_create(&Strata::new(n, None, None, None, ord).unwrap())
                .await
                .unwrap();
        }
        let names: Vec<_> = repo
            .strata_list()
            .await
            .unwrap()
            .into_iter()
            .map(|s| s.name)
            .collect();
        // sort_order 0 comes first (C), then sort_order 1 by name (A, B)
        assert_eq!(names, vec!["C", "A", "B"]);
    }

    #[tokio::test]
    async fn update_changes_heights() {
        let repo = fresh().await;
        let s = Strata::new("Test", None, Some(dec!(1)), Some(dec!(2)), 0).unwrap();
        repo.strata_create(&s).await.unwrap();
        let updated = Strata {
            min_height_m: Some(dec!(0.5)),
            max_height_m: Some(dec!(3.5)),
            ..s.clone()
        };
        repo.strata_update(&updated).await.unwrap();
        let got = repo.strata_get(s.id).await.unwrap().unwrap();
        assert_eq!(got.min_height_m, Some(dec!(0.5)));
        assert_eq!(got.max_height_m, Some(dec!(3.5)));
    }

    #[tokio::test]
    async fn delete_works() {
        let repo = fresh().await;
        let s = Strata::new("Tmp", None, None, None, 0).unwrap();
        repo.strata_create(&s).await.unwrap();
        repo.strata_delete(s.id).await.unwrap();
        assert!(repo.strata_get(s.id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn delete_missing_not_found() {
        let repo = fresh().await;
        let err = repo.strata_delete(StrataId::new()).await.unwrap_err();
        assert!(matches!(err, DbError::NotFound { kind: "strata", .. }));
    }
}
