//! `LocationRepo` implementation for SQLite.
//!
//! Updates that would form a hierarchy cycle are detected by walking up from
//! the proposed parent and rejecting if we hit the location being updated.

use crate::codec::{decimal_from_text, decimal_to_text};
use crate::error::{DbError, DbResult};
use crate::repository::LocationRepo;
use crate::sqlite::SqliteRepository;
use async_trait::async_trait;
use pomone_domain::{Location, LocationId, LocationKindId};
use sqlx::Row;
use uuid::Uuid;

#[async_trait]
impl LocationRepo for SqliteRepository {
    async fn location_get(&self, id: LocationId) -> DbResult<Option<Location>> {
        let row = sqlx::query(
            "SELECT id, parent_id, kind_id, name, length_m, width_m, notes \
             FROM location WHERE id = ?1",
        )
        .bind(id.as_uuid())
        .fetch_optional(&self.pool)
        .await?;
        row.map(row_to_location).transpose()
    }

    async fn location_list(&self) -> DbResult<Vec<Location>> {
        let rows = sqlx::query(
            "SELECT id, parent_id, kind_id, name, length_m, width_m, notes \
             FROM location ORDER BY name COLLATE NOCASE",
        )
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(row_to_location).collect()
    }

    async fn location_list_roots(&self) -> DbResult<Vec<Location>> {
        let rows = sqlx::query(
            "SELECT id, parent_id, kind_id, name, length_m, width_m, notes \
             FROM location WHERE parent_id IS NULL ORDER BY name COLLATE NOCASE",
        )
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(row_to_location).collect()
    }

    async fn location_list_children(&self, parent_id: LocationId) -> DbResult<Vec<Location>> {
        let rows = sqlx::query(
            "SELECT id, parent_id, kind_id, name, length_m, width_m, notes \
             FROM location WHERE parent_id = ?1 ORDER BY name COLLATE NOCASE",
        )
        .bind(parent_id.as_uuid())
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(row_to_location).collect()
    }

