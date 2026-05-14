//! Presentation-layer helpers + write operations for the Strates screen.
//!
//! Strata define the vertical layering of an agroforestry / permaculture
//! design (canopy → shrub → herbaceous → ground cover → root). A fresh
//! database is seeded with the seven classic levels, but users edit and
//! extend them through this module's CRUD entry points.

use crate::error::{AppError, AppResult};
use crate::plantings_view::parse_id;
use pomone_db::Repository;
use pomone_domain::{Strata, StrataId};
use rust_decimal::Decimal;
use std::collections::HashSet;

/// Flat row for the Strates list, sorted by `sort_order`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StrataRow {
    pub id: String,
    pub name: String,
    pub description: String,
    /// Human-readable height range (e.g. `"6 – 40 m"`, `"≤ 0.5 m"`, `"—"`).
    pub height_label: String,
    pub sort_order: i32,
    /// True when at least one Crop references this stratum — the UI uses
    /// this to disable the Delete control (the domain has no FK-cascade).
    pub in_use: bool,
}

/// Input bag for creating a new stratum.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StrataInput {
    pub name: String,
    pub description: Option<String>,
    pub min_height_m: Option<Decimal>,
    pub max_height_m: Option<Decimal>,
    pub sort_order: i32,
}

fn fmt_height(min: Option<Decimal>, max: Option<Decimal>) -> String {
    match (min, max) {
        (Some(a), Some(b)) if a == b => format!("{} m", a.normalize()),
        (Some(a), Some(b)) => format!("{} – {} m", a.normalize(), b.normalize()),
        (Some(a), None) => format!("≥ {} m", a.normalize()),
        (None, Some(b)) => format!("≤ {} m", b.normalize()),
        (None, None) => "—".to_owned(),
    }
}

/// Return all strata sorted by their `sort_order` (then name as a tiebreak),
/// each tagged with `in_use` so the UI can disable destructive controls.
pub async fn list_strata_rows(repo: &dyn Repository) -> AppResult<Vec<StrataRow>> {
    let mut strata = repo.strata_list().await?;
    strata.sort_by(|a, b| {
        a.sort_order
            .cmp(&b.sort_order)
            .then_with(|| a.name.cmp(&b.name))
    });

    // Pre-compute which strata are referenced by at least one crop. A single
    // pass over the crops list keeps this linear; Pomone-sized catalogs make
    // this trivially cheap.
    let crops = repo.crop_list().await?;
    let used: HashSet<StrataId> = crops.iter().map(|c| c.strata_id).collect();

    let rows = strata
        .into_iter()
        .map(|s| StrataRow {
            id: s.id.to_string(),
            name: s.name,
            description: s.description.unwrap_or_default(),
            height_label: fmt_height(s.min_height_m, s.max_height_m),
            sort_order: s.sort_order,
            in_use: used.contains(&s.id),
        })
        .collect();
    Ok(rows)
}

/// Persist a new stratum. Domain `Strata::new` validates the name and the
/// height range (min ≤ max when both provided).
pub async fn create_strata(repo: &dyn Repository, input: StrataInput) -> AppResult<Strata> {
    let strata = Strata::new(
        input.name,
        input.description,
        input.min_height_m,
        input.max_height_m,
        input.sort_order,
    )?;
    repo.strata_create(&strata).await?;
    Ok(strata)
}

/// Delete a stratum after confirming no crop still references it. Returns
/// `Inconsistent` if the stratum is in use — the UI is expected to disable
/// the action in that case, but we guard server-side too.
pub async fn delete_strata(repo: &dyn Repository, id_str: &str) -> AppResult<()> {
    let id: StrataId = parse_id(id_str)?;
    let crops = repo.crop_list().await?;
    if crops.iter().any(|c| c.strata_id == id) {
        return Err(AppError::Inconsistent(
            "cannot delete a stratum still referenced by a crop".into(),
        ));
    }
    repo.strata_delete(id).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pomone_db::{seed_defaults, CropRepo, FamilyRepo, SqliteRepository, StrataRepo};
    use pomone_domain::{Crop, Family, Lifespan, PruningSeason};
    use rust_decimal_macros::dec;

    async fn fresh_repo() -> SqliteRepository {
        let repo = SqliteRepository::in_memory().await.unwrap();
        seed_defaults(&repo).await.unwrap();
        repo
    }

    #[tokio::test]
    async fn list_returns_seeded_strata_sorted_by_order() {
        let repo = fresh_repo().await;
        let rows = list_strata_rows(&repo).await.unwrap();
        assert!(!rows.is_empty());
        let orders: Vec<_> = rows.iter().map(|r| r.sort_order).collect();
        let mut sorted = orders.clone();
        sorted.sort_unstable();
        assert_eq!(orders, sorted);
        assert!(rows.iter().all(|r| !r.in_use));
    }

    #[tokio::test]
    async fn create_then_list_shows_new_stratum() {
        let repo = fresh_repo().await;
        let before = list_strata_rows(&repo).await.unwrap().len();
        create_strata(
            &repo,
            StrataInput {
                name: "Vigne".to_owned(),
                description: Some("lianes grimpantes".to_owned()),
                min_height_m: Some(dec!(2.0)),
                max_height_m: Some(dec!(10.0)),
                sort_order: 25,
            },
        )
        .await
        .unwrap();
        let after = list_strata_rows(&repo).await.unwrap();
        assert_eq!(after.len(), before + 1);
        let row = after.iter().find(|r| r.name == "Vigne").unwrap();
        assert_eq!(row.height_label, "2 – 10 m");
        assert!(!row.in_use);
    }

    #[tokio::test]
    async fn delete_removes_unused_stratum() {
        let repo = fresh_repo().await;
        let rows = list_strata_rows(&repo).await.unwrap();
        let target = &rows[0];
        delete_strata(&repo, &target.id).await.unwrap();
        let after = list_strata_rows(&repo).await.unwrap();
        assert!(after.iter().all(|r| r.id != target.id));
    }

    #[tokio::test]
    async fn delete_refuses_in_use_stratum() {
        let repo = fresh_repo().await;
        let strata = repo.strata_list().await.unwrap();
        let target = &strata[0];
        let family = Family::new("Test", None, None).unwrap();
        repo.family_create(&family).await.unwrap();
        let crop = Crop::new(
            family.id,
            target.id,
            "Démo",
            None,
            Lifespan::Annual,
            PruningSeason::None,
        )
        .unwrap();
        repo.crop_create(&crop).await.unwrap();

        let rows = list_strata_rows(&repo).await.unwrap();
        let row = rows.iter().find(|r| r.id == target.id.to_string()).unwrap();
        assert!(row.in_use);

        let err = delete_strata(&repo, &target.id.to_string())
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::Inconsistent(_)));
    }
}
