//! `FamilyRepo` implementation for MariaDB.

use crate::error::{DbError, DbResult};
use crate::mariadb::MariaDbRepository;
use crate::repository::FamilyRepo;
use async_trait::async_trait;
use pomone_domain::{Family, FamilyId};
use sqlx::Row;
use uuid::Uuid;

#[async_trait]
impl FamilyRepo for MariaDbRepository {
    async fn family_get(&self, id: FamilyId) -> DbResult<Option<Family>> {
        let row =
            sqlx::query("SELECT id, name, latin_name, description, color FROM family WHERE id = ?")
                .bind(id.as_uuid())
                .fetch_optional(&self.pool)
                .await?;
        row.map(row_to_family).transpose()
    }

    async fn family_list(&self) -> DbResult<Vec<Family>> {
        let rows = sqlx::query(
            "SELECT id, name, latin_name, description, color FROM family ORDER BY name",
        )
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(row_to_family).collect()
    }

    async fn family_create(&self, family: &Family) -> DbResult<()> {
        sqlx::query(
            "INSERT INTO family (id, name, latin_name, description, color) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(family.id.as_uuid())
        .bind(&family.name)
        .bind(family.latin_name.as_deref())
        .bind(family.description.as_deref())
        .bind(&family.color)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn family_update(&self, family: &Family) -> DbResult<()> {
        let result = sqlx::query(
            "UPDATE family SET name = ?, latin_name = ?, description = ?, color = ? WHERE id = ?",
        )
        .bind(&family.name)
        .bind(family.latin_name.as_deref())
        .bind(family.description.as_deref())
        .bind(&family.color)
        .bind(family.id.as_uuid())
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
        let result = sqlx::query("DELETE FROM family WHERE id = ?")
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

fn row_to_family(row: sqlx::mysql::MySqlRow) -> DbResult<Family> {
    let id: Uuid = row.try_get("id")?;
    Ok(Family {
        id: FamilyId::from(id),
        name: row.try_get("name")?,
        latin_name: row.try_get("latin_name")?,
        description: row.try_get("description")?,
        color: row.try_get("color")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mariadb::test_helpers::fresh_repo;

    #[tokio::test]
    #[ignore = "requires Docker for MariaDB testcontainer"]
    async fn create_get_roundtrip() {
        let repo = fresh_repo().await;
        let f = Family::new("Solanaceae", Some("Solanaceae".into()), None).unwrap();
        repo.family_create(&f).await.unwrap();
        let got = repo.family_get(f.id).await.unwrap().unwrap();
        assert_eq!(got, f);
    }

    #[tokio::test]
    #[ignore = "requires Docker for MariaDB testcontainer"]
    async fn list_orders_alphabetically() {
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
    #[ignore = "requires Docker for MariaDB testcontainer"]
    async fn update_and_delete() {
        let repo = fresh_repo().await;
        let f = Family::new("Old", None, None).unwrap();
        repo.family_create(&f).await.unwrap();
        let updated = Family {
            name: "New".into(),
            color: "#123456".into(),
            ..f.clone()
        };
        repo.family_update(&updated).await.unwrap();
        let got = repo.family_get(f.id).await.unwrap().unwrap();
        assert_eq!(got.name, "New");
        assert_eq!(got.color, "#123456");

        repo.family_delete(f.id).await.unwrap();
        assert!(repo.family_get(f.id).await.unwrap().is_none());

        let err = repo.family_delete(FamilyId::new()).await.unwrap_err();
        assert!(matches!(err, DbError::NotFound { kind: "family", .. }));
    }
}
