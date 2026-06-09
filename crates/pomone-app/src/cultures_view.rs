//! Presentation-layer helpers for the Cultures/Varieties screen.
//!
//! Same shape as `plantings_view`: plain-string DTOs and parsers, so the
//! Slint UI never sees `Uuid` / `Decimal`. Listing helpers are intentionally
//! simple — one query per entity then in-memory joins via `HashMap`.

use crate::error::{AppError, AppResult};
use pomone_db::Repository;
use pomone_domain::{
    AnnualProfile, Crop, FamilyId, Lifespan, PluriannualProfile, PruningSeason, Variety,
    VarietyProfile,
};
use rust_decimal::Decimal;
use std::collections::HashMap;

/// What kind of lifespan the UI selected for a new crop. Maps directly to
/// the three concrete `pomone_domain::Lifespan` constructors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifespanKind {
    Annual,
    PluriannualSingleCycle,
    PluriannualRecurring,
}

/// What kind of profile the UI selected for a new variety. The UI is in
/// charge of matching it to the parent crop's lifespan; the service
/// double-checks before persisting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VarietyProfileKind {
    Annual,
    Pluriannual,
}

/// One row of the Cultures list.
///
/// `is_annual` is duplicated as a plain bool (alongside `lifespan_label`)
/// so the UI can quickly decide which variety-form panel to show when this
/// row is the selected crop, without parsing the localized label.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CropRow {
    pub id: String,
    pub name: String,
    pub family_label: String,
    pub lifespan_label: String,
    pub pruning_label: String,
    pub variety_count: u32,
    pub is_annual: bool,
}

/// One row of the Varieties list (always shown filtered by a parent crop).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VarietyRow {
    pub id: String,
    pub name: String,
    pub description: String,
    pub profile_label: String,
}

/// One option for the Family dropdown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FamilyOption {
    pub id: String,
    pub label: String,
}

/// One option for the Strata dropdown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StrataOption {
    pub id: String,
    pub label: String,
}

/// Return crops sorted by name, each annotated with its family label and the
/// number of varieties currently attached. (Strata moved to the planting —
/// issue #86 — so it's no longer a crop attribute.)
pub async fn list_crops(repo: &dyn Repository) -> AppResult<Vec<CropRow>> {
    let mut crops = repo.crop_list().await?;
    crops.sort_by(|a, b| a.name.cmp(&b.name));

    let families = repo.family_list().await?;
    let varieties = repo.variety_list().await?;

    let family_by_id: HashMap<_, _> = families.iter().map(|f| (f.id, f)).collect();
    let mut variety_count: HashMap<_, u32> = HashMap::new();
    for v in &varieties {
        *variety_count.entry(v.crop_id).or_insert(0) += 1;
    }

    let rows = crops
        .into_iter()
        .map(|c| CropRow {
            id: c.id.to_string(),
            family_label: family_by_id
                .get(&c.family_id)
                .map_or_else(|| "?".to_owned(), |f| f.name.clone()),
            lifespan_label: lifespan_label(c.lifespan),
            pruning_label: pruning_label(c.pruning_season),
            variety_count: *variety_count.get(&c.id).unwrap_or(&0),
            is_annual: c.lifespan.is_annual(),
            name: c.name,
        })
        .collect();
    Ok(rows)
}

/// Varieties of a given crop, sorted by name. Caller passes a stringified
/// `CropId` (the UI never holds a typed UUID).
pub async fn list_varieties_for_crop(
    repo: &dyn Repository,
    crop_id_str: &str,
) -> AppResult<Vec<VarietyRow>> {
    let crop_id: pomone_domain::CropId = crate::plantings_view::parse_id(crop_id_str)?;
    let mut varieties = repo.variety_list_for_crop(crop_id).await?;
    varieties.sort_by(|a, b| a.name.cmp(&b.name));
    let rows = varieties.into_iter().map(variety_to_row).collect();
    Ok(rows)
}

/// Families as dropdown options (sorted by display name).
pub async fn list_family_options(repo: &dyn Repository) -> AppResult<Vec<FamilyOption>> {
    let mut families = repo.family_list().await?;
    families.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(families
        .into_iter()
        .map(|f| FamilyOption {
            id: f.id.to_string(),
            label: match &f.latin_name {
                Some(latin) if latin != &f.name => format!("{} ({latin})", f.name),
                _ => f.name,
            },
        })
        .collect())
}

