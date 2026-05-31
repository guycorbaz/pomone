//! `LocationKindRepo` implementation for SQLite.

use crate::error::{DbError, DbResult};
use crate::repository::LocationKindRepo;
use crate::sqlite::SqliteRepository;
use async_trait::async_trait;
use pomone_domain::{LocationKind, LocationKindId};
use sqlx::Row;
use uuid::Uuid;

#[async_trait]
impl LocationKindRepo for SqliteRepository {
    async fn location_kind_get(&self, id: LocationKindId) -> DbResult<Option<LocationKind>> {
        let row =
            sqlx::query("SELECT id, name, description, covered FROM location_kind WHERE id = ?1")
                .bind(id.as_uuid())
                .fetch_optional(&self.pool)
                .await?;
        row.map(row_to_kind).transpose()
    }

    async fn location_kind_list(&self) -> DbResult<Vec<LocationKind>> {
        let rows = sqlx::query(
            "SELECT id, name, description, covered FROM location_kind ORDER BY name COLLATE NOCASE",
        )
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(row_to_kind).collect()
    }

    async fn location_kind_create(&self, k: &LocationKind) -> DbResult<()> {
        sqlx::query(
            "INSERT INTO location_kind (id, name, description, covered) VALUES (?1, ?2, ?3, ?4)",
        )
        .bind(k.id.as_uuid())
        .bind(&k.name)
        .bind(k.description.as_deref())
        .bind(k.covered)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn location_kind_update(&self, k: &LocationKind) -> DbResult<()> {
        let result = sqlx::query(
            "UPDATE location_kind SET name = ?2, description = ?3, covered = ?4 WHERE id = ?1",
        )
        .bind(k.id.as_uuid())
        .bind(&k.name)
        .bind(k.description.as_deref())
        .bind(k.covered)
        .execute(&self.pool)
        .await?;
        if result.rows_affected() == 0 {
            return Err(DbError::NotFound {
                kind: "location_kind",
                id: k.id.to_string(),
            });
        }
        Ok(())
    }

    async fn location_kind_delete(&self, id: LocationKindId) -> DbResult<()> {
        let result = sqlx::query("DELETE FROM location_kind WHERE id = ?1")
            .bind(id.as_uuid())
            .execute(&self.pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(DbError::NotFound {
                kind: "location_kind",
                id: id.to_string(),
            });
        }
        Ok(())
    }
}

fn row_to_kind(row: sqlx::sqlite::SqliteRow) -> DbResult<LocationKind> {
    let id: Uuid = row.try_get("id")?;
    Ok(LocationKind {
        id: LocationKindId::from(id),
        name: row.try_get("name")?,
        description: row.try_get("description")?,
        covered: row.try_get("covered")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn fresh() -> SqliteRepository {
        SqliteRepository::in_memory().await.unwrap()
    }

    #[tokio::test]
    async fn crud_roundtrip() {
        let repo = fresh().await;
        let k = LocationKind::new("Verger", Some("arbres fruitiers".into())).unwrap();
        repo.location_kind_create(&k).await.unwrap();
        assert_eq!(
            repo.location_kind_get(k.id).await.unwrap().as_ref(),
            Some(&k)
        );

        let updated = LocationKind {
            description: Some("changé".into()),
            ..k.clone()
        };
        repo.location_kind_update(&updated).await.unwrap();
        let got = repo.location_kind_get(k.id).await.unwrap().unwrap();
        assert_eq!(got.description.as_deref(), Some("changé"));

        repo.location_kind_delete(k.id).await.unwrap();
        assert!(repo.location_kind_get(k.id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn covered_roundtrips_through_create_and_update() {
        let repo = fresh().await;
        let k = LocationKind::new("Serre", None).unwrap().with_covered(true);
        repo.location_kind_create(&k).await.unwrap();
        assert!(repo.location_kind_get(k.id).await.unwrap().unwrap().covered);

        // Toggle it back off via update.
        let off = LocationKind {
            covered: false,
            ..k.clone()
        };
        repo.location_kind_update(&off).await.unwrap();
        assert!(!repo.location_kind_get(k.id).await.unwrap().unwrap().covered);
    }

    #[tokio::test]
    async fn list_orders_alphabetically() {
        let repo = fresh().await;
        for n in ["Verger", "Haie", "Parcelle"] {
            repo.location_kind_create(&LocationKind::new(n, None).unwrap())
                .await
                .unwrap();
        }
        let names: Vec<_> = repo
            .location_kind_list()
            .await
            .unwrap()
            .into_iter()
            .map(|k| k.name)
            .collect();
        assert_eq!(names, vec!["Haie", "Parcelle", "Verger"]);
    }
}
