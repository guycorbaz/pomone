//! `FamilyRepo` implementation for SQLite.

use crate::error::{DbError, DbResult};
use crate::repository::FamilyRepo;
use crate::sqlite::SqliteRepository;
use async_trait::async_trait;
use pomone_domain::{Family, FamilyId};
use sqlx::Row;
use uuid::Uuid;

#[async_trait]
impl FamilyRepo for SqliteRepository {
    async fn family_get(&self, id: FamilyId) -> DbResult<Option<Family>> {
        let row = sqlx::query("SELECT id, name, latin_name, description FROM family WHERE id = ?1")
            .bind(id.as_uuid())
            .fetch_optional(&self.pool)
            .await?;
        row.map(row_to_family).transpose()
    }

    async fn family_list(&self) -> DbResult<Vec<Family>> {
        let rows = sqlx::query(
            "SELECT id, name, latin_name, description FROM family ORDER BY name COLLATE NOCASE",
        )
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(row_to_family).collect()
    }

    async fn family_create(&self, family: &Family) -> DbResult<()> {
        sqlx::query(
            "INSERT INTO family (id, name, latin_name, description) VALUES (?1, ?2, ?3, ?4)",
        )
        .bind(family.id.as_uuid())
        .bind(&family.name)
        .bind(family.latin_name.as_deref())
        .bind(family.description.as_deref())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn family_update(&self, family: &Family) -> DbResult<()> {
        let result = sqlx::query(
            "UPDATE family SET name = ?2, latin_name = ?3, description = ?4 WHERE id = ?1",
        )
        .bind(family.id.as_uuid())
        .bind(&family.name)
        .bind(family.latin_name.as_deref())
        .bind(family.description.as_deref())
        .execute(&self.pool)
        .await?;
        if result.rows_affected() == 0 {
            return Err(DbError::NotFound {
                kind: "family",
                id: family.id.to_string(),
            });
        }
        Ok(())
    }

    async fn family_delete(&self, id: FamilyId) -> DbResult<()> {
        let result = sqlx::query("DELETE FROM family WHERE id = ?1")
            .bind(id.as_uuid())
            .execute(&self.pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(DbError::NotFound {
                kind: "family",
                id: id.to_string(),
            });
        }
        Ok(())
    }
}

fn row_to_family(row: sqlx::sqlite::SqliteRow) -> DbResult<Family> {
    let id: Uuid = row.try_get("id")?;
    let name: String = row.try_get("name")?;
    let latin_name: Option<String> = row.try_get("latin_name")?;
    let description: Option<String> = row.try_get("description")?;
    Ok(Family {
        id: FamilyId::from(id),
        name,
        latin_name,
        description,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::FamilyRepo;
    use pomone_domain::Family;

    async fn fresh_repo() -> SqliteRepository {
        SqliteRepository::in_memory().await.unwrap()
    }

    #[tokio::test]
    async fn create_get_roundtrip() {
        let repo = fresh_repo().await;
        let f = Family::new("Solanaceae", Some("Solanaceae".into()), None).unwrap();
        repo.family_create(&f).await.unwrap();
        let got = repo.family_get(f.id).await.unwrap().unwrap();
        assert_eq!(got, f);
    }

    #[tokio::test]
    async fn get_missing_returns_none() {
        let repo = fresh_repo().await;
        let id = FamilyId::new();
        assert!(repo.family_get(id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn list_orders_by_name() {
        let repo = fresh_repo().await;
        for n in ["Rosaceae", "Asteraceae", "Solanaceae"] {
            repo.family_create(&Family::new(n, None, None).unwrap())
                .await
                .unwrap();
        }
        let names: Vec<_> = repo
            .family_list()
            .await
            .unwrap()
            .into_iter()
            .map(|f| f.name)
            .collect();
        assert_eq!(names, vec!["Asteraceae", "Rosaceae", "Solanaceae"]);
    }

    #[tokio::test]
    async fn update_changes_fields() {
        let repo = fresh_repo().await;
        let f = Family::new("Old", None, None).unwrap();
        repo.family_create(&f).await.unwrap();
        let updated = Family {
            name: "New".into(),
            description: Some("changed".into()),
            ..f.clone()
        };
        repo.family_update(&updated).await.unwrap();
        let got = repo.family_get(f.id).await.unwrap().unwrap();
        assert_eq!(got.name, "New");
        assert_eq!(got.description.as_deref(), Some("changed"));
    }

    #[tokio::test]
    async fn update_missing_id_returns_not_found() {
        let repo = fresh_repo().await;
        let f = Family::new("Ghost", None, None).unwrap();
        let err = repo.family_update(&f).await.unwrap_err();
        assert!(matches!(err, DbError::NotFound { kind: "family", .. }));
    }

    #[tokio::test]
    async fn delete_removes_row() {
        let repo = fresh_repo().await;
        let f = Family::new("Tmp", None, None).unwrap();
        repo.family_create(&f).await.unwrap();
        repo.family_delete(f.id).await.unwrap();
        assert!(repo.family_get(f.id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn delete_missing_returns_not_found() {
        let repo = fresh_repo().await;
        let err = repo.family_delete(FamilyId::new()).await.unwrap_err();
        assert!(matches!(err, DbError::NotFound { kind: "family", .. }));
    }
}