/// Strata as dropdown options (preserving the seeded `sort_order`).
pub async fn list_strata_options(repo: &dyn Repository) -> AppResult<Vec<StrataOption>> {
    let strata = repo.strata_list().await?;
    Ok(strata
        .into_iter()
        .map(|s| StrataOption {
            id: s.id.to_string(),
            label: s.name,
        })
        .collect())
}

/// Payload for `create_crop`. Fields that only apply to certain lifespan
/// kinds are still present in every input (the UI parses every textbox
/// regardless); `create_crop` ignores irrelevant ones based on
/// `lifespan_kind`.
#[derive(Debug, Clone)]
pub struct CropInput {
    pub family_id_str: String,
    pub name: String,
    pub latin_name: Option<String>,
    pub lifespan_kind: LifespanKind,
    /// Used only when `lifespan_kind` is one of the pluriannual variants
    /// (must be ≥ 2 in that case; ignored for `Annual`).
    pub lifespan_years: u8,
    /// Used only when `lifespan_kind == PluriannualRecurring`
    /// (must be `< lifespan_years`; ignored otherwise).
    pub years_to_first_yield: u8,
    pub pruning_season: PruningSeason,
}

/// Create a crop with the lifespan + pruning season selected in the UI.
/// The concrete `Lifespan` variant is built from the input, then `Crop::new`
/// performs the field-level validation.
pub async fn create_crop(repo: &dyn Repository, input: CropInput) -> AppResult<Crop> {
    let family_id: FamilyId = crate::plantings_view::parse_id(&input.family_id_str)?;
    let lifespan = match input.lifespan_kind {
        LifespanKind::Annual => Lifespan::Annual,
        LifespanKind::PluriannualSingleCycle => {
            Lifespan::pluriannual_single_cycle(input.lifespan_years)?
        }
        LifespanKind::PluriannualRecurring => {
            Lifespan::perennial(input.lifespan_years, input.years_to_first_yield)?
        }
    };
    let crop = Crop::new(
        family_id,
        input.name,
        input.latin_name,
        lifespan,
        input.pruning_season,
    )?;
    repo.crop_create(&crop).await?;
    Ok(crop)
}

/// Payload for `create_variety`. Holds the union of fields for both profile
/// kinds; `create_variety` dispatches on `profile_kind` and ignores the
/// fields that don't apply.
#[derive(Debug, Clone)]
pub struct VarietyInput {
    pub crop_id_str: String,
    pub name: String,
    pub description: Option<String>,
    pub profile_kind: VarietyProfileKind,
    // Annual fields
    pub days_to_transplant: Option<u16>,
    pub days_to_maturity: u16,
    pub harvest_window_days: u16,
    // Pluriannual fields
    pub bud_break_doy: Option<u16>,
    pub flowering_doy: Option<u16>,
    pub harvest_start_doy: u16,
    pub harvest_end_doy: u16,
    pub expected_yield_kg_per_plant: Option<Decimal>,
}

/// Create a variety of an existing crop. Surfaces a clear `Inconsistent`
/// error when the UI sent a profile kind that doesn't match the parent crop's
/// lifespan (the domain layer would catch it anyway, but we want a better
/// message than the generic mismatch).
pub async fn create_variety(repo: &dyn Repository, input: VarietyInput) -> AppResult<Variety> {
    let crop_id: pomone_domain::CropId = crate::plantings_view::parse_id(&input.crop_id_str)?;
    let crop = repo
        .crop_get(crop_id)
        .await?
        .ok_or_else(|| AppError::NotFound {
            kind: "crop",
            id: crop_id.to_string(),
        })?;
    match (input.profile_kind, crop.lifespan.is_annual()) {
        (VarietyProfileKind::Annual, false) => {
            return Err(AppError::Inconsistent(
                "annual variety profile selected for a pluriannual crop".into(),
            ));
        }
        (VarietyProfileKind::Pluriannual, true) => {
            return Err(AppError::Inconsistent(
                "pluriannual variety profile selected for an annual crop".into(),
            ));
        }
        _ => {}
    }
    let profile = match input.profile_kind {
        VarietyProfileKind::Annual => VarietyProfile::Annual(AnnualProfile::new(
            input.days_to_transplant,
            input.days_to_maturity,
            input.harvest_window_days,
        )?),
        VarietyProfileKind::Pluriannual => VarietyProfile::Pluriannual(PluriannualProfile::new(
            input.bud_break_doy,
            input.flowering_doy,
            input.harvest_start_doy,
            input.harvest_end_doy,
            input.expected_yield_kg_per_plant,
        )?),
    };
    let variety = Variety::new(
        crop_id,
        crop.lifespan,
        input.name,
        input.description,
        profile,
    )?;
    repo.variety_create(&variety).await?;
    Ok(variety)
}

