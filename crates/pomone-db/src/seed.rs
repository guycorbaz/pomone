//! Seed default lookup data on a fresh database.
//!
//! Idempotent: only writes the default rows if the corresponding table is
//! empty. Safe to run on every application start.

use crate::error::DbResult;
use crate::repository::Repository;
use pomone_domain::{Family, LocationKind, Strata, TaskCategory, TaskType};
use rust_decimal::Decimal;
use std::str::FromStr;

/// Seven classic agroforestry/permaculture strata, with indicative heights.
fn default_strata() -> Vec<Strata> {
    let m = |s: &str| Decimal::from_str(s).ok();
    [
        ("Canopée", "arbres de haut-jet (> 6 m)", m("6"), m("40"), 10),
        (
            "Sous-étage",
            "arbres et arbres fruitiers moyens (3 à 6 m)",
            m("3"),
            m("6"),
            20,
        ),
        (
            "Arbustive",
            "arbustes à fruits, haies basses (1 à 3 m)",
            m("1"),
            m("3"),
            30,
        ),
        (
            "Herbacée",
            "légumes annuels, vivaces basses (0,3 à 1 m)",
            m("0.3"),
            m("1"),
            40,
        ),
        (
            "Couvre-sol",
            "couvre-sols, fraisiers (< 0,3 m)",
            m("0"),
            m("0.3"),
            50,
        ),
        (
            "Racinaire",
            "tubercules et racines (sous-sol)",
            None,
            None,
            60,
        ),
        (
            "Grimpante",
            "vigne, kiwi, haricot grimpant (variable)",
            None,
            None,
            70,
        ),
    ]
    .into_iter()
    .map(|(name, desc, min_h, max_h, sort)| {
        Strata::new(name, Some(desc.into()), min_h, max_h, sort)
            .expect("static seed Strata is always valid")
    })
    .collect()
}

/// Common kinds of physical locations on a polycultural farm.
fn default_location_kinds() -> Vec<LocationKind> {
    [
        ("Parcelle", "parcelle de plein champ"),
        ("Planche", "planche maraîchère"),
        ("Serre", "serre ou tunnel"),
        ("Verger", "ensemble d'arbres fruitiers"),
        ("Ligne agroforestière", "rang d'agroforesterie"),
        ("Haie", "haie bocagère ou brise-vent"),
    ]
    .into_iter()
    .map(|(name, desc)| {
        LocationKind::new(name, Some(desc.into()))
            .expect("static seed LocationKind is always valid")
    })
    .collect()
}

/// Common botanical families seen across market gardening, field crops,
/// orcharding, and agroforestry.
fn default_families() -> Vec<Family> {
    [
        (
            "Solanacées",
            "Solanaceae",
            "tomate, pomme de terre, poivron, aubergine",
        ),
        (
            "Brassicacées",
            "Brassicaceae",
            "chou, radis, navet, moutarde, roquette",
        ),
        ("Apiacées", "Apiaceae", "carotte, persil, céleri, fenouil"),
        ("Fabacées", "Fabaceae", "haricot, pois, fève, lentille"),
        (
            "Astéracées",
            "Asteraceae",
            "salade, chicorée, topinambour, artichaut",
        ),
        (
            "Cucurbitacées",
            "Cucurbitaceae",
            "courge, courgette, concombre, melon",
        ),
        ("Liliacées", "Liliaceae", "ail, oignon, poireau, asperge"),
        (
            "Rosacées",
            "Rosaceae",
            "pommier, poirier, prunier, fraisier, framboisier",
        ),
        ("Bétulacées", "Betulaceae", "noisetier, bouleau, aulne"),
        ("Juglandacées", "Juglandaceae", "noyer"),
        ("Fagacées", "Fagaceae", "châtaignier, chêne, hêtre"),
    ]
    .into_iter()
    .map(|(name, latin, desc)| {
        Family::new(name, Some(latin.into()), Some(desc.into()))
            .expect("static seed Family is always valid")
    })
    .collect()
}

/// Default task types — one per `TaskCategory`. Names are in French
/// (Pomone's primary language); users can rename in the app. Colors follow
/// the light-theme palette: greens for growth phases, terracotta for
/// remediation, accent for harvest, neutrals for prep.
fn default_task_types() -> Vec<TaskType> {
    [
        (TaskCategory::Sow, "Semis", "#6FAF7A"),
        (TaskCategory::Transplant, "Repiquage", "#3C6E47"),
        (TaskCategory::Harvest, "Récolte", "#B85C38"),
        (TaskCategory::Weeding, "Désherbage", "#B07C25"),
        (TaskCategory::Irrigation, "Irrigation", "#5F9F8B"),
        (TaskCategory::Treatment, "Traitement", "#A64238"),
        (TaskCategory::Tillage, "Travail du sol", "#6B5D4D"),
        (TaskCategory::Other, "Autre", "#A09887"),
    ]
    .into_iter()
    .map(|(category, name, color)| {
        TaskType::new(name, category, color).expect("static seed TaskType is always valid")
    })
    .collect()
}

/// Run all seed steps. Each step is a no-op if its target table is non-empty.
pub async fn seed_defaults(repo: &dyn Repository) -> DbResult<()> {
    if repo.strata_list().await?.is_empty() {
        for s in default_strata() {
            repo.strata_create(&s).await?;
        }
    }
    if repo.location_kind_list().await?.is_empty() {
        for k in default_location_kinds() {
            repo.location_kind_create(&k).await?;
        }
    }
    if repo.family_list().await?.is_empty() {
        for f in default_families() {
            repo.family_create(&f).await?;
        }
    }
    if repo.task_type_list().await?.is_empty() {
        for tt in default_task_types() {
            repo.task_type_create(&tt).await?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::{FamilyRepo, LocationKindRepo, StrataRepo};
    use crate::SqliteRepository;

    #[tokio::test]
    async fn seed_populates_three_lookup_tables() {
        let repo = SqliteRepository::in_memory().await.unwrap();
        seed_defaults(&repo).await.unwrap();
        assert!(!repo.strata_list().await.unwrap().is_empty());
        assert!(!repo.location_kind_list().await.unwrap().is_empty());
        assert!(!repo.family_list().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn seed_is_idempotent() {
        let repo = SqliteRepository::in_memory().await.unwrap();
        seed_defaults(&repo).await.unwrap();
        let first_count = repo.strata_list().await.unwrap().len();
        seed_defaults(&repo).await.unwrap();
        let second_count = repo.strata_list().await.unwrap().len();
        assert_eq!(first_count, second_count);
    }

    #[tokio::test]
    async fn seed_skips_already_populated_table() {
        let repo = SqliteRepository::in_memory().await.unwrap();
        // Pre-populate strata only
        let s = Strata::new("Custom", None, None, None, 0).unwrap();
        repo.strata_create(&s).await.unwrap();
        seed_defaults(&repo).await.unwrap();
        let strata = repo.strata_list().await.unwrap();
        // Only the custom one — defaults skipped
        assert_eq!(strata.len(), 1);
        // But families and location_kinds should still be seeded
        assert!(!repo.family_list().await.unwrap().is_empty());
        assert!(!repo.location_kind_list().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn seeded_strata_count_matches_static_list() {
        let repo = SqliteRepository::in_memory().await.unwrap();
        seed_defaults(&repo).await.unwrap();
        assert_eq!(
            repo.strata_list().await.unwrap().len(),
            default_strata().len()
        );
    }
}
