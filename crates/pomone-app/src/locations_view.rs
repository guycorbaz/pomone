//! Presentation-layer helpers for the Locations screen.
//!
//! Same shape as the other `*_view` modules: plain-string DTOs and parsers,
//! so the Slint UI never sees `Uuid` / `Decimal`. The defining detail here
//! is the hierarchy: `list_locations_tree` returns rows in pre-order with a
//! `depth` field the UI uses to indent visually.

use crate::error::{AppError, AppResult};
use pomone_db::Repository;
use pomone_domain::{Location, LocationId, LocationKindId};
use rust_decimal::Decimal;
use std::collections::HashMap;

/// Hard cap on the depth `list_locations_tree` will walk. Real-world farms
/// nest 3–4 levels at most; anything past 50 indicates a corrupted DB and
/// we bail out rather than risk an infinite loop.
const MAX_TREE_DEPTH: u32 = 50;

/// One row of the Locations list, pre-flattened from the hierarchy.
///
/// `depth` is the number of ancestors (0 for roots). The UI renders an
/// indent proportional to it. `full_path` is a `/`-separated label string
/// suitable for the parent dropdown. `dimensions_label` shows
/// `"L × W = area m²"`; `area_label` keeps the legacy single-figure form
/// for compact lists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocationListItem {
    pub id: String,
    pub name: String,
    pub kind_label: String,
    pub area_label: String,
    pub dimensions_label: String,
    pub parent_label: String,
    pub full_path: String,
    pub depth: u32,
    /// True when the location has child locations or is used by a planting —
    /// the UI disables Delete (both FKs are `ON DELETE RESTRICT`).
    pub in_use: bool,
}

/// One option for the LocationKind dropdown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocationKindOption {
    pub id: String,
    pub label: String,
}

/// One option for the "parent location" dropdown. The first slot is always
/// the synthetic "(none) / root" choice with an empty `id`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParentLocationOption {
    /// Empty string means "no parent — create as root".
    pub id: String,
    pub label: String,
}

/// Return all locations as a flat, depth-tagged list in pre-order. Siblings
/// are sorted alphabetically.
pub async fn list_locations_tree(repo: &dyn Repository) -> AppResult<Vec<LocationListItem>> {
    let locations = repo.location_list().await?;
    let kinds = repo.location_kind_list().await?;
    let kind_by_id: HashMap<_, _> = kinds.iter().map(|k| (k.id, k)).collect();
    let by_id: HashMap<LocationId, &Location> = locations.iter().map(|l| (l.id, l)).collect();

    let mut children_by_parent: HashMap<Option<LocationId>, Vec<&Location>> = HashMap::new();
    for l in &locations {
        children_by_parent.entry(l.parent_id).or_default().push(l);
    }
    for kids in children_by_parent.values_mut() {
        kids.sort_by(|a, b| a.name.cmp(&b.name));
    }

    // Locations referenced by a planting can't be deleted (FK RESTRICT).
    let planted: std::collections::HashSet<LocationId> = repo
        .planting_list()
        .await?
        .iter()
        .map(|p| p.location_id)
        .collect();

    let mut out = Vec::with_capacity(locations.len());
    walk(
        None,
        0,
        &children_by_parent,
        &kind_by_id,
        &by_id,
        &planted,
        &mut out,
    );
    Ok(out)
}

#[allow(clippy::too_many_arguments)]
fn walk(
    parent: Option<LocationId>,
    depth: u32,
    children_by_parent: &HashMap<Option<LocationId>, Vec<&Location>>,
    kind_by_id: &HashMap<LocationKindId, &pomone_domain::LocationKind>,
    by_id: &HashMap<LocationId, &Location>,
    planted: &std::collections::HashSet<LocationId>,
    out: &mut Vec<LocationListItem>,
) {
    if depth > MAX_TREE_DEPTH {
        tracing::warn!(depth, "location tree depth cap reached — cycle suspected");
        return;
    }
    let Some(kids) = children_by_parent.get(&parent) else {
        return;
    };
    for child in kids {
        let parent_label = parent
            .and_then(|p| by_id.get(&p))
            .map_or_else(String::new, |p| p.name.clone());
        out.push(LocationListItem {
            id: child.id.to_string(),
            name: child.name.clone(),
            kind_label: kind_by_id
                .get(&child.kind_id)
                .map_or_else(|| "?".to_owned(), |k| k.name.clone()),
            area_label: format_area(child.area_m2()),
            dimensions_label: format_dimensions(child.length_m, child.width_m, child.area_m2()),
            parent_label,
            full_path: build_full_path(child, by_id),
            depth,
            in_use: children_by_parent.contains_key(&Some(child.id)) || planted.contains(&child.id),
        });
        walk(
            Some(child.id),
            depth + 1,
            children_by_parent,
            kind_by_id,
            by_id,
            planted,
            out,
        );
    }
}

