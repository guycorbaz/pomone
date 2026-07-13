//! Presentation-layer helpers for the per-planting yearly-harvest table.
//!
//! Mirrors the other `*_view` modules: returns flat string DTOs so the
//! Slint UI doesn't see `Decimal`. Both `expected` and `actual` yields
//! are optional on the domain side; this module turns `None` into a dash
//! placeholder so the table column is always non-empty.

use crate::error::{AppError, AppResult};
use crate::units::MassUnit;
use pomone_db::Repository;
use pomone_domain::{PlantingId, YearlyHarvest};
use rust_decimal::Decimal;
use std::str::FromStr;
use uuid::Uuid;

/// One row of the yearly-harvest table on the Planting detail screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YearlyHarvestRow {
    pub year: i32,
    pub expected_label: String,
    pub actual_label: String,
    pub variance_label: String,
    pub notes: String,
}

/// Format an optional stored-kg value in the display unit, `"—"` when unset.
fn fmt_mass(opt: Option<Decimal>, unit: MassUnit) -> String {
    match opt {
        Some(v) => unit.format(v),
        None => "—".to_owned(),
    }
}

/// Format the variance (actual − expected) with a leading sign when both
/// sides are known, or `"—"` when one is missing.
fn fmt_variance(h: &YearlyHarvest, unit: MassUnit) -> String {
    match h.variance_kg() {
        Some(v) if v >= Decimal::ZERO => format!("+{}", unit.format(v)),
        Some(v) => unit.format(v),
        None => "—".to_owned(),
    }
}

fn to_row(h: &YearlyHarvest, unit: MassUnit) -> YearlyHarvestRow {
    YearlyHarvestRow {
        year: h.year,
        expected_label: fmt_mass(h.expected_yield_kg, unit),
        actual_label: fmt_mass(h.actual_yield_kg, unit),
        variance_label: fmt_variance(h, unit),
        notes: h.notes.clone().unwrap_or_default(),
    }
}

/// Return the harvests recorded for `planting_id_str`, sorted by year ascending.
pub async fn list_yearly_harvests_for_planting(
    repo: &dyn Repository,
    planting_id_str: &str,
    mass_unit: MassUnit,
) -> AppResult<Vec<YearlyHarvestRow>> {
    let uuid = Uuid::from_str(planting_id_str).map_err(|e| {
        AppError::Inconsistent(format!("invalid PlantingId '{planting_id_str}': {e}"))
    })?;
    let id = PlantingId::from(uuid);
    let mut harvests = repo.yearly_harvest_list_for_planting(id).await?;
    harvests.sort_by_key(|h| h.year);
    Ok(harvests.iter().map(|h| to_row(h, mass_unit)).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::{
        create_perennial_planting, record_yearly_harvest, PerennialPlantingRequest,
        YearlyHarvestRequest,
    };
    use chrono::NaiveDate;
    use pomone_db::{
        CropRepo, FamilyRepo, LocationKindRepo, LocationRepo, SqliteRepository, StrataRepo,
        VarietyRepo,
    };
    use pomone_domain::{
        Crop, Family, Lifespan, Location, LocationKind, PluriannualProfile, PruningSeason, Strata,
        Variety,
    };
    use rust_decimal_macros::dec;

    /// Build a fully seeded perennial setup ready for harvest tests.
    async fn setup_perennial() -> (SqliteRepository, String) {
        let repo = SqliteRepository::in_memory().await.unwrap();
        let f = Family::new("Rosaceae", None, None).unwrap();
        let s = Strata::new("Arborée", None, None, None, 500).unwrap();
        let k = LocationKind::new("Verger", None).unwrap();
        repo.family_create(&f).await.unwrap();
        repo.strata_create(&s).await.unwrap();
        repo.location_kind_create(&k).await.unwrap();
        let crop = Crop::new(
            f.id,
            "Pommier",
            None,
            Lifespan::perennial(40, 3).unwrap(),
            PruningSeason::Winter,
        )
        .unwrap();
        repo.crop_create(&crop).await.unwrap();
        let variety = Variety::new(
            crop.id,
            Lifespan::perennial(40, 3).unwrap(),
            "Reine des Reinettes",
            None,
            pomone_domain::VarietyProfile::Pluriannual(
                PluriannualProfile::new(Some(80), Some(120), 220, 280, None).unwrap(),
            ),
        )
        .unwrap();
        repo.variety_create(&variety).await.unwrap();
        let location = Location::new(k.id, "Verger Est", dec!(20), dec!(5), None, None).unwrap();
        repo.location_create(&location).await.unwrap();
        let planting = create_perennial_planting(
            &repo,
            PerennialPlantingRequest::new(
                variety.id,
                location.id,
                s.id,
                NaiveDate::from_ymd_opt(2026, 3, 15).unwrap(),
                dec!(100),
                10,
            ),
        )
        .await
        .unwrap();
        (repo, planting.id.to_string())
    }

    #[tokio::test]
    async fn list_is_empty_until_a_harvest_is_recorded() {
        let (repo, pid) = setup_perennial().await;
        let rows = list_yearly_harvests_for_planting(&repo, &pid, MassUnit::Kilograms)
            .await
            .unwrap();
        assert!(rows.is_empty());
    }

    #[tokio::test]
    async fn list_returns_rows_sorted_by_year_with_variance() {
        let (repo, pid) = setup_perennial().await;
        let uuid_id = pomone_domain::PlantingId::from(Uuid::from_str(&pid).unwrap());

        record_yearly_harvest(
            &repo,
            YearlyHarvestRequest::new(uuid_id, 2031)
                .with_expected_yield(dec!(50))
                .with_actual_yield(dec!(60)),
        )
        .await
        .unwrap();
        record_yearly_harvest(
            &repo,
            YearlyHarvestRequest::new(uuid_id, 2029)
                .with_expected_yield(dec!(40))
                .with_notes("ramp-up"),
        )
        .await
        .unwrap();
        record_yearly_harvest(
            &repo,
            YearlyHarvestRequest::new(uuid_id, 2030)
                .with_expected_yield(dec!(45))
                .with_actual_yield(dec!(30)),
        )
        .await
        .unwrap();

        let rows = list_yearly_harvests_for_planting(&repo, &pid, MassUnit::Kilograms)
            .await
            .unwrap();
        let years: Vec<_> = rows.iter().map(|r| r.year).collect();
        assert_eq!(years, vec![2029, 2030, 2031]);

        assert_eq!(rows[0].actual_label, "—");
        assert_eq!(rows[0].variance_label, "—");
        assert_eq!(rows[0].notes, "ramp-up");

        assert_eq!(rows[1].variance_label, "-15 kg");
        assert_eq!(rows[2].variance_label, "+10 kg");
    }

    #[tokio::test]
    async fn invalid_uuid_is_rejected_cleanly() {
        let repo = SqliteRepository::in_memory().await.unwrap();
        let err = list_yearly_harvests_for_planting(&repo, "not-a-uuid", MassUnit::Kilograms)
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::Inconsistent(_)));
    }
}
