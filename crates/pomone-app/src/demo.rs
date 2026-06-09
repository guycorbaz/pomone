//! Demo data: a realistic-but-small starting dataset for trying Pomone
//! without having to type everything in by hand.
//!
//! Unlike [`crate::test_helpers::seed_test_data`] (which only adds the
//! bare minimum the unit-tests need), this populates:
//!
//! - 5 crops (Tomate, Carotte, Laitue, Courgette, Pommier) using the
//!   already-seeded families + strata + location kinds.
//! - 7 varieties spread across those crops.
//! - 1 root location ("Jardin de démo") with 5 child beds + 1 greenhouse,
//!   plus a separate "Verger Est" root for the perennial planting.
//! - 7 annual plantings + 1 perennial planting laid out around the
//!   reference date — every one triggers the operational auto-gen so
//!   the Tasks calendar is non-empty too.
//! - 1 recurring weekly weeding series on a bed.
//!
//! The function is idempotent in the "all-or-nothing" sense: if any
//! `Crop` already exists, it bails out — we don't want to scramble a
//! database the user has carefully populated.
//!
//! `today` is injected so callers (CLI, integration tests) keep
//! deterministic behavior. The dates of the demo plantings are anchored
//! to `today` so the result always looks current.

use crate::error::AppResult;
use crate::services::{create_annual_planting_from_sowing, create_perennial_planting};
use crate::tasks_view::create_recurring_task;
use chrono::{Datelike, NaiveDate};
use pomone_db::Repository;
use pomone_domain::{
    AnnualProfile, Crop, Family, Lifespan, Location, LocationKind, PluriannualProfile,
    PruningSeason, Strata, Variety, VarietyId, VarietyProfile,
};
use rust_decimal::Decimal;

/// Summary returned by [`seed_demo_data`] so callers (CLI) can echo what
/// was created without having to re-query the DB.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DemoSummary {
    pub crops_created: u32,
    pub varieties_created: u32,
    pub locations_created: u32,
    pub plantings_created: u32,
    pub recurring_series_created: u32,
}

impl DemoSummary {
    /// Compact one-line description for CLI output.
    #[must_use]
    pub fn one_line(&self) -> String {
        format!(
            "{} cultures, {} variétés, {} lieux, {} plantations, {} série(s) récurrente(s)",
            self.crops_created,
            self.varieties_created,
            self.locations_created,
            self.plantings_created,
            self.recurring_series_created,
        )
    }
}