fn build_full_path(loc: &Location, by_id: &HashMap<LocationId, &Location>) -> String {
    let mut segments: Vec<&str> = vec![loc.name.as_str()];
    let mut current = loc.parent_id;
    let mut hops = 0u32;
    while let Some(parent_id) = current {
        if hops > MAX_TREE_DEPTH {
            break;
        }
        let Some(parent) = by_id.get(&parent_id) else {
            break;
        };
        segments.push(parent.name.as_str());
        current = parent.parent_id;
        hops += 1;
    }
    segments.reverse();
    segments.join(" / ")
}

fn format_area(area: Decimal) -> String {
    let s = area.normalize().to_string();
    format!("{s} m²")
}

fn format_dimensions(length: Decimal, width: Decimal, area: Decimal) -> String {
    let l = length.normalize();
    let w = width.normalize();
    let a = area.normalize();
    format!("{l} × {w} m = {a} m²")
}

/// `LocationKind` options as a dropdown, sorted by name.
pub async fn list_location_kind_options(
    repo: &dyn Repository,
) -> AppResult<Vec<LocationKindOption>> {
    let mut kinds = repo.location_kind_list().await?;
    kinds.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(kinds
        .into_iter()
        .map(|k| LocationKindOption {
            id: k.id.to_string(),
            label: k.name,
        })
        .collect())
}

/// Parent dropdown options. First slot is always the "(no parent)" choice
/// with `id == ""`. Following entries are every existing location's
/// `full_path` label, in pre-order (sibling alpha sort), so the user picks
/// the breadcrumb they see in the list.
///
/// `none_label` is whatever the UI wants to show for the root option (e.g.
/// "(aucun)" / "(none)"), kept out of the locale layer so the caller can
/// translate it.
pub async fn list_parent_options(
    repo: &dyn Repository,
    none_label: &str,
) -> AppResult<Vec<ParentLocationOption>> {
    let tree = list_locations_tree(repo).await?;
    let mut out = Vec::with_capacity(tree.len() + 1);
    out.push(ParentLocationOption {
        id: String::new(),
        label: none_label.to_owned(),
    });
    for item in tree {
        out.push(ParentLocationOption {
            id: item.id,
            label: item.full_path,
        });
    }
    Ok(out)
}

/// Validation-aware payload for `create_location`.
#[derive(Debug, Clone)]
pub struct LocationInput {
    pub kind_id_str: String,
    pub name: String,
    pub length_m: Decimal,
    pub width_m: Decimal,
    /// Empty string means "create as root".
    pub parent_id_str: String,
    pub notes: Option<String>,
}

/// Create a new `Location` and persist it. The `Repository` enforces parent
/// existence and cycle prevention; we only translate the UI strings into
/// typed IDs and the right `Option<LocationId>`.
pub async fn create_location(repo: &dyn Repository, input: LocationInput) -> AppResult<Location> {
    let kind_id: LocationKindId = crate::plantings_view::parse_id(&input.kind_id_str)?;
    let parent_id = if input.parent_id_str.trim().is_empty() {
        None
    } else {
        let id: LocationId = crate::plantings_view::parse_id(input.parent_id_str.trim())?;
        Some(id)
    };
    let location = Location::new(
        kind_id,
        input.name,
        input.length_m,
        input.width_m,
        parent_id,
        input.notes,
    )?;
    repo.location_create(&location)
        .await
        .map_err(AppError::from)?;
    Ok(location)
}

/// Flattened location fields for prefilling the edit form. Dimensions are
/// stringified for the text inputs; `parent_id_str` is empty for a root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocationEditForm {
    pub id: String,
    pub kind_id_str: String,
    pub name: String,
    pub length: String,
    pub width: String,
    pub parent_id_str: String,
    pub notes: String,
}

