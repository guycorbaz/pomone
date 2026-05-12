//! Shared test helpers for the `pomone-app` crate.
//!
//! Started as the runtime `seed_demo` that bootstrapped the very first UI
//! sessions when no `crop` / `variety` / `location` had been created yet.
//! Now that the UI exposes the full CRUD path for those entities, the seed
//! only survives here as test scaffolding — the runtime no longer touches
//! it, and the user sees an empty DB at first launch.

use crate::error::AppResult;
use pomone_db::Repository;
use pomone_domain::{
    AnnualProfile, Crop, Family, Lifespan, Location, LocationKind, PruningSeason, Strata, Variety,
    VarietyProfile,
};
use rust_decimal::Decimal;

/// Populate `repo` with: one annual `Tomate` crop with two varieties
/// (Marmande, Roma), plus a `Jardin Pomone` parcel containing a 25 × 0.8 m
/// `Planche A`. Idempotent — does nothing if `crop_list()` is non-empty.
///
/// Used as the starting point for `plantings_view`, `cultures_view` and
/// `locations_view` tests, so each one has a non-trivial baseline DB and we
/// only assert on the deltas we care about.
pub(crate) async fn seed_test_data(repo: &dyn Repository) -> AppResult<()> {
    if !repo.crop_list().await?.is_empty() {
        return Ok(());
    }

    let solanaceae = find_family_by_latin(repo, "Solanaceae").await?;
    let herbacee = find_strata_by_name(repo, "Herbacée").await?;
    let parcelle_kind = find_kind_by_name(repo, "Parcelle").await?;
    let planche_kind = find_kind_by_name(repo, "Planche").await?;

    let (Some(family), Some(strata), Some(parcelle_kind), Some(planche_kind)) =
        (solanaceae, herbacee, parcelle_kind, planche_kind)
    else {
        return Ok(());
    };

    let tomato = Crop::new(
        family.id,
        strata.id,
        "Tomate",
        Some("Solanum lycopersicum".to_owned()),
        Lifespan::Annual,
        PruningSeason::Summer,
    )?;
    repo.crop_create(&tomato).await?;

    let marmande = Variety::new(
        tomato.id,
        Lifespan::Annual,
        "Marmande",
        Some("ancienne variété, fruits côtelés, mi-précoce".to_owned()),
        VarietyProfile::Annual(AnnualProfile::new(Some(35), 70, 60)?),
    )?;
    repo.variety_create(&marmande).await?;

    let roma = Variety::new(
        tomato.id,
        Lifespan::Annual,
        "Roma",
        Some("tomate allongée à sauce, productive".to_owned()),
        VarietyProfile::Annual(AnnualProfile::new(Some(30), 65, 50)?),
    )?;
    repo.variety_create(&roma).await?;

    let jardin = Location::new(
        parcelle_kind.id,
        "Jardin Pomone",
        Decimal::from(20),
        Decimal::from(10),
        None,
        Some("jardin de démonstration (tests uniquement)".to_owned()),
    )?;
    repo.location_create(&jardin).await?;

    let planche = Location::new(
        planche_kind.id,
        "Planche A",
        Decimal::from(25),
        Decimal::new(8, 1), // 0.8 m
        Some(jardin.id),
        None,
    )?;
    repo.location_create(&planche).await?;

    Ok(())
}

async fn find_family_by_latin(repo: &dyn Repository, latin: &str) -> AppResult<Option<Family>> {
    let families = repo.family_list().await?;
    Ok(families
        .into_iter()
        .find(|f| f.latin_name.as_deref() == Some(latin)))
}

async fn find_strata_by_name(repo: &dyn Repository, name: &str) -> AppResult<Option<Strata>> {
    let strata = repo.strata_list().await?;
    Ok(strata.into_iter().find(|s| s.name == name))
}

async fn find_kind_by_name(repo: &dyn Repository, name: &str) -> AppResult<Option<LocationKind>> {
    let kinds = repo.location_kind_list().await?;
    Ok(kinds.into_iter().find(|k| k.name == name))
}
