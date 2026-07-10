//! Demo data: a realistic-but-small starting dataset for trying Pomone
//! without having to type everything in by hand.
//!
//! Unlike [`crate::test_helpers::seed_test_data`] (which only adds the
//! bare minimum the unit-tests need), this populates:
//!
//! - 9 crops (Tomate, Carotte, Laitue, Courgette, Pommier + the field
//!   crops Blé tendre, Maïs grain, Pomme de terre, Betterave sucrière)
//!   using the already-seeded families + strata + location kinds.
//! - 11 varieties spread across those crops.
//! - 1 root location ("Jardin de démo") with 5 child beds + 1 greenhouse,
//!   a separate "Verger Est" root for the perennial planting, and a
//!   field-crop farm root ("Les Grandes Terres", 20 ha) with 4 hectare-
//!   scale parcels (issue #29 — the demo exercises both m²- and ha-scale
//!   data).
//! - 10 annual plantings + 1 perennial planting laid out around the
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
    let poacees = find_family(repo, "Poacées").await?;
    let amaranthacees = find_family(repo, "Amaranthacées").await?;
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
        Some(poacees),
        Some(amaranthacees),
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
        poacees,
        amaranthacees,
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
    // Field crops (issue #29): hectare-scale references alongside the
    // market-garden set.
    let ble = Crop::new(
        poacees.id,
        "Blé tendre",
        Some("Triticum aestivum".to_owned()),
        Lifespan::Annual,
        PruningSeason::None,
    )?;
    repo.crop_create(&ble).await?;
    let mais = Crop::new(
        poacees.id,
        "Maïs grain",
        Some("Zea mays".to_owned()),
        Lifespan::Annual,
        PruningSeason::None,
    )?;
    repo.crop_create(&mais).await?;
    let pomme_de_terre = Crop::new(
        solanacees.id,
        "Pomme de terre",
        Some("Solanum tuberosum".to_owned()),
        Lifespan::Annual,
        PruningSeason::None,
    )?;
    repo.crop_create(&pomme_de_terre).await?;
    let betterave = Crop::new(
        amaranthacees.id,
        "Betterave sucrière",
        Some("Beta vulgaris".to_owned()),
        Lifespan::Annual,
        PruningSeason::None,
    )?;
    repo.crop_create(&betterave).await?;
    s.crops_created = 9;

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
    // Field-crop varieties — direct-sown (no transplant phase), season-long
    // cycles: winter wheat ~270 days sowing→harvest, grain maize ~170,
    // Charlotte potato ~100 with a long lifting window, sugar beet ~190.
    let arina = annual_variety(ble.id, "Arina", None, 270, 10)?;
    let lg_mais = annual_variety(mais.id, "LG 30.222", None, 170, 15)?;
    let charlotte = annual_variety(pomme_de_terre.id, "Charlotte", None, 100, 45)?;
    let belamia = annual_variety(betterave.id, "Belamia", None, 190, 30)?;
    for v in [
        &marmande,
        &roma,
        &nantaise,
        &batavia,
        &romaine,
        &verte_milan,
        &reine_reinettes,
        &arina,
        &lg_mais,
        &charlotte,
        &belamia,
    ] {
        repo.variety_create(v).await?;
    }
    s.varieties_created = 11;

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

    // Field-crop farm (issue #29): a 20 ha root with hectare-scale parcels,
    // so the demo shows both m²- and ha-scale data side by side.
    let grandes_terres = Location::new(
        parcelle_kind.id,
        "Les Grandes Terres",
        Decimal::from(500),
        Decimal::from(400),
        None,
        Some("ferme grandes cultures de démonstration (20 ha)".to_owned()),
    )?;
    repo.location_create(&grandes_terres).await?;
    let parcel = |name: &str, length_m: i64, width_m: i64| -> AppResult<Location> {
        Location::new(
            parcelle_kind.id,
            name,
            Decimal::from(length_m),
            Decimal::from(width_m),
            Some(grandes_terres.id),
            None,
        )
        .map_err(Into::into)
    };
    let piece_moulin = parcel("Pièce du Moulin", 400, 200)?; // 8 ha
    let piece_lac = parcel("Pièce du Lac", 250, 200)?; // 5 ha
    let piece_longue = parcel("Pièce Longue", 300, 100)?; // 3 ha
    let piece_vernes = parcel("Pièce des Vernes", 200, 150)?; // 3 ha
    for l in [&piece_moulin, &piece_lac, &piece_longue, &piece_vernes] {
        repo.location_create(l).await?;
    }
    s.locations_created = 12;

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

    // Field-crop plantings (issue #29), hectare-scale areas and realistic
    // plant densities. Winter wheat is sown the *previous* autumn so its
    // harvest (~270 days later) lands in the current season.
    let previous_autumn = NaiveDate::from_ymd_opt(year - 1, 10, 15).unwrap_or(today);
    let early_april = NaiveDate::from_ymd_opt(year, 4, 5).unwrap_or(today);
    let late_april = NaiveDate::from_ymd_opt(year, 4, 25).unwrap_or(today);
    let late_march = NaiveDate::from_ymd_opt(year, 3, 20).unwrap_or(today);
    create_annual_planting_from_sowing(
        repo,
        arina.id,
        piece_moulin.id,
        herbacee.id,
        previous_autumn,
        Decimal::from(80_000), // 8 ha
        28_000_000,            // ~3.5 M plants/ha
        Some("démo: blé d'hiver".to_owned()),
        None,
    )
    .await?;
    create_annual_planting_from_sowing(
        repo,
        lg_mais.id,
        piece_lac.id,
        herbacee.id,
        late_april,
        Decimal::from(50_000), // 5 ha
        450_000,               // ~90 000 plants/ha
        Some("démo: maïs grain".to_owned()),
        None,
    )
    .await?;
    create_annual_planting_from_sowing(
        repo,
        charlotte.id,
        piece_longue.id,
        racinaire.id,
        early_april,
        Decimal::from(30_000), // 3 ha
        120_000,               // ~40 000 plants/ha
        Some("démo: pomme de terre Charlotte".to_owned()),
        None,
    )
    .await?;
    create_annual_planting_from_sowing(
        repo,
        belamia.id,
        piece_vernes.id,
        racinaire.id,
        late_march,
        Decimal::from(30_000), // 3 ha
        300_000,               // ~100 000 plants/ha
        Some("démo: betterave sucrière".to_owned()),
        None,
    )
    .await?;
    s.plantings_created += 4;

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
    use pomone_db::{
        seed_defaults, CropRepo, LocationKindRepo, LocationRepo, SqliteRepository, StrataRepo,
        VarietyRepo,
    };

    #[tokio::test]
    async fn seed_demo_populates_expected_counts() {
        let repo = SqliteRepository::in_memory().await.unwrap();
        seed_defaults(&repo).await.unwrap();
        let today = NaiveDate::from_ymd_opt(2026, 5, 25).unwrap();
        let summary = seed_demo_data(&repo, today).await.unwrap();
        assert_eq!(summary.crops_created, 9);
        assert_eq!(summary.varieties_created, 11);
        assert_eq!(summary.locations_created, 12);
        assert_eq!(summary.plantings_created, 11);
        assert_eq!(summary.recurring_series_created, 1);

        // Sanity: the DB now has the corresponding rows.
        assert_eq!(repo.crop_list().await.unwrap().len(), 9);
        assert_eq!(repo.variety_list().await.unwrap().len(), 11);
        assert_eq!(repo.location_list().await.unwrap().len(), 12);
    }

    /// Issue #29 — "80 ha cereal farm" scenario: hectare-scale areas must
    /// survive the full stack (services → storage → view formatting) with
    /// no precision loss in either display unit.
    #[tokio::test]
    async fn field_crop_scenario_80ha_formats_in_both_units() {
        use crate::locations_view::list_locations_tree;
        use crate::plantings_view::list_plantings;
        use crate::units::AreaUnit;

        let repo = SqliteRepository::in_memory().await.unwrap();
        seed_defaults(&repo).await.unwrap();
        let today = NaiveDate::from_ymd_opt(2026, 5, 25).unwrap();
        seed_demo_data(&repo, today).await.unwrap();

        // One 80 ha wheat block on top of the demo farm.
        let kinds = repo.location_kind_list().await.unwrap();
        let parcelle = kinds.iter().find(|k| k.name == "Parcelle").unwrap();
        let big_field = Location::new(
            parcelle.id,
            "Grande pièce 80 ha",
            Decimal::from(1000),
            Decimal::from(800),
            None,
            None,
        )
        .unwrap();
        repo.location_create(&big_field).await.unwrap();
        let arina = repo
            .variety_list()
            .await
            .unwrap()
            .into_iter()
            .find(|v| v.name == "Arina")
            .unwrap();
        let herbacee = repo
            .strata_list()
            .await
            .unwrap()
            .into_iter()
            .find(|s| s.name == "Herbacée")
            .unwrap();
        create_annual_planting_from_sowing(
            &repo,
            arina.id,
            big_field.id,
            herbacee.id,
            NaiveDate::from_ymd_opt(2025, 10, 15).unwrap(),
            Decimal::from(800_000), // 80 ha
            280_000_000,            // ~3.5 M plants/ha over 80 ha
            None,
            None,
        )
        .await
        .unwrap();

        // Plantings list: exact in both units, no scientific notation, no
        // rounding artefacts.
        let rows_ha = list_plantings(&repo, AreaUnit::Hectares).await.unwrap();
        let wheat = rows_ha
            .iter()
            .find(|r| r.area_m2 == Decimal::from(800_000))
            .unwrap();
        assert_eq!(wheat.area_label, "80 ha");
        assert_eq!(wheat.plants_count, 280_000_000);
        let rows_m2 = list_plantings(&repo, AreaUnit::SquareMeters).await.unwrap();
        let wheat_m2 = rows_m2
            .iter()
            .find(|r| r.area_m2 == Decimal::from(800_000))
            .unwrap();
        assert_eq!(wheat_m2.area_label, "800000 m²");

        // Locations tree: the 80 ha field and the demo's 8 ha parcel both
        // format exactly.
        let tree = list_locations_tree(&repo, AreaUnit::Hectares)
            .await
            .unwrap();
        let field = tree
            .iter()
            .find(|l| l.name == "Grande pièce 80 ha")
            .unwrap();
        assert_eq!(field.area_label, "80 ha");
        assert_eq!(field.dimensions_label, "1000 × 800 m = 80 ha");
        let moulin = tree.iter().find(|l| l.name == "Pièce du Moulin").unwrap();
        assert_eq!(moulin.area_label, "8 ha");
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