/// Load one location and flatten it for the edit form. `NotFound` if unknown.
pub async fn get_location_for_edit(
    repo: &dyn Repository,
    id_str: &str,
) -> AppResult<LocationEditForm> {
    let id: LocationId = crate::plantings_view::parse_id(id_str)?;
    let loc = repo
        .location_get(id)
        .await?
        .ok_or_else(|| AppError::NotFound {
            kind: "location",
            id: id_str.to_owned(),
        })?;
    Ok(LocationEditForm {
        id: loc.id.to_string(),
        kind_id_str: loc.kind_id.to_string(),
        name: loc.name,
        length: loc.length_m.normalize().to_string(),
        width: loc.width_m.normalize().to_string(),
        parent_id_str: loc.parent_id.map(|p| p.to_string()).unwrap_or_default(),
        notes: loc.notes.unwrap_or_default(),
    })
}

/// Update an existing location from the same `LocationInput` the create form
/// produces. Reuses `Location::new` for validation, keeps the original id, and
/// surfaces a hierarchy cycle (reparenting under a descendant) as the
/// `Inconsistent("location_cycle")` sentinel.
pub async fn update_location(
    repo: &dyn Repository,
    id_str: &str,
    input: LocationInput,
) -> AppResult<()> {
    let id: LocationId = crate::plantings_view::parse_id(id_str)?;
    if repo.location_get(id).await?.is_none() {
        return Err(AppError::NotFound {
            kind: "location",
            id: id_str.to_owned(),
        });
    }
    let kind_id: LocationKindId = crate::plantings_view::parse_id(&input.kind_id_str)?;
    let parent_id = if input.parent_id_str.trim().is_empty() {
        None
    } else {
        Some(crate::plantings_view::parse_id::<LocationId>(
            input.parent_id_str.trim(),
        )?)
    };
    let mut location = Location::new(
        kind_id,
        input.name,
        input.length_m,
        input.width_m,
        parent_id,
        input.notes,
    )?;
    location.id = id;
    match repo.location_update(&location).await {
        Ok(()) => Ok(()),
        Err(pomone_db::DbError::HierarchyCycle) => {
            Err(AppError::Inconsistent("location_cycle".to_owned()))
        }
        Err(other) => Err(AppError::Db(other)),
    }
}

