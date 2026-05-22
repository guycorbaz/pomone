//! Data migration between two [`Repository`] backends.
//!
//! `copy_all` reads every entity from a source repository and writes it
//! into a target repository in foreign-key-safe order. The target must be
//! empty of user data (the caller skips `seed_defaults` for that reason);
//! lookup tables (families, strata, location kinds) are copied verbatim
//! from the source rather than re-seeded so the user's edits survive the
//! migration.
//!
//! The function is intentionally synchronous-feeling — it walks the entity
//! graph once, sequential awaits, no parallelism. Pomone-sized catalogs
//! make the simple loop trivially fast; parallelism would buy nothing and
//! complicate FK ordering.
//!
//! Returned [`MigrationReport`] carries the count of records copied per
//! entity so the UI can display a one-line summary.

use crate::error::AppResult;
use pomone_db::Repository;
use pomone_domain::{Location, LocationId};
use std::collections::{HashMap, HashSet};

/// Per-entity count of records copied during a [`copy_all`] run.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MigrationReport {
    pub families: usize,
    pub strata: usize,
    pub location_kinds: usize,
    pub locations: usize,
    pub crops: usize,
    pub varieties: usize,
    pub plantings: usize,
    pub yearly_harvests: usize,
}

/// Copy every record from `source` into `target`. Honours FK order:
/// families/strata/location_kinds → locations (roots before children)
/// → crops → varieties → plantings → yearly_harvests.
///
/// The target must be a fresh schema with **no** seeded defaults — IDs of
/// the source's lookup tables (Family/Strata/LocationKind) are reused
/// verbatim, and the underlying `_create` calls would otherwise fail on a
/// duplicate primary key.
pub async fn copy_all(
    source: &dyn Repository,
    target: &dyn Repository,
) -> AppResult<MigrationReport> {
    let mut report = MigrationReport::default();

    // 1. Independent lookup tables.
    for f in source.family_list().await? {
        target.family_create(&f).await?;
        report.families += 1;
    }
    for s in source.strata_list().await? {
        target.strata_create(&s).await?;
        report.strata += 1;
    }
    for k in source.location_kind_list().await? {
        target.location_kind_create(&k).await?;
        report.location_kinds += 1;
    }

    // 2. Locations: pre-order walk so each parent lands before its children
    //    (the FK on `parent_id` would otherwise reject the insert).
    let locations = source.location_list().await?;
    let by_id: HashMap<LocationId, Location> =
        locations.iter().map(|l| (l.id, l.clone())).collect();
    let mut ordered: Vec<Location> = Vec::with_capacity(locations.len());
    let mut emitted: HashSet<LocationId> = HashSet::new();
    for l in &locations {
        push_with_ancestors(l, &by_id, &mut ordered, &mut emitted);
    }
    for l in &ordered {
        target.location_create(l).await?;
        report.locations += 1;
    }

    // 3. Crops, varieties, plantings, yearly harvests.
    for c in source.crop_list().await? {
        target.crop_create(&c).await?;
        report.crops += 1;
    }
    for v in source.variety_list().await? {
        target.variety_create(&v).await?;
        report.varieties += 1;
    }
    let plantings = source.planting_list().await?;
    for p in &plantings {
        target.planting_create(p).await?;
        report.plantings += 1;
    }
    for p in &plantings {
        for h in source.yearly_harvest_list_for_planting(p.id).await? {
            target.yearly_harvest_upsert(&h).await?;
            report.yearly_harvests += 1;
        }
    }

    Ok(report)
}

