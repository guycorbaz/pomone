//! Presentation-layer helpers for managing the botanical-families catalog.
//!
//! Pomone seeds eleven common families (each with a distinct starting colour)
//! on a fresh database; this module is the user-facing path for adding custom
//! families, renaming them, recolouring them, or deleting unused ones.
//!
//! The colour is user-configurable and drives per-family tinting of plantings
//! and the crop map — mirroring Qrop's `family.color`. It is validated as
//! `#RGB` / `#RRGGBB` by the domain (`Family::new_with_color`).

use crate::error::{AppError, AppResult};
use crate::plantings_view::parse_id;
use pomone_db::Repository;
use pomone_domain::{Family, FamilyId};
use std::collections::HashSet;

/// One row of the Families admin screen. `in_use` lets the UI grey out the
/// Delete button for families still referenced by a crop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FamilyAdminRow {
    pub id: String,
    pub name: String,
    pub latin_name: String,
    pub color: String,
    pub in_use: bool,
}

/// Pre-filled state for editing an existing family.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FamilyEditForm {
    pub id: String,
    pub name: String,
    pub latin_name: String,
    pub description: String,
    pub color: String,
}

/// List every family with an `in_use` flag computed from the crop list. Two
/// DB reads (families + crops) — fine at Pomone's scale.
pub async fn list_families_admin(repo: &dyn Repository) -> AppResult<Vec<FamilyAdminRow>> {
    let mut families = repo.family_list().await?;
    families.sort_by(|a, b| a.name.cmp(&b.name));
    let crops = repo.crop_list().await?;
    let used: HashSet<FamilyId> = crops.iter().map(|c| c.family_id).collect();
    Ok(families
        .into_iter()
        .map(|f| FamilyAdminRow {
            id: f.id.to_string(),
            in_use: used.contains(&f.id),
            latin_name: f.latin_name.unwrap_or_default(),
            name: f.name,
            color: f.color,
        })
        .collect())
}

/// Load one family into edit-form shape. `NotFound` if it was deleted in
/// another window.
pub async fn get_family_for_edit(repo: &dyn Repository, id_str: &str) -> AppResult<FamilyEditForm> {
    let id: FamilyId = parse_id(id_str)?;
    let f = repo
        .family_get(id)
        .await?
        .ok_or_else(|| AppError::NotFound {
            kind: "family",
            id: id_str.to_owned(),
        })?;
    Ok(FamilyEditForm {
        id: f.id.to_string(),
        name: f.name,
        latin_name: f.latin_name.unwrap_or_default(),
        description: f.description.unwrap_or_default(),
        color: f.color,
    })
}

/// Create a new family. Validates name (non-empty after trim) and colour
/// (`#RGB` / `#RRGGBB`) via `Family::new_with_color`.
pub async fn create_family(
    repo: &dyn Repository,
    name: &str,
    latin_name: &str,
    description: &str,
    color: &str,
) -> AppResult<String> {
    let f = Family::new_with_color(name, optional(latin_name), optional(description), color)?;
    repo.family_create(&f).await?;
    Ok(f.id.to_string())
}

/// Rename / re-describe / recolour an existing family, preserving its id.
/// Rebuilds through the domain constructor so the same invariants apply.
pub async fn update_family(
    repo: &dyn Repository,
    id_str: &str,
    name: &str,
    latin_name: &str,
    description: &str,
    color: &str,
) -> AppResult<()> {
    let id: FamilyId = parse_id(id_str)?;
    if repo.family_get(id).await?.is_none() {
        return Err(AppError::NotFound {
            kind: "family",
            id: id_str.to_owned(),
        });
    }
    let mut f = Family::new_with_color(name, optional(latin_name), optional(description), color)?;
    f.id = id;
    repo.family_update(&f).await?;
    Ok(())
}

