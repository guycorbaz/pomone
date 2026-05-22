//! Presentation-layer helpers for the Plantings screen.
//!
//! Pulls plain-text DTOs out of the repository so the Slint UI can render
//! them without depending on `Uuid`, `NaiveDate`, `Decimal`, or the domain
//! enum shapes. The domain layer stays pure; only this module knows about
//! UI-friendly formatting.

use crate::error::{AppError, AppResult};
use chrono::NaiveDate;
use pomone_db::Repository;
use pomone_domain::{Location, LocationId};
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::str::FromStr;
use uuid::Uuid;

/// A variety entry suitable for a dropdown: stable stringified UUID + human
/// label, plus the `is_annual` hint so callers can pre-validate before invoking
/// `create_annual_planting_from_sowing`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VarietyOption {
    pub id: String,
    pub label: String,
    pub is_annual: bool,
}

/// A location entry suitable for a dropdown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocationOption {
    pub id: String,
    pub label: String,
}

/// A planting row suitable for the list view.
///
/// `cycle_dates` is `Some(...)` only for annual `PlantingSchedule::Cycle`
/// plantings; perennials carry it as `None` because their multi-year span
/// doesn't fit the single-season Gantt axis (the Yearly Harvests view is
/// the right place for them). The four dates inside `CycleDates` mirror
/// the domain enum field-for-field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlantingRow {
    pub id: String,
    pub variety_label: String,
    pub location_label: String,
    pub schedule_summary: String,
    pub area_label: String,
    pub plants_count: u32,
    pub cycle_dates: Option<CycleDates>,
}

/// Raw dates from `PlantingSchedule::Cycle`, surfaced so the UI's Gantt
/// timeline can position each planting's segments without re-parsing the
/// formatted `schedule_summary`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CycleDates {
    pub sown_on: Option<chrono::NaiveDate>,
    pub transplanted_on: Option<chrono::NaiveDate>,
    pub first_harvest_on: chrono::NaiveDate,
    pub last_harvest_on: chrono::NaiveDate,
}

/// Format a `Decimal` area as a French-style number with `m²` suffix.
fn format_area(area: Decimal) -> String {
    // Normalise trailing zeros: "20.000" → "20", "12.500" → "12.5".
    let s = area.normalize().to_string();
    format!("{s} m²")
}

/// Format the start/end of a planting in a compact summary line.
fn schedule_summary(planting: &pomone_domain::Planting) -> String {
    use pomone_domain::PlantingSchedule;
    match planting.schedule {
        PlantingSchedule::Cycle {
            sown_on,
            transplanted_on,
            first_harvest_on,
            last_harvest_on,
        } => {
            let mut parts: Vec<String> = Vec::new();
            if let Some(d) = sown_on {
                parts.push(format!("semis {d}"));
            }
            if let Some(d) = transplanted_on {
                parts.push(format!("repiquage {d}"));
            }
            parts.push(format!("récolte {first_harvest_on} → {last_harvest_on}"));
            parts.join(" · ")
        }
        PlantingSchedule::Perennial {
            established_on,
            expected_removal_on,
        } => match expected_removal_on {
            Some(r) => format!("plantation {established_on} → arrachage prévu {r}"),
            None => format!("plantation {established_on}"),
        },
    }
}

/// Return the list of varieties as labelled options for a dropdown.
///
/// Labels are `"<Crop> · <Variety>"` so the user sees what crop a variety
/// belongs to without an extra column.
pub async fn list_variety_options(repo: &dyn Repository) -> AppResult<Vec<VarietyOption>> {
    let crops = repo.crop_list().await?;
    let crops_by_id: HashMap<_, _> = crops.iter().map(|c| (c.id, c)).collect();

    let mut varieties = repo.variety_list().await?;
    varieties.sort_by(|a, b| {
        let crop_a = crops_by_id.get(&a.crop_id).map_or("", |c| c.name.as_str());
        let crop_b = crops_by_id.get(&b.crop_id).map_or("", |c| c.name.as_str());
        crop_a.cmp(crop_b).then_with(|| a.name.cmp(&b.name))
    });

    let options = varieties
        .into_iter()
        .map(|v| {
            let crop_name = crops_by_id.get(&v.crop_id).map_or("?", |c| c.name.as_str());
            let is_annual = crops_by_id
                .get(&v.crop_id)
                .is_some_and(|c| c.lifespan.is_annual());
            VarietyOption {
                id: v.id.to_string(),
                label: format!("{crop_name} · {}", v.name),
                is_annual,
            }
        })
        .collect();
    Ok(options)
}

