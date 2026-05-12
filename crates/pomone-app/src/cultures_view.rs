//! Presentation-layer helpers for the Cultures/Varieties screen.
//!
//! Same shape as `plantings_view`: plain-string DTOs and parsers, so the
//! Slint UI never sees `Uuid` / `Decimal`. Listing helpers are intentionally
//! simple — one query per entity then in-memory joins via `HashMap`.

use crate::error::{AppError, AppResult};
use pomone_db::Repository;
use pomone_domain::{
    AnnualProfile, Crop, FamilyId, Lifespan, PruningSeason, StrataId, Variety, VarietyProfile,
};
use std::collections::HashMap;

/// One row of the Cultures list. Annual-vs-pluriannual is surfaced as a
/// short human string; richer renderings (pruning, lifespan years) will come
/// when we add the full Lifespan editor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CropRow {
    pub id: String,
    pub name: String,
    pub family_label: String,
    pub strata_label: String,
    pub lifespan_label: String,
    pub variety_count: u32,
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

/// Return crops sorted by name, each annotated with its family + strata
/// labels and the number of varieties currently attached.
pub async fn list_crops(repo: &dyn Repository) -> AppResult<Vec<CropRow>> {
    let mut crops = repo.crop_list().await?;
    crops.sort_by(|a, b| a.name.cmp(&b.name));

    let families = repo.family_list().await?;
    let strata = repo.strata_list().await?;
    let varieties = repo.variety_list().await?;

    let family_by_id: HashMap<_, _> = families.iter().map(|f| (f.id, f)).collect();
    let strata_by_id: HashMap<_, _> = strata.iter().map(|s| (s.id, s)).collect();
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
            strata_label: strata_by_id
                .get(&c.strata_id)
                .map_or_else(|| "?".to_owned(), |s| s.name.clone()),
            lifespan_label: lifespan_label(c.lifespan),
            variety_count: *variety_count.get(&c.id).unwrap_or(&0),
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

/// Validation-aware payload for `create_annual_crop`.
#[derive(Debug, Clone)]
pub struct AnnualCropInput {
    pub family_id_str: String,
    pub strata_id_str: String,
    pub name: String,
    pub latin_name: Option<String>,
}

/// Create an Annual crop with `PruningSeason::None` and persist it.
///
/// Pluriannual crops / non-None pruning seasons will require a richer form
/// and live in a follow-up PR.
pub async fn create_annual_crop(repo: &dyn Repository, input: AnnualCropInput) -> AppResult<Crop> {
    let family_id: FamilyId = crate::plantings_view::parse_id(&input.family_id_str)?;
    let strata_id: StrataId = crate::plantings_view::parse_id(&input.strata_id_str)?;
    let crop = Crop::new(
        family_id,
        strata_id,
        input.name,
        input.latin_name,
        Lifespan::Annual,
        PruningSeason::None,
    )?;
    repo.crop_create(&crop).await?;
    Ok(crop)
}

/// Validation-aware payload for `create_annual_variety`.
#[derive(Debug, Clone)]
pub struct AnnualVarietyInput {
    pub crop_id_str: String,
    pub name: String,
    pub description: Option<String>,
    pub days_to_transplant: Option<u16>,
    pub days_to_maturity: u16,
    pub harvest_window_days: u16,
}

/// Create a Variety of an existing Annual crop. Rejects pluriannual crops
/// (use a different code path with a `PluriannualProfile`).
pub async fn create_annual_variety(
    repo: &dyn Repository,
    input: AnnualVarietyInput,
) -> AppResult<Variety> {
    let crop_id: pomone_domain::CropId = crate::plantings_view::parse_id(&input.crop_id_str)?;
    let crop = repo
        .crop_get(crop_id)
        .await?
        .ok_or_else(|| AppError::NotFound {
            kind: "crop",
            id: crop_id.to_string(),
        })?;
    if !crop.lifespan.is_annual() {
        return Err(AppError::Inconsistent(
            "create_annual_variety called on a pluriannual crop".into(),
        ));
    }
    let profile = AnnualProfile::new(
        input.days_to_transplant,
        input.days_to_maturity,
        input.harvest_window_days,
    )?;
    let variety = Variety::new(
        crop_id,
        crop.lifespan,
        input.name,
        input.description,
        VarietyProfile::Annual(profile),
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
            pattern: ProductivePattern::Recurring { .. },
            lifespan_years,
        } => format!("Pluriannuelle récurrente ({lifespan_years} ans)"),
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
    use crate::plantings_view::seed_demo;
    use pomone_db::{seed_defaults, CropRepo, SqliteRepository};

    async fn fresh_repo() -> SqliteRepository {
        let repo = SqliteRepository::in_memory().await.unwrap();
        seed_defaults(&repo).await.unwrap();
        seed_demo(&repo).await.unwrap();
        repo
    }

    fn solanaceae_id_str(rows: &[FamilyOption]) -> String {
        rows.iter()
            .find(|f| f.label.contains("Solanacées"))
            .expect("seed includes Solanacées")
            .id
            .clone()
    }

    fn herbacee_id_str(rows: &[StrataOption]) -> String {
        rows.iter()
            .find(|s| s.label == "Herbacée")
            .expect("seed includes Herbacée")
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
        assert_eq!(rows[0].strata_label, "Herbacée");
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

    #[tokio::test]
    async fn create_annual_crop_persists_and_lists() {
        let repo = fresh_repo().await;
        let families = list_family_options(&repo).await.unwrap();
        let strata = list_strata_options(&repo).await.unwrap();
        let crop = create_annual_crop(
            &repo,
            AnnualCropInput {
                family_id_str: solanaceae_id_str(&families),
                strata_id_str: herbacee_id_str(&strata),
                name: "Aubergine".to_owned(),
                latin_name: Some("Solanum melongena".to_owned()),
            },
        )
        .await
        .unwrap();
        assert_eq!(crop.name, "Aubergine");
        let rows = list_crops(&repo).await.unwrap();
        // Tomate + Aubergine
        assert_eq!(rows.len(), 2);
    }

    #[tokio::test]
    async fn create_annual_variety_persists_under_crop() {
        let repo = fresh_repo().await;
        let crops = list_crops(&repo).await.unwrap();
        let _ = create_annual_variety(
            &repo,
            AnnualVarietyInput {
                crop_id_str: crops[0].id.clone(),
                name: "Cœur de bœuf".to_owned(),
                description: None,
                days_to_transplant: Some(35),
                days_to_maturity: 75,
                harvest_window_days: 55,
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
    async fn create_annual_variety_rejects_pluriannual_crop() {
        let repo = SqliteRepository::in_memory().await.unwrap();
        seed_defaults(&repo).await.unwrap();
        // Seed a perennial crop directly (no seed_demo here).
        let families = list_family_options(&repo).await.unwrap();
        let strata = list_strata_options(&repo).await.unwrap();
        let rosacees_id = families
            .iter()
            .find(|f| f.label.contains("Rosacées"))
            .expect("seed includes Rosacées")
            .id
            .clone();
        let canopee_id = strata
            .iter()
            .find(|s| s.label == "Canopée")
            .expect("seed includes Canopée")
            .id
            .clone();
        let family_id: FamilyId = crate::plantings_view::parse_id(&rosacees_id).unwrap();
        let strata_id: StrataId = crate::plantings_view::parse_id(&canopee_id).unwrap();
        let crop = Crop::new(
            family_id,
            strata_id,
            "Pommier",
            None,
            Lifespan::perennial(40, 3).unwrap(),
            PruningSeason::Winter,
        )
        .unwrap();
        repo.crop_create(&crop).await.unwrap();

        let err = create_annual_variety(
            &repo,
            AnnualVarietyInput {
                crop_id_str: crop.id.to_string(),
                name: "Reine".to_owned(),
                description: None,
                days_to_transplant: None,
                days_to_maturity: 60,
                harvest_window_days: 30,
            },
        )
        .await
        .unwrap_err();
        assert!(matches!(err, AppError::Inconsistent(_)));
    }

    #[tokio::test]
    async fn create_annual_variety_unknown_crop() {
        let repo = fresh_repo().await;
        let err = create_annual_variety(
            &repo,
            AnnualVarietyInput {
                crop_id_str: pomone_domain::CropId::new().to_string(),
                name: "Test".to_owned(),
                description: None,
                days_to_transplant: None,
                days_to_maturity: 60,
                harvest_window_days: 30,
            },
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