/// Delete a family. `crop.family_id` is `ON DELETE RESTRICT`, so deleting a
/// family still referenced by a crop fails at the DB layer; we surface that
/// as an `Inconsistent` sentinel the UI re-keys to a friendly message.
pub async fn delete_family(repo: &dyn Repository, id_str: &str) -> AppResult<()> {
    let id: FamilyId = parse_id(id_str)?;
    match repo.family_delete(id).await {
        Ok(()) => Ok(()),
        Err(e) if e.is_foreign_key_violation() => {
            Err(AppError::Inconsistent("family_in_use".to_owned()))
        }
        Err(other) => Err(AppError::Db(other)),
    }
}

fn optional(s: &str) -> Option<String> {
    let t = s.trim();
    if t.is_empty() {
        None
    } else {
        Some(t.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pomone_db::{seed_defaults, CropRepo, SqliteRepository};

    async fn fresh() -> SqliteRepository {
        let repo = SqliteRepository::in_memory().await.unwrap();
        seed_defaults(&repo).await.unwrap();
        repo
    }

    #[tokio::test]
    async fn list_admin_returns_seeded_families_with_colors() {
        let repo = fresh().await;
        let rows = list_families_admin(&repo).await.unwrap();
        assert_eq!(rows.len(), 11);
        // Sorted by display name.
        assert!(rows.windows(2).all(|w| w[0].name <= w[1].name));
        // Every seeded family has a valid-looking hex colour.
        assert!(rows.iter().all(|r| r.color.starts_with('#')));
    }

    #[tokio::test]
    async fn create_then_edit_then_delete_roundtrip() {
        let repo = fresh().await;
        let id = create_family(
            &repo,
            "Lamiacées",
            "Lamiaceae",
            "menthe, basilic",
            "#5F9F8B",
        )
        .await
        .unwrap();
        let form = get_family_for_edit(&repo, &id).await.unwrap();
        assert_eq!(form.name, "Lamiacées");
        assert_eq!(form.latin_name, "Lamiaceae");
        assert_eq!(form.color, "#5F9F8B");

        update_family(
            &repo,
            &id,
            "Lamiacées",
            "Lamiaceae",
            "aromatiques",
            "#3C6E47",
        )
        .await
        .unwrap();
        let after = get_family_for_edit(&repo, &id).await.unwrap();
        assert_eq!(after.color, "#3C6E47");
        assert_eq!(after.description, "aromatiques");

        delete_family(&repo, &id).await.unwrap();
        assert!(get_family_for_edit(&repo, &id).await.is_err());
    }

    #[tokio::test]
    async fn create_rejects_invalid_color() {
        let repo = fresh().await;
        let err = create_family(&repo, "Foo", "", "", "notacolor")
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::Domain(_)));
    }

    #[tokio::test]
    async fn create_rejects_blank_name() {
        let repo = fresh().await;
        let err = create_family(&repo, "   ", "", "", "#3C6E47")
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::Domain(_)));
    }

    #[tokio::test]
    async fn delete_used_family_returns_in_use_sentinel() {
        let repo = fresh().await;
        let rows = list_families_admin(&repo).await.unwrap();
        // Every seeded family is referenced by at least one seeded crop? Not
        // necessarily — seed only creates families, not crops. Create a crop
        // under the first family to make it in-use.
        let fam_id = rows[0].id.clone();
        let fam_uuid: FamilyId = parse_id(&fam_id).unwrap();
        let crop = pomone_domain::Crop::new(
            fam_uuid,
            "Test crop",
            None,
            pomone_domain::Lifespan::Annual,
            pomone_domain::PruningSeason::None,
        )
        .unwrap();
        repo.crop_create(&crop).await.unwrap();

        let err = delete_family(&repo, &fam_id).await.unwrap_err();
        match err {
            AppError::Inconsistent(s) => assert_eq!(s, "family_in_use"),
            other => panic!("expected in-use sentinel, got {other:?}"),
        }
    }
}