/// Populate `repo` with a realistic demo dataset anchored to `today`.
/// Refuses to do anything if `crop_list()` is non-empty — protecting an
/// already-populated database.
// The function is a long but linear sequence of inserts; splitting it
// into half-a-dozen helpers would make it harder to read, not easier.
#[allow(clippy::too_many_lines)]
pub async fn seed_demo_data(repo: &dyn Repository, today: NaiveDate) -> AppResult<DemoSummary> {
    if !repo.crop_list().await?.is_empty() {
        return Ok(DemoSummary::default());
    }

    let mut s = DemoSummary::default();
    let solanacees = find_family(repo, "Solanacées").await?;
    let apiacees = find_family(repo, "Apiacées").await?;
    let asteracees = find_family(repo, "Astéracées").await?;
    let cucurbitacees = find_family(repo, "Cucurbitacées").await?;
    let rosacees = find_family(repo, "Rosacées").await?;
    let herbacee = find_strata(repo, "Herbacée").await?;
    let racinaire = find_strata(repo, "Racinaire").await?;
    let sous_etage = find_strata(repo, "Sous-étage").await?;
    let parcelle_kind = find_kind(repo, "Parcelle").await?;
    let planche_kind = find_kind(repo, "Planche").await?;
    let serre_kind = find_kind(repo, "Serre").await?;
    let verger_kind = find_kind(repo, "Verger").await?;

    // If the user wiped the default seeds (unusual), short-circuit
    // rather than fail half-way.
    let (
        Some(solanacees),
        Some(apiacees),
        Some(asteracees),
        Some(cucurbitacees),
        Some(rosacees),
        Some(herbacee),
        Some(racinaire),
        Some(sous_etage),
        Some(parcelle_kind),
        Some(planche_kind),
        Some(serre_kind),
        Some(verger_kind),
    ) = (
        solanacees,
        apiacees,
        asteracees,
        cucurbitacees,
        rosacees,
        herbacee,
        racinaire,
        sous_etage,
        parcelle_kind,
        planche_kind,
        serre_kind,
        verger_kind,
    )
    else {
        return Ok(s);
    };

    // ---------------- Crops + varieties ----------------
    let tomate = Crop::new(
        solanacees.id,
        "Tomate",
        Some("Solanum lycopersicum".to_owned()),
        Lifespan::Annual,
        PruningSeason::Summer,
    )?;
    repo.crop_create(&tomate).await?;
    let carotte = Crop::new(
        apiacees.id,
        "Carotte",
        Some("Daucus carota".to_owned()),
        Lifespan::Annual,
        PruningSeason::None,
    )?;
    repo.crop_create(&carotte).await?;
    let laitue = Crop::new(
        asteracees.id,
        "Laitue",
        Some("Lactuca sativa".to_owned()),
        Lifespan::Annual,
        PruningSeason::None,
    )?;
    repo.crop_create(&laitue).await?;
    let courgette = Crop::new(
        cucurbitacees.id,
        "Courgette",
        Some("Cucurbita pepo".to_owned()),
        Lifespan::Annual,
        PruningSeason::None,
    )?;
    repo.crop_create(&courgette).await?;
    let pommier = Crop::new(
        rosacees.id,
        "Pommier",
        Some("Malus domestica".to_owned()),
        Lifespan::perennial(30, 3)?,
        PruningSeason::Winter,
    )?;
    repo.crop_create(&pommier).await?;
    s.crops_created = 5;

    // Two tomato varieties, one carrot, two lettuces, one zucchini, one apple.
    let marmande = annual_variety(tomate.id, "Marmande", Some(35), 70, 60)?;
    let roma = annual_variety(tomate.id, "Roma", Some(30), 65, 50)?;
    let nantaise = annual_variety(carotte.id, "Nantaise", None, 80, 30)?;
    let batavia = annual_variety(laitue.id, "Batavia", Some(20), 50, 25)?;
    let romaine = annual_variety(laitue.id, "Romaine", Some(20), 55, 25)?;
    let verte_milan = annual_variety(courgette.id, "Verte de Milan", Some(25), 55, 60)?;
    let reine_reinettes = Variety::new(
        pommier.id,
        Lifespan::perennial(30, 3)?,
        "Reine des Reinettes",
        Some("ancienne variété, fruits dessert".to_owned()),
        VarietyProfile::Pluriannual(PluriannualProfile::new(
            Some(80),
            Some(120),
            220,
            280,
            None,
        )?),
    )?;
    for v in [
        &marmande,
        &roma,
        &nantaise,
        &batavia,
        &romaine,
        &verte_milan,
        &reine_reinettes,
    ] {
        repo.variety_create(v).await?;
    }
    s.varieties_created = 7;

    // ---------------- Locations ----------------
    let jardin = Location::new(
        parcelle_kind.id,
        "Jardin de démo",
        Decimal::from(40),
        Decimal::from(15),
        None,
        Some("jardin créé par `pomone-cli seed-demo`".to_owned()),
    )?;
    repo.location_create(&jardin).await?;

    let bed = |name: &str| -> AppResult<Location> {
        Location::new(
            planche_kind.id,
            name,
            Decimal::from(25),
            Decimal::new(8, 1), // 0.8 m
            Some(jardin.id),
            None,
        )
        .map_err(Into::into)
    };
    let planche_a = bed("Planche A")?;
    let planche_b = bed("Planche B")?;
    let planche_c = bed("Planche C")?;
    let planche_d = bed("Planche D")?;
    let serre = Location::new(
        serre_kind.id,
        "Serre 1",
        Decimal::from(10),
        Decimal::from(4),
        Some(jardin.id),
        None,
    )?;
    let verger = Location::new(
        verger_kind.id,
        "Verger Est",
        Decimal::from(30),
        Decimal::from(20),
        None,
        Some("verger de démonstration (pommier)".to_owned()),
    )?;
    for l in [
        &planche_a, &planche_b, &planche_c, &planche_d, &serre, &verger,
    ] {
        repo.location_create(l).await?;
    }
    s.locations_created = 7;

    // ---------------- Plantings ----------------
    // Dates anchored on the year of `today` so the demo always looks
    // "this season": early-spring = March 1, mid-spring = May 1,
    // late-spring = June 1. The chrono constructor never fails for these
    // (well-formed dates of the current year).
    let year = today.year();
    let early_spring = NaiveDate::from_ymd_opt(year, 3, 1).unwrap_or(today);
    let mid_spring = NaiveDate::from_ymd_opt(year, 5, 1).unwrap_or(today);
    let late_spring = NaiveDate::from_ymd_opt(year, 6, 1).unwrap_or(today);

    // Annual plantings — every call also triggers task auto-gen.
    create_annual_planting_from_sowing(
        repo,
        marmande.id,
        serre.id,
        herbacee.id,
        early_spring,
        Decimal::from(8),
        24,
        Some("démo: Marmande sous serre".to_owned()),
        None,
    )
    .await?;
    create_annual_planting_from_sowing(
        repo,
        roma.id,
        planche_a.id,
        herbacee.id,
        early_spring,
        Decimal::from(20),
        60,
        Some("démo: Roma plein champ".to_owned()),
        None,
    )
    .await?;
    create_annual_planting_from_sowing(
        repo,
        nantaise.id,
        planche_b.id,
        racinaire.id,
        mid_spring,
        Decimal::from(20),
        400,
        Some("démo: carotte semis direct".to_owned()),
        None,
    )
    .await?;
    create_annual_planting_from_sowing(
        repo,
        batavia.id,
        planche_c.id,
        herbacee.id,
        early_spring,
        Decimal::from(10),
        80,
        Some("démo: laitue 1re succession".to_owned()),
        None,
    )
    .await?;
    create_annual_planting_from_sowing(
        repo,
        romaine.id,
        planche_c.id,
        herbacee.id,
        mid_spring,
        Decimal::from(10),
        80,
        Some("démo: laitue 2e succession".to_owned()),
        None,
    )
    .await?;
    create_annual_planting_from_sowing(
        repo,
        verte_milan.id,
        planche_d.id,
        herbacee.id,
        mid_spring,
        Decimal::from(20),
        12,
        Some("démo: courgette".to_owned()),
        None,
    )
    .await?;
    s.plantings_created += 6;

    // Perennial: 1 apple tree on the orchard.
    create_perennial_planting(
        repo,
        reine_reinettes.id,
        verger.id,
        sous_etage.id,
        early_spring,
        None,
        Decimal::from(20),
        1,
        Some("démo: pommier Reine des Reinettes".to_owned()),
        None,
    )
    .await?;
    s.plantings_created += 1;

    // ---------------- Recurring series ----------------
    // Weekly weeding on Planche A starting from late-spring, no end date
    // (the calendar will extend it forward to today+1y automatically).
    let weeding_type = repo
        .task_type_list()
        .await?
        .into_iter()
        .find(|t| t.category == pomone_domain::TaskCategory::Weeding);
    if let Some(tt) = weeding_type {
        let planting_id = repo
            .planting_list_for_location(planche_a.id)
            .await?
            .into_iter()
            .next()
            .map(|p| p.id.to_string())
            .unwrap_or_default();
        create_recurring_task(
            repo,
            &planting_id,
            &tt.id.to_string(),
            &late_spring.format("%Y-%m-%d").to_string(),
            "désherbage hebdomadaire — démo",
            1,
            "weeks",
            None,
            today,
        )
        .await?;
        s.recurring_series_created = 1;
    }

    Ok(s)
}