    async fn location_create(&self, l: &Location) -> DbResult<()> {
        sqlx::query(
            "INSERT INTO location (id, parent_id, kind_id, name, length_m, width_m, notes) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        )
        .bind(l.id.as_uuid())
        .bind(l.parent_id.map(LocationId::as_uuid))
        .bind(l.kind_id.as_uuid())
        .bind(&l.name)
        .bind(decimal_to_text(l.length_m))
        .bind(decimal_to_text(l.width_m))
        .bind(l.notes.as_deref())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn location_update(&self, l: &Location) -> DbResult<()> {
        // Detect cycles before writing.
        if let Some(parent) = l.parent_id {
            check_no_cycle(&self.pool, l.id, parent).await?;
        }
        let result = sqlx::query(
            "UPDATE location SET parent_id = ?2, kind_id = ?3, name = ?4, \
             length_m = ?5, width_m = ?6, notes = ?7 WHERE id = ?1",
        )
        .bind(l.id.as_uuid())
        .bind(l.parent_id.map(LocationId::as_uuid))
        .bind(l.kind_id.as_uuid())
        .bind(&l.name)
        .bind(decimal_to_text(l.length_m))
        .bind(decimal_to_text(l.width_m))
        .bind(l.notes.as_deref())
        .execute(&self.pool)
        .await?;
        if result.rows_affected() == 0 {
            return Err(DbError::NotFound {
                kind: "location",
                id: l.id.to_string(),
            });
        }
        Ok(())
    }

    async fn location_delete(&self, id: LocationId) -> DbResult<()> {
        let result = sqlx::query("DELETE FROM location WHERE id = ?1")
            .bind(id.as_uuid())
            .execute(&self.pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(DbError::NotFound {
                kind: "location",
                id: id.to_string(),
            });
        }
        Ok(())
    }
}

async fn check_no_cycle(
    pool: &sqlx::SqlitePool,
    self_id: LocationId,
    proposed_parent: LocationId,
) -> DbResult<()> {
    // Walk up from `proposed_parent` and reject if we ever hit `self_id`.
    let mut current = Some(proposed_parent);
    let mut steps = 0_u32;
    while let Some(id) = current {
        if id == self_id {
            return Err(DbError::HierarchyCycle);
        }
        // Defensive cap: a sane hierarchy is < 1000 deep; anything more
        // is a sign of pre-existing corruption.
        steps += 1;
        if steps > 10_000 {
            return Err(DbError::HierarchyCycle);
        }
        let row = sqlx::query("SELECT parent_id FROM location WHERE id = ?1")
            .bind(id.as_uuid())
            .fetch_optional(pool)
            .await?;
        current = match row {
            Some(r) => r
                .try_get::<Option<Uuid>, _>("parent_id")?
                .map(LocationId::from),
            None => None, // proposed_parent doesn't exist; FK will reject the update
        };
    }
    Ok(())
}

fn row_to_location(row: sqlx::sqlite::SqliteRow) -> DbResult<Location> {
    let id: Uuid = row.try_get("id")?;
    let parent_id: Option<Uuid> = row.try_get("parent_id")?;
    let kind_id: Uuid = row.try_get("kind_id")?;
    let length_text: String = row.try_get("length_m")?;
    let width_text: String = row.try_get("width_m")?;
    Ok(Location {
        id: LocationId::from(id),
        parent_id: parent_id.map(LocationId::from),
        kind_id: LocationKindId::from(kind_id),
        name: row.try_get("name")?,
        length_m: decimal_from_text(&length_text)?,
        width_m: decimal_from_text(&width_text)?,
        notes: row.try_get("notes")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::LocationKindRepo;
    use pomone_domain::LocationKind;
    use rust_decimal_macros::dec;

    async fn fresh_with_kind() -> (SqliteRepository, LocationKindId) {
        let repo = SqliteRepository::in_memory().await.unwrap();
        let k = LocationKind::new("Parcelle", None).unwrap();
        repo.location_kind_create(&k).await.unwrap();
        (repo, k.id)
    }

    #[tokio::test]
    async fn create_and_get_root() {
        let (repo, kind) = fresh_with_kind().await;
        let l = Location::new(kind, "Ferme", dec!(250), dec!(200), None, None).unwrap();
        repo.location_create(&l).await.unwrap();
        let got = repo.location_get(l.id).await.unwrap().unwrap();
        assert_eq!(got, l);
        assert!(got.is_root());
        assert_eq!(got.area_m2(), dec!(50000));
    }

    #[tokio::test]
    async fn create_child_and_list_children() {
        let (repo, kind) = fresh_with_kind().await;
        let root = Location::new(kind, "Ferme", dec!(250), dec!(200), None, None).unwrap();
        repo.location_create(&root).await.unwrap();
        let p1 = Location::new(kind, "P1", dec!(40), dec!(50), Some(root.id), None).unwrap();
        let p2 = Location::new(kind, "P2", dec!(50), dec!(60), Some(root.id), None).unwrap();
        repo.location_create(&p1).await.unwrap();
        repo.location_create(&p2).await.unwrap();

        let roots = repo.location_list_roots().await.unwrap();
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0].id, root.id);

        let children = repo.location_list_children(root.id).await.unwrap();
        assert_eq!(children.len(), 2);
    }

    #[tokio::test]
    async fn cycle_detection_rejects_self_loop() {
        let (repo, kind) = fresh_with_kind().await;
        let l = Location::new(kind, "L", dec!(10), dec!(10), None, None).unwrap();
        repo.location_create(&l).await.unwrap();
        let bad = Location {
            parent_id: Some(l.id),
            ..l.clone()
        };
        let err = repo.location_update(&bad).await.unwrap_err();
        assert!(matches!(err, DbError::HierarchyCycle));
    }

    #[tokio::test]
    async fn cycle_detection_rejects_indirect_loop() {
        let (repo, kind) = fresh_with_kind().await;
        // Build a → b → c, then try to set a.parent = c (would form a cycle)
        let a = Location::new(kind, "a", dec!(5), dec!(2), None, None).unwrap();
        let b = Location::new(kind, "b", dec!(5), dec!(2), Some(a.id), None).unwrap();
        let c = Location::new(kind, "c", dec!(5), dec!(2), Some(b.id), None).unwrap();
        for l in [&a, &b, &c] {
            repo.location_create(l).await.unwrap();
        }
        let bad = Location {
            parent_id: Some(c.id),
            ..a.clone()
        };
        let err = repo.location_update(&bad).await.unwrap_err();
        assert!(matches!(err, DbError::HierarchyCycle));
    }

    #[tokio::test]
    async fn dimensions_decimal_roundtrip() {
        let (repo, kind) = fresh_with_kind().await;
        let l = Location::new(kind, "P", dec!(25.123), dec!(0.85), None, None).unwrap();
        repo.location_create(&l).await.unwrap();
        let got = repo.location_get(l.id).await.unwrap().unwrap();
        assert_eq!(got.length_m, dec!(25.123));
        assert_eq!(got.width_m, dec!(0.85));
        // Derived area: 25.123 * 0.85 = 21.35455
        assert_eq!(got.area_m2(), dec!(21.35455));
    }

    #[tokio::test]
    async fn delete_with_children_blocked_by_fk() {
        let (repo, kind) = fresh_with_kind().await;
        let parent = Location::new(kind, "P", dec!(5), dec!(2), None, None).unwrap();
        let child = Location::new(kind, "C", dec!(2.5), dec!(2), Some(parent.id), None).unwrap();
        repo.location_create(&parent).await.unwrap();
        repo.location_create(&child).await.unwrap();
        // ON DELETE RESTRICT on parent_id → deletion of parent must fail
        let err = repo.location_delete(parent.id).await.unwrap_err();
        assert!(matches!(err, DbError::Sqlx(_)));
    }
}