fn push_with_ancestors(
    loc: &Location,
    by_id: &HashMap<LocationId, Location>,
    ordered: &mut Vec<Location>,
    emitted: &mut HashSet<LocationId>,
) {
    if emitted.contains(&loc.id) {
        return;
    }
    if let Some(pid) = loc.parent_id {
        if let Some(parent) = by_id.get(&pid) {
            push_with_ancestors(parent, by_id, ordered, emitted);
        }
    }
    emitted.insert(loc.id);
    ordered.push(loc.clone());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::{
        create_annual_planting_from_sowing, create_perennial_planting, record_yearly_harvest,
    };
    use crate::test_helpers::seed_test_data;
    use chrono::NaiveDate;
    use pomone_db::{
        seed_defaults, FamilyRepo, LocationRepo, PlantingRepo, SqliteRepository, VarietyRepo,
    };
    use rust_decimal_macros::dec;

    async fn seeded_repo() -> SqliteRepository {
        let repo = SqliteRepository::in_memory().await.unwrap();
        seed_defaults(&repo).await.unwrap();
        repo
    }

    #[tokio::test]
    async fn copy_all_round_trips_seeded_data() {
        let source = seeded_repo().await;
        seed_test_data(&source).await.unwrap();
        let target = SqliteRepository::in_memory().await.unwrap();

        let report = copy_all(&source, &target).await.unwrap();
        assert!(report.families > 0);
        assert!(report.strata > 0);
        assert!(report.location_kinds > 0);
        assert!(report.locations > 0);
        assert!(report.crops > 0);
        assert!(report.varieties >= 2);

        // Spot-check that the data is identical.
        assert_eq!(
            source.family_list().await.unwrap(),
            target.family_list().await.unwrap()
        );
        assert_eq!(
            source.location_list().await.unwrap(),
            target.location_list().await.unwrap()
        );
        assert_eq!(
            source.variety_list().await.unwrap(),
            target.variety_list().await.unwrap()
        );
    }

    #[tokio::test]
    async fn copy_all_preserves_plantings_and_harvests() {
        let source = seeded_repo().await;
        seed_test_data(&source).await.unwrap();
        let varieties = source.variety_list().await.unwrap();
        let locations = source.location_list().await.unwrap();
        let bed = locations.iter().find(|l| l.parent_id.is_some()).unwrap();
        create_annual_planting_from_sowing(
            &source,
            varieties[0].id,
            bed.id,
            NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
            dec!(20),
            100,
            None,
            None,
        )
        .await
        .unwrap();

        let target = SqliteRepository::in_memory().await.unwrap();
        let report = copy_all(&source, &target).await.unwrap();
        assert_eq!(report.plantings, 1);
        assert_eq!(report.yearly_harvests, 0);

        let target_plantings = target
            .planting_list()
            .await
            .unwrap()
            .into_iter()
            .map(|p| p.id)
            .collect::<Vec<_>>();
        let source_plantings = source
            .planting_list()
            .await
            .unwrap()
            .into_iter()
            .map(|p| p.id)
            .collect::<Vec<_>>();
        assert_eq!(target_plantings, source_plantings);
    }

    #[tokio::test]
    async fn copy_all_handles_perennial_with_harvests() {
        use pomone_db::{CropRepo, FamilyRepo, LocationKindRepo, StrataRepo};
        use pomone_domain::{
            Crop, Family, Lifespan, Location, LocationKind, PluriannualProfile, PruningSeason,
            Strata, Variety, VarietyProfile,
        };
        let source = seeded_repo().await;
        // Build a perennial setup separately so we can record harvests.
        let f = Family::new("Rosaceae", None, None).unwrap();
        let s = Strata::new("Arborée", None, None, None, 500).unwrap();
        let k = LocationKind::new("Verger", None).unwrap();
        source.family_create(&f).await.unwrap();
        source.strata_create(&s).await.unwrap();
        source.location_kind_create(&k).await.unwrap();
        let crop = Crop::new(
            f.id,
            s.id,
            "Pommier",
            None,
            Lifespan::perennial(40, 3).unwrap(),
            PruningSeason::Winter,
        )
        .unwrap();
        source.crop_create(&crop).await.unwrap();
        let variety = Variety::new(
            crop.id,
            Lifespan::perennial(40, 3).unwrap(),
            "Reinette",
            None,
            VarietyProfile::Pluriannual(
                PluriannualProfile::new(Some(80), Some(120), 220, 280, None).unwrap(),
            ),
        )
        .unwrap();
        source.variety_create(&variety).await.unwrap();
        let location = Location::new(k.id, "Verger Est", dec!(20), dec!(5), None, None).unwrap();
        source.location_create(&location).await.unwrap();
        let planting = create_perennial_planting(
            &source,
            variety.id,
            location.id,
            NaiveDate::from_ymd_opt(2026, 3, 15).unwrap(),
            None,
            dec!(100),
            10,
            None,
            None,
        )
        .await
        .unwrap();
        record_yearly_harvest(
            &source,
            planting.id,
            2030,
            Some(dec!(50)),
            Some(dec!(48)),
            None,
        )
        .await
        .unwrap();

        let target = SqliteRepository::in_memory().await.unwrap();
        let report = copy_all(&source, &target).await.unwrap();
        assert!(report.plantings >= 1);
        assert_eq!(report.yearly_harvests, 1);
    }

    #[tokio::test]
    async fn copy_all_emits_locations_in_parent_first_order() {
        use pomone_db::{LocationKindRepo, StrataRepo};
        use pomone_domain::{LocationKind, Strata};
        // Build a 3-level location chain in the source.
        let source = SqliteRepository::in_memory().await.unwrap();
        let _ = source
            .strata_create(&Strata::new("X", None, None, None, 10).unwrap())
            .await;
        let k = LocationKind::new("Lieu", None).unwrap();
        source.location_kind_create(&k).await.unwrap();
        let farm =
            pomone_domain::Location::new(k.id, "Ferme", dec!(100), dec!(100), None, None).unwrap();
        source.location_create(&farm).await.unwrap();
        let bed =
            pomone_domain::Location::new(k.id, "Planche", dec!(5), dec!(1), Some(farm.id), None)
                .unwrap();
        source.location_create(&bed).await.unwrap();
        let row =
            pomone_domain::Location::new(k.id, "Rang", dec!(5), dec!(0.3), Some(bed.id), None)
                .unwrap();
        source.location_create(&row).await.unwrap();

        let target = SqliteRepository::in_memory().await.unwrap();
        let report = copy_all(&source, &target).await.unwrap();
        assert_eq!(report.locations, 3);
        let copied = target.location_list().await.unwrap();
        assert_eq!(copied.len(), 3);
    }
}