/// Return the list of locations as labelled options for a dropdown.
///
/// Labels include the parent name when present: `"<Parent> / <Location>"`.
pub async fn list_location_options(repo: &dyn Repository) -> AppResult<Vec<LocationOption>> {
    let mut locations = repo.location_list().await?;
    locations.sort_by(|a, b| a.name.cmp(&b.name));
    let by_id: HashMap<LocationId, &Location> = locations.iter().map(|l| (l.id, l)).collect();

    let options = locations
        .iter()
        .map(|l| {
            let label = match l.parent_id.and_then(|p| by_id.get(&p)) {
                Some(parent) => format!("{} / {}", parent.name, l.name),
                None => l.name.clone(),
            };
            LocationOption {
                id: l.id.to_string(),
                label,
            }
        })
        .collect();
    Ok(options)
}

/// Return the plantings table for the list view. One DB read per planting to
/// resolve variety+crop+location names; fine for the dataset sizes Pomone
/// targets (tens to a few hundred plantings per farm), can be replaced with a
/// JOIN once profiles say so.
pub async fn list_plantings(repo: &dyn Repository) -> AppResult<Vec<PlantingRow>> {
    let plantings = repo.planting_list().await?;
    let varieties = repo.variety_list().await?;
    let crops = repo.crop_list().await?;
    let locations = repo.location_list().await?;

    let var_by_id: HashMap<_, _> = varieties.iter().map(|v| (v.id, v)).collect();
    let crop_by_id: HashMap<_, _> = crops.iter().map(|c| (c.id, c)).collect();
    let loc_by_id: HashMap<_, _> = locations.iter().map(|l| (l.id, l)).collect();

    let mut rows: Vec<PlantingRow> = plantings
        .iter()
        .map(|p| {
            let variety_label = var_by_id.get(&p.variety_id).map_or_else(
                || "?".to_owned(),
                |v| {
                    let crop_name = crop_by_id.get(&v.crop_id).map_or("?", |c| c.name.as_str());
                    format!("{crop_name} · {}", v.name)
                },
            );
            let location_label = loc_by_id.get(&p.location_id).map_or_else(
                || "?".to_owned(),
                |l| match l.parent_id.and_then(|pid| loc_by_id.get(&pid)) {
                    Some(parent) => format!("{} / {}", parent.name, l.name),
                    None => l.name.clone(),
                },
            );
            let cycle_dates = match p.schedule {
                pomone_domain::PlantingSchedule::Cycle {
                    sown_on,
                    transplanted_on,
                    first_harvest_on,
                    last_harvest_on,
                } => Some(CycleDates {
                    sown_on,
                    transplanted_on,
                    first_harvest_on,
                    last_harvest_on,
                }),
                pomone_domain::PlantingSchedule::Perennial { .. } => None,
            };
            PlantingRow {
                id: p.id.to_string(),
                variety_label,
                location_label,
                schedule_summary: schedule_summary(p),
                area_label: format_area(p.area_m2),
                plants_count: p.plants_count,
                cycle_dates,
            }
        })
        .collect();

    rows.sort_by(|a, b| a.variety_label.cmp(&b.variety_label));
    Ok(rows)
}

/// Parse a string UUID back into a domain-typed ID. Returns `Inconsistent` if
/// the string isn't a valid UUID (the UI should never produce one).
pub fn parse_id<T: From<Uuid>>(s: &str) -> AppResult<T> {
    let uuid = Uuid::from_str(s)
        .map_err(|e| AppError::Inconsistent(format!("invalid UUID from UI '{s}': {e}")))?;
    Ok(T::from(uuid))
}