/// Delete a location. Blocked (FK `ON DELETE RESTRICT`) when it has child
/// locations or is used by a planting — surfaced as the
/// `Inconsistent("location_in_use")` sentinel the UI re-keys to a localized
/// message (same convention as crops / varieties).
pub async fn delete_location(repo: &dyn Repository, id_str: &str) -> AppResult<()> {
    let id: LocationId = crate::plantings_view::parse_id(id_str)?;
    match repo.location_delete(id).await {
        Ok(()) => Ok(()),
        Err(e) if e.is_foreign_key_violation() => {
            Err(AppError::Inconsistent("location_in_use".to_owned()))
        }
        Err(other) => Err(AppError::Db(other)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::seed_test_data;
    use pomone_db::{seed_defaults, SqliteRepository};
    use rust_decimal_macros::dec;

    async fn fresh_repo() -> SqliteRepository {
        let repo = SqliteRepository::in_memory().await.unwrap();
        seed_defaults(&repo).await.unwrap();
        seed_test_data(&repo).await.unwrap();
        repo
    }

    #[tokio::test]
    async fn list_tree_returns_seeded_parent_then_child() {
        let repo = fresh_repo().await;
        let tree = list_locations_tree(&repo).await.unwrap();
        assert_eq!(tree.len(), 2);
        assert_eq!(tree[0].depth, 0);
        assert_eq!(tree[0].name, "Jardin Pomone");
        assert!(tree[0].parent_label.is_empty());
        assert_eq!(tree[0].full_path, "Jardin Pomone");
        assert_eq!(tree[1].depth, 1);
        assert_eq!(tree[1].name, "Planche A");
        assert_eq!(tree[1].parent_label, "Jardin Pomone");
        assert_eq!(tree[1].full_path, "Jardin Pomone / Planche A");
    }

    #[tokio::test]
    async fn parent_options_include_none_first() {
        let repo = fresh_repo().await;
        let opts = list_parent_options(&repo, "(aucun)").await.unwrap();
        assert_eq!(opts[0].id, "");
        assert_eq!(opts[0].label, "(aucun)");
        assert!(opts.iter().any(|o| o.label == "Jardin Pomone"));
        assert!(opts.iter().any(|o| o.label == "Jardin Pomone / Planche A"));
    }

    #[tokio::test]
    async fn kind_options_include_seeded_kinds() {
        let repo = fresh_repo().await;
        let opts = list_location_kind_options(&repo).await.unwrap();
        assert!(opts.iter().any(|k| k.label == "Parcelle"));
        assert!(opts.iter().any(|k| k.label == "Planche"));
        assert!(opts.iter().any(|k| k.label == "Verger"));
    }

    #[tokio::test]
    async fn create_location_at_root_persists() {
        let repo = fresh_repo().await;
        let kinds = list_location_kind_options(&repo).await.unwrap();
        let verger = kinds
            .iter()
            .find(|k| k.label == "Verger")
            .unwrap()
            .id
            .clone();
        let loc = create_location(
            &repo,
            LocationInput {
                kind_id_str: verger,
                name: "Verger Sud".to_owned(),
                length_m: dec!(25),
                width_m: dec!(20),
                parent_id_str: String::new(),
                notes: None,
            },
        )
        .await
        .unwrap();
        assert!(loc.parent_id.is_none());
        let tree = list_locations_tree(&repo).await.unwrap();
        assert!(tree.iter().any(|l| l.name == "Verger Sud" && l.depth == 0));
    }

    #[tokio::test]
    async fn create_location_under_parent_persists() {
        let repo = fresh_repo().await;
        let kinds = list_location_kind_options(&repo).await.unwrap();
        let planche = kinds
            .iter()
            .find(|k| k.label == "Planche")
            .unwrap()
            .id
            .clone();
        let parents = list_parent_options(&repo, "(aucun)").await.unwrap();
        let jardin_id = parents
            .iter()
            .find(|p| p.label == "Jardin Pomone")
            .unwrap()
            .id
            .clone();

        let loc = create_location(
            &repo,
            LocationInput {
                kind_id_str: planche,
                name: "Planche B".to_owned(),
                length_m: dec!(25),
                width_m: dec!(1),
                parent_id_str: jardin_id,
                notes: Some("orientation est-ouest".to_owned()),
            },
        )
        .await
        .unwrap();
        assert!(loc.parent_id.is_some());
        let tree = list_locations_tree(&repo).await.unwrap();
        let row = tree.iter().find(|l| l.name == "Planche B").unwrap();
        assert_eq!(row.depth, 1);
        assert_eq!(row.parent_label, "Jardin Pomone");
        assert_eq!(row.full_path, "Jardin Pomone / Planche B");
    }

    #[tokio::test]
    async fn create_rejects_empty_name() {
        let repo = fresh_repo().await;
        let kinds = list_location_kind_options(&repo).await.unwrap();
        let err = create_location(
            &repo,
            LocationInput {
                kind_id_str: kinds[0].id.clone(),
                name: "   ".to_owned(),
                length_m: dec!(5),
                width_m: dec!(2),
                parent_id_str: String::new(),
                notes: None,
            },
        )
        .await
        .unwrap_err();
        assert!(matches!(err, AppError::Domain(_)));
    }

    #[tokio::test]
    async fn create_rejects_invalid_kind() {
        let repo = fresh_repo().await;
        let err = create_location(
            &repo,
            LocationInput {
                kind_id_str: "not-a-uuid".to_owned(),
                name: "X".to_owned(),
                length_m: dec!(5),
                width_m: dec!(2),
                parent_id_str: String::new(),
                notes: None,
            },
        )
        .await
        .unwrap_err();
        assert!(matches!(err, AppError::Inconsistent(_)));
    }

    #[tokio::test]
    async fn deep_chain_renders_with_breadcrumb() {
        let repo = SqliteRepository::in_memory().await.unwrap();
        seed_defaults(&repo).await.unwrap();
        // Build a 4-level chain: A → B → C → D
        let kinds = list_location_kind_options(&repo).await.unwrap();
        let parcelle = kinds
            .iter()
            .find(|k| k.label == "Parcelle")
            .unwrap()
            .id
            .clone();
        let mut last_id = String::new();
        for name in ["A", "B", "C", "D"] {
            let loc = create_location(
                &repo,
                LocationInput {
                    kind_id_str: parcelle.clone(),
                    name: name.to_owned(),
                    length_m: dec!(5),
                    width_m: dec!(2),
                    parent_id_str: last_id.clone(),
                    notes: None,
                },
            )
            .await
            .unwrap();
            last_id = loc.id.to_string();
        }
        let tree = list_locations_tree(&repo).await.unwrap();
        assert_eq!(tree.len(), 4);
        let leaf = tree.iter().find(|l| l.name == "D").unwrap();
        assert_eq!(leaf.depth, 3);
        assert_eq!(leaf.full_path, "A / B / C / D");
    }

    // ----- Delete guards (issue #86 follow-up) ---------------------------

    #[tokio::test]
    async fn update_location_changes_name() {
        let repo = fresh_repo().await;
        let planche = list_locations_tree(&repo)
            .await
            .unwrap()
            .into_iter()
            .find(|l| l.name == "Planche A")
            .unwrap();
        let form = get_location_for_edit(&repo, &planche.id).await.unwrap();
        update_location(
            &repo,
            &planche.id,
            LocationInput {
                kind_id_str: form.kind_id_str,
                name: "Planche B".to_owned(),
                length_m: dec!(25),
                width_m: dec!(0.8),
                parent_id_str: form.parent_id_str,
                notes: None,
            },
        )
        .await
        .unwrap();
        let after = list_locations_tree(&repo).await.unwrap();
        assert!(after.iter().any(|l| l.name == "Planche B"));
        assert!(after.iter().all(|l| l.name != "Planche A"));
    }

    #[tokio::test]
    async fn update_location_rejects_parent_cycle() {
        let repo = fresh_repo().await; // Jardin Pomone → Planche A
        let tree = list_locations_tree(&repo).await.unwrap();
        let jardin = tree.iter().find(|l| l.name == "Jardin Pomone").unwrap();
        let planche = tree.iter().find(|l| l.name == "Planche A").unwrap();
        let form = get_location_for_edit(&repo, &jardin.id).await.unwrap();
        // Reparent the root under its own child → cycle.
        let err = update_location(
            &repo,
            &jardin.id,
            LocationInput {
                kind_id_str: form.kind_id_str,
                name: form.name,
                length_m: dec!(20),
                width_m: dec!(10),
                parent_id_str: planche.id.clone(),
                notes: None,
            },
        )
        .await
        .unwrap_err();
        assert!(matches!(err, AppError::Inconsistent(m) if m == "location_cycle"));
    }

    #[tokio::test]
    async fn delete_location_refused_when_it_has_children() {
        let repo = fresh_repo().await; // Jardin Pomone (parent) → Planche A
        let jardin = list_locations_tree(&repo)
            .await
            .unwrap()
            .into_iter()
            .find(|l| l.name == "Jardin Pomone")
            .unwrap();
        assert!(jardin.in_use);
        let err = delete_location(&repo, &jardin.id).await.unwrap_err();
        assert!(matches!(err, AppError::Inconsistent(m) if m == "location_in_use"));
    }

    #[tokio::test]
    async fn delete_location_removes_an_unused_leaf() {
        let repo = fresh_repo().await;
        let planche = list_locations_tree(&repo)
            .await
            .unwrap()
            .into_iter()
            .find(|l| l.name == "Planche A")
            .unwrap();
        assert!(!planche.in_use);
        delete_location(&repo, &planche.id).await.unwrap();
        assert!(list_locations_tree(&repo)
            .await
            .unwrap()
            .iter()
            .all(|l| l.name != "Planche A"));
    }

    #[tokio::test]
    async fn delete_location_refused_when_it_holds_a_planting() {
        use crate::services::create_annual_planting_from_sowing;
        use chrono::NaiveDate;
        use pomone_db::{StrataRepo, VarietyRepo};
        let repo = fresh_repo().await;
        let variety = repo.variety_list().await.unwrap()[0].id;
        let strata = repo.strata_list().await.unwrap()[0].id;
        let planche = list_locations_tree(&repo)
            .await
            .unwrap()
            .into_iter()
            .find(|l| l.name == "Planche A")
            .unwrap();
        let loc_id: LocationId = crate::plantings_view::parse_id(&planche.id).unwrap();
        create_annual_planting_from_sowing(
            &repo,
            variety,
            loc_id,
            strata,
            NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
            dec!(10),
            10,
            None,
            None,
        )
        .await
        .unwrap();
        let planche = list_locations_tree(&repo)
            .await
            .unwrap()
            .into_iter()
            .find(|l| l.name == "Planche A")
            .unwrap();
        assert!(planche.in_use);
        let err = delete_location(&repo, &planche.id).await.unwrap_err();
        assert!(matches!(err, AppError::Inconsistent(m) if m == "location_in_use"));
    }
}