fn lifespan_label(lifespan: Lifespan) -> String {
    use pomone_domain::ProductivePattern;
    match lifespan {
        Lifespan::Annual => "Annuelle".to_owned(),
        Lifespan::Pluriannual {
            pattern: ProductivePattern::SingleCycle,
            lifespan_years,
        } => format!("Pluriannuelle cycle unique ({lifespan_years} ans)"),
        Lifespan::Pluriannual {
            pattern:
                ProductivePattern::Recurring {
                    years_to_first_yield,
                },
            lifespan_years,
        } => format!(
            "Pluriannuelle récurrente ({lifespan_years} ans, 1ère récolte +{years_to_first_yield})"
        ),
    }
}

fn pruning_label(pruning: PruningSeason) -> String {
    match pruning {
        PruningSeason::None => "Sans taille".to_owned(),
        PruningSeason::Winter => "Taille d'hiver".to_owned(),
        PruningSeason::Summer => "Taille d'été".to_owned(),
        PruningSeason::Both => "Taille hiver + été".to_owned(),
    }
}

fn variety_to_row(v: Variety) -> VarietyRow {
    let profile_label = match v.profile {
        VarietyProfile::Annual(p) => {
            let dtt = p
                .days_to_transplant
                .map_or_else(|| "—".to_owned(), |d| d.to_string());
            format!(
                "DTT {dtt} · DTM {} · fenêtre {}",
                p.days_to_maturity, p.harvest_window_days
            )
        }
        VarietyProfile::Pluriannual(p) => {
            format!("récolte DOY {}→{}", p.harvest_start_doy, p.harvest_end_doy)
        }
    };
    VarietyRow {
        id: v.id.to_string(),
        name: v.name,
        description: v.description.unwrap_or_default(),
        profile_label,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::seed_test_data;
    use pomone_db::{seed_defaults, SqliteRepository};

    async fn fresh_repo() -> SqliteRepository {
        let repo = SqliteRepository::in_memory().await.unwrap();
        seed_defaults(&repo).await.unwrap();
        seed_test_data(&repo).await.unwrap();
        repo
    }

    fn solanaceae_id_str(rows: &[FamilyOption]) -> String {
        rows.iter()
            .find(|f| f.label.contains("Solanacées"))
            .expect("seed includes Solanacées")
            .id
            .clone()
    }

    #[tokio::test]
    async fn list_crops_returns_seeded_tomato() {
        let repo = fresh_repo().await;
        let rows = list_crops(&repo).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, "Tomate");
        assert_eq!(rows[0].family_label, "Solanacées");
        assert_eq!(rows[0].lifespan_label, "Annuelle");
        assert_eq!(rows[0].variety_count, 2);
    }

    #[tokio::test]
    async fn list_varieties_for_crop_returns_seeded_pair() {
        let repo = fresh_repo().await;
        let crops = list_crops(&repo).await.unwrap();
        let rows = list_varieties_for_crop(&repo, &crops[0].id).await.unwrap();
        assert_eq!(rows.len(), 2);
        assert!(rows[0].profile_label.contains("DTM"));
    }

    #[tokio::test]
    async fn family_options_include_seeded_solanaceae() {
        let repo = fresh_repo().await;
        let rows = list_family_options(&repo).await.unwrap();
        assert!(rows.iter().any(|f| f.label.contains("Solanacées")));
    }

    #[tokio::test]
    async fn strata_options_preserve_seed_order() {
        let repo = fresh_repo().await;
        let rows = list_strata_options(&repo).await.unwrap();
        // First seeded strata in `pomone_db::seed::default_strata` is Canopée.
        assert_eq!(rows[0].label, "Canopée");
    }

    fn annual_crop_input(family_id: &str, name: &str) -> CropInput {
        CropInput {
            family_id_str: family_id.to_owned(),
            name: name.to_owned(),
            latin_name: None,
            lifespan_kind: LifespanKind::Annual,
            lifespan_years: 0,
            years_to_first_yield: 0,
            pruning_season: PruningSeason::None,
        }
    }

    fn annual_variety_input(crop_id: &str, name: &str) -> VarietyInput {
        VarietyInput {
            crop_id_str: crop_id.to_owned(),
            name: name.to_owned(),
            description: None,
            profile_kind: VarietyProfileKind::Annual,
            days_to_transplant: Some(35),
            days_to_maturity: 75,
            harvest_window_days: 55,
            bud_break_doy: None,
            flowering_doy: None,
            harvest_start_doy: 0,
            harvest_end_doy: 0,
            expected_yield_kg_per_plant: None,
        }
    }

    fn pluriannual_variety_input(crop_id: &str, name: &str) -> VarietyInput {
        VarietyInput {
            crop_id_str: crop_id.to_owned(),
            name: name.to_owned(),
            description: None,
            profile_kind: VarietyProfileKind::Pluriannual,
            days_to_transplant: None,
            days_to_maturity: 0,
            harvest_window_days: 0,
            bud_break_doy: Some(80),
            flowering_doy: Some(120),
            harvest_start_doy: 220,
            harvest_end_doy: 280,
            expected_yield_kg_per_plant: None,
        }
    }

    #[tokio::test]
    async fn create_annual_crop_persists_and_lists() {
        let repo = fresh_repo().await;
        let families = list_family_options(&repo).await.unwrap();
        let crop = create_crop(
            &repo,
            CropInput {
                latin_name: Some("Solanum melongena".to_owned()),
                ..annual_crop_input(&solanaceae_id_str(&families), "Aubergine")
            },
        )
        .await
        .unwrap();
        assert_eq!(crop.name, "Aubergine");
        assert!(crop.lifespan.is_annual());
        // Tomate (seed_demo) + Aubergine
        assert_eq!(list_crops(&repo).await.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn create_pluriannual_single_cycle_crop_persists() {
        let repo = fresh_repo().await;
        let families = list_family_options(&repo).await.unwrap();
        let crop = create_crop(
            &repo,
            CropInput {
                lifespan_kind: LifespanKind::PluriannualSingleCycle,
                lifespan_years: 2,
                ..annual_crop_input(&solanaceae_id_str(&families), "Carotte porte-graine")
            },
        )
        .await
        .unwrap();
        assert!(crop.lifespan.is_pluriannual());
        assert!(!crop.lifespan.is_recurring());
        assert_eq!(crop.lifespan.lifespan_years(), 2);
    }

    #[tokio::test]
    async fn create_pluriannual_recurring_crop_persists_with_pruning() {
        let repo = fresh_repo().await;
        let families = list_family_options(&repo).await.unwrap();
        let rosacees = families
            .iter()
            .find(|f| f.label.contains("Rosacées"))
            .unwrap()
            .id
            .clone();
        let crop = create_crop(
            &repo,
            CropInput {
                lifespan_kind: LifespanKind::PluriannualRecurring,
                lifespan_years: 40,
                years_to_first_yield: 3,
                pruning_season: PruningSeason::Winter,
                ..annual_crop_input(&rosacees, "Pommier")
            },
        )
        .await
        .unwrap();
        assert!(crop.lifespan.is_recurring());
        assert_eq!(crop.lifespan.lifespan_years(), 40);
        assert_eq!(crop.pruning_season, PruningSeason::Winter);
        let rows = list_crops(&repo).await.unwrap();
        let row = rows.iter().find(|r| r.name == "Pommier").unwrap();
        assert!(!row.is_annual);
        assert!(row.lifespan_label.contains("40"));
        assert!(row.pruning_label.contains("hiver"));
    }

    #[tokio::test]
    async fn create_pluriannual_recurring_rejects_first_yield_ge_lifespan() {
        let repo = fresh_repo().await;
        let families = list_family_options(&repo).await.unwrap();
        let err = create_crop(
            &repo,
            CropInput {
                lifespan_kind: LifespanKind::PluriannualRecurring,
                lifespan_years: 5,
                years_to_first_yield: 5,
                ..annual_crop_input(&solanaceae_id_str(&families), "Bad")
            },
        )
        .await
        .unwrap_err();
        assert!(matches!(err, AppError::Domain(_)));
    }

    #[tokio::test]
    async fn create_annual_variety_persists_under_crop() {
        let repo = fresh_repo().await;
        let crops = list_crops(&repo).await.unwrap();
        let _ = create_variety(
            &repo,
            VarietyInput {
                description: Some("Cœur boursouflé".to_owned()),
                ..annual_variety_input(&crops[0].id, "Cœur de bœuf")
            },
        )
        .await
        .unwrap();
        let rows = list_varieties_for_crop(&repo, &crops[0].id).await.unwrap();
        assert_eq!(rows.len(), 3);
        let new_row = rows.iter().find(|r| r.name == "Cœur de bœuf").unwrap();
        assert!(new_row.profile_label.contains("DTT 35"));
    }

    #[tokio::test]
    async fn create_pluriannual_variety_persists_under_recurring_crop() {
        let repo = fresh_repo().await;
        let families = list_family_options(&repo).await.unwrap();
        let rosacees = families
            .iter()
            .find(|f| f.label.contains("Rosacées"))
            .unwrap()
            .id
            .clone();
        let crop = create_crop(
            &repo,
            CropInput {
                lifespan_kind: LifespanKind::PluriannualRecurring,
                lifespan_years: 40,
                years_to_first_yield: 3,
                pruning_season: PruningSeason::Winter,
                ..annual_crop_input(&rosacees, "Pommier")
            },
        )
        .await
        .unwrap();
        let v = create_variety(
            &repo,
            VarietyInput {
                expected_yield_kg_per_plant: Some(Decimal::new(155, 1)),
                ..pluriannual_variety_input(&crop.id.to_string(), "Reine des Reinettes")
            },
        )
        .await
        .unwrap();
        assert_eq!(v.name, "Reine des Reinettes");
        let rows = list_varieties_for_crop(&repo, &crop.id.to_string())
            .await
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert!(rows[0].profile_label.contains("DOY 220"));
    }

    #[tokio::test]
    async fn create_annual_variety_rejects_pluriannual_crop() {
        let repo = fresh_repo().await;
        let families = list_family_options(&repo).await.unwrap();
        // Make a perennial crop via the public service so this also covers
        // create_crop's pluriannual path.
        let crop = create_crop(
            &repo,
            CropInput {
                lifespan_kind: LifespanKind::PluriannualRecurring,
                lifespan_years: 40,
                years_to_first_yield: 3,
                pruning_season: PruningSeason::Winter,
                ..annual_crop_input(&solanaceae_id_str(&families), "Pommier")
            },
        )
        .await
        .unwrap();
        let err = create_variety(&repo, annual_variety_input(&crop.id.to_string(), "Reine"))
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::Inconsistent(_)));
    }

    #[tokio::test]
    async fn create_pluriannual_variety_rejects_annual_crop() {
        let repo = fresh_repo().await;
        let crops = list_crops(&repo).await.unwrap();
        let err = create_variety(
            &repo,
            pluriannual_variety_input(&crops[0].id, "Pommier annuel"),
        )
        .await
        .unwrap_err();
        assert!(matches!(err, AppError::Inconsistent(_)));
    }

    #[tokio::test]
    async fn create_variety_unknown_crop() {
        let repo = fresh_repo().await;
        let err = create_variety(
            &repo,
            annual_variety_input(&pomone_domain::CropId::new().to_string(), "Test"),
        )
        .await
        .unwrap_err();
        assert!(matches!(err, AppError::NotFound { kind: "crop", .. }));
    }

    #[tokio::test]
    async fn list_varieties_rejects_invalid_crop_id() {
        let repo = fresh_repo().await;
        let err = list_varieties_for_crop(&repo, "not-a-uuid")
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::Inconsistent(_)));
    }
}