/// Parse `YYYY-MM-DD` into a NaiveDate. Returns `Inconsistent` for any other
/// input; the UI is expected to validate the field shape before submitting.
pub fn parse_iso_date(s: &str) -> AppResult<NaiveDate> {
    NaiveDate::parse_from_str(s.trim(), "%Y-%m-%d").map_err(|e| {
        AppError::Inconsistent(format!(
            "expected date in YYYY-MM-DD format, got '{s}': {e}"
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::create_annual_planting_from_sowing;
    use crate::test_helpers::seed_test_data;
    use pomone_db::{seed_defaults, LocationRepo, SqliteRepository, VarietyRepo};
    use pomone_domain::{LocationId, VarietyId};
    use rust_decimal_macros::dec;

    async fn fresh_repo() -> SqliteRepository {
        let repo = SqliteRepository::in_memory().await.unwrap();
        seed_defaults(&repo).await.unwrap();
        repo
    }

    #[tokio::test]
    async fn variety_options_labelled_with_crop_name() {
        let repo = fresh_repo().await;
        seed_test_data(&repo).await.unwrap();
        let opts = list_variety_options(&repo).await.unwrap();
        assert_eq!(opts.len(), 2);
        assert!(opts.iter().all(|o| o.label.starts_with("Tomate · ")));
        assert!(opts.iter().all(|o| o.is_annual));
    }

    #[tokio::test]
    async fn location_options_include_parent_prefix() {
        let repo = fresh_repo().await;
        seed_test_data(&repo).await.unwrap();
        let opts = list_location_options(&repo).await.unwrap();
        let child = opts.iter().find(|o| o.label.contains(" / ")).unwrap();
        assert!(child.label.starts_with("Jardin Pomone / "));
    }

    #[tokio::test]
    async fn plantings_list_is_empty_on_fresh_seed() {
        let repo = fresh_repo().await;
        seed_test_data(&repo).await.unwrap();
        let rows = list_plantings(&repo).await.unwrap();
        assert!(rows.is_empty());
    }

    #[tokio::test]
    async fn plantings_list_returns_a_row_after_create() {
        let repo = fresh_repo().await;
        seed_test_data(&repo).await.unwrap();
        let varieties = repo.variety_list().await.unwrap();
        let locations = repo.location_list().await.unwrap();
        let bed = locations.iter().find(|l| l.parent_id.is_some()).unwrap();
        let date = NaiveDate::from_ymd_opt(2026, 3, 1).unwrap();
        let _ = create_annual_planting_from_sowing(
            &repo,
            varieties[0].id,
            bed.id,
            date,
            dec!(20),
            100,
            Some("démo".to_owned()),
            None,
        )
        .await
        .unwrap();
        let rows = list_plantings(&repo).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert!(rows[0].variety_label.starts_with("Tomate · "));
        assert!(rows[0].location_label.contains(" / "));
        assert!(rows[0].schedule_summary.contains("semis"));
        assert_eq!(rows[0].plants_count, 100);
    }

    #[test]
    fn parse_iso_date_accepts_iso_format() {
        let d = parse_iso_date("2026-03-01").unwrap();
        assert_eq!(d, NaiveDate::from_ymd_opt(2026, 3, 1).unwrap());
    }

    #[test]
    fn parse_iso_date_rejects_garbage() {
        assert!(parse_iso_date("nope").is_err());
        assert!(parse_iso_date("2026/03/01").is_err());
    }

    #[test]
    fn parse_id_rejects_non_uuid() {
        let res: AppResult<VarietyId> = parse_id("not-a-uuid");
        assert!(matches!(res, Err(AppError::Inconsistent(_))));
    }

    #[test]
    fn parse_id_roundtrips_via_display() {
        let id = LocationId::new();
        let back: LocationId = parse_id(&id.to_string()).unwrap();
        assert_eq!(back, id);
    }
}