fn annual_variety(
    crop_id: pomone_domain::CropId,
    name: &str,
    dtt: Option<u16>,
    dtm: u16,
    harvest_window: u16,
) -> AppResult<Variety> {
    Variety::new(
        crop_id,
        Lifespan::Annual,
        name,
        None,
        VarietyProfile::Annual(AnnualProfile::new(dtt, dtm, harvest_window)?),
    )
    .map_err(Into::into)
}

async fn find_family(repo: &dyn Repository, name: &str) -> AppResult<Option<Family>> {
    let families = repo.family_list().await?;
    Ok(families.into_iter().find(|f| f.name == name))
}

async fn find_strata(repo: &dyn Repository, name: &str) -> AppResult<Option<Strata>> {
    let strata = repo.strata_list().await?;
    Ok(strata.into_iter().find(|s| s.name == name))
}

async fn find_kind(repo: &dyn Repository, name: &str) -> AppResult<Option<LocationKind>> {
    let kinds = repo.location_kind_list().await?;
    Ok(kinds.into_iter().find(|k| k.name == name))
}

// Silence "unused" until the demo grows further uses; the type IS used
// by the Variety profile path even though clippy doesn't notice through
// the `match` on profile.
#[allow(dead_code)]
fn _ensure_variety_id_in_scope(_v: VarietyId) {}

#[cfg(test)]
mod tests {
    use super::*;
    use pomone_db::{seed_defaults, CropRepo, LocationRepo, SqliteRepository, VarietyRepo};

    #[tokio::test]
    async fn seed_demo_populates_expected_counts() {
        let repo = SqliteRepository::in_memory().await.unwrap();
        seed_defaults(&repo).await.unwrap();
        let today = NaiveDate::from_ymd_opt(2026, 5, 25).unwrap();
        let summary = seed_demo_data(&repo, today).await.unwrap();
        assert_eq!(summary.crops_created, 5);
        assert_eq!(summary.varieties_created, 7);
        assert_eq!(summary.locations_created, 7);
        assert_eq!(summary.plantings_created, 7);
        assert_eq!(summary.recurring_series_created, 1);

        // Sanity: the DB now has the corresponding rows.
        assert_eq!(repo.crop_list().await.unwrap().len(), 5);
        assert_eq!(repo.variety_list().await.unwrap().len(), 7);
        assert_eq!(repo.location_list().await.unwrap().len(), 7);
    }

    #[tokio::test]
    async fn seed_demo_is_a_noop_on_populated_db() {
        let repo = SqliteRepository::in_memory().await.unwrap();
        seed_defaults(&repo).await.unwrap();
        let today = NaiveDate::from_ymd_opt(2026, 5, 25).unwrap();
        // First call populates.
        seed_demo_data(&repo, today).await.unwrap();
        let crops_before = repo.crop_list().await.unwrap().len();
        // Second call must NOT add anything.
        let summary2 = seed_demo_data(&repo, today).await.unwrap();
        assert_eq!(summary2, DemoSummary::default());
        let crops_after = repo.crop_list().await.unwrap().len();
        assert_eq!(crops_before, crops_after);
    }

    #[test]
    fn one_line_summary_is_readable() {
        let s = DemoSummary {
            crops_created: 5,
            varieties_created: 7,
            locations_created: 7,
            plantings_created: 7,
            recurring_series_created: 1,
        };
        assert!(s.one_line().contains("5 cultures"));
        assert!(s.one_line().contains("1 série"));
    }
}
