//! Presentation-layer helpers for the (read-only) Planting detail screen.
//!
//! Produces a flat DTO that the UI can render without touching `Uuid` or
//! `chrono`. Schedule fields collapse into `DetailLine { label_key, value }`
//! tuples — the UI host resolves the Fluent key against its `I18n`
//! instance, keeping this module independent of the translation layer.

use crate::error::{AppError, AppResult};
use chrono::NaiveDate;
use pomone_db::Repository;
use pomone_domain::{Planting, PlantingId, PlantingSchedule};
use rust_decimal::Decimal;
use std::str::FromStr;
use uuid::Uuid;

/// One label/value row in the detail view. `label_key` is the Fluent
/// identifier; the host calls `i18n.t(label_key)` to get the displayed text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetailLine {
    pub label_key: &'static str,
    pub value: String,
}

/// Flattened, translation-ready representation of one Planting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlantingDetail {
    pub id: String,
    pub variety_label: String,
    pub location_label: String,
    pub area_label: String,
    pub plants_count: u32,
    pub name: Option<String>,
    pub notes: Option<String>,
    /// Schedule entries: sowing/transplant/harvest for `Cycle`, establishment
    /// + (optional) removal for `Perennial`. Always non-empty.
    pub schedule_lines: Vec<DetailLine>,
    /// True iff the underlying schedule is `Perennial`. The UI uses this to
    /// decide whether to render the yearly-harvest section.
    pub is_perennial: bool,
}

/// Format `Some(date)` as `YYYY-MM-DD`, `None` as a dash placeholder.
fn fmt_date_opt(d: Option<NaiveDate>) -> String {
    match d {
        Some(d) => d.format("%Y-%m-%d").to_string(),
        None => "—".to_owned(),
    }
}

fn fmt_area(area: Decimal) -> String {
    let s = area.normalize().to_string();
    format!("{s} m²")
}

fn schedule_lines(planting: &Planting) -> Vec<DetailLine> {
    match planting.schedule {
        PlantingSchedule::Cycle {
            sown_on,
            transplanted_on,
            first_harvest_on,
            last_harvest_on,
        } => vec![
            DetailLine {
                label_key: "label-sown-on",
                value: fmt_date_opt(sown_on),
            },
            DetailLine {
                label_key: "label-transplanted-on",
                value: fmt_date_opt(transplanted_on),
            },
            DetailLine {
                label_key: "label-first-harvest",
                value: first_harvest_on.format("%Y-%m-%d").to_string(),
            },
            DetailLine {
                label_key: "label-last-harvest",
                value: last_harvest_on.format("%Y-%m-%d").to_string(),
            },
        ],
        PlantingSchedule::Perennial {
            established_on,
            expected_removal_on,
        } => vec![
            DetailLine {
                label_key: "label-established-on",
                value: established_on.format("%Y-%m-%d").to_string(),
            },
            DetailLine {
                label_key: "label-removal-on",
                value: fmt_date_opt(expected_removal_on),
            },
        ],
    }
}

/// Load one Planting and resolve the labels needed by the detail screen.
/// Returns `NotFound` if no planting matches `id_str`.
pub async fn get_planting_detail(repo: &dyn Repository, id_str: &str) -> AppResult<PlantingDetail> {
    let uuid = Uuid::from_str(id_str)
        .map_err(|e| AppError::Inconsistent(format!("invalid PlantingId '{id_str}': {e}")))?;
    let planting_id = PlantingId::from(uuid);

    let planting = repo
        .planting_get(planting_id)
        .await?
        .ok_or_else(|| AppError::Inconsistent(format!("planting {id_str} not found")))?;

    let variety = repo
        .variety_get(planting.variety_id)
        .await?
        .ok_or_else(|| AppError::Inconsistent("planting refers to unknown variety".to_owned()))?;
    let crop = repo
        .crop_get(variety.crop_id)
        .await?
        .ok_or_else(|| AppError::Inconsistent("variety refers to unknown crop".to_owned()))?;
    let location = repo
        .location_get(planting.location_id)
        .await?
        .ok_or_else(|| AppError::Inconsistent("planting refers to unknown location".to_owned()))?;
    let parent_name = match location.parent_id {
        Some(pid) => repo.location_get(pid).await?.map(|l| l.name),
        None => None,
    };

    let variety_label = format!("{} · {}", crop.name, variety.name);
    let location_label = match parent_name {
        Some(p) => format!("{p} / {}", location.name),
        None => location.name.clone(),
    };

    let is_perennial = matches!(planting.schedule, PlantingSchedule::Perennial { .. });
    Ok(PlantingDetail {
        id: planting.id.to_string(),
        variety_label,
        location_label,
        area_label: fmt_area(planting.area_m2),
        plants_count: planting.plants_count,
        name: planting.name.clone(),
        notes: planting.notes.clone(),
        schedule_lines: schedule_lines(&planting),
        is_perennial,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::create_annual_planting_from_sowing;
    use crate::test_helpers::seed_test_data;
    use pomone_db::{seed_defaults, LocationRepo, SqliteRepository, VarietyRepo};
    use rust_decimal_macros::dec;

    async fn fresh_repo() -> SqliteRepository {
        let repo = SqliteRepository::in_memory().await.unwrap();
        seed_defaults(&repo).await.unwrap();
        repo
    }

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    #[tokio::test]
    async fn detail_of_annual_planting_contains_sowing_and_harvest_lines() {
        let repo = fresh_repo().await;
        seed_test_data(&repo).await.unwrap();

        let varieties = repo.variety_list().await.unwrap();
        let locations = repo.location_list().await.unwrap();
        let bed = locations.iter().find(|l| l.parent_id.is_some()).unwrap();
        let planting = create_annual_planting_from_sowing(
            &repo,
            varieties[0].id,
            bed.id,
            d(2026, 3, 1),
            dec!(20),
            100,
            Some("démo".to_owned()),
            Some("notes".to_owned()),
        )
        .await
        .unwrap();

        let detail = get_planting_detail(&repo, &planting.id.to_string())
            .await
            .unwrap();

        assert!(detail.variety_label.starts_with("Tomate · "));
        assert!(detail.location_label.contains(" / "));
        assert_eq!(detail.plants_count, 100);
        assert_eq!(detail.name.as_deref(), Some("démo"));
        assert_eq!(detail.notes.as_deref(), Some("notes"));
        let keys: Vec<_> = detail.schedule_lines.iter().map(|l| l.label_key).collect();
        assert!(keys.contains(&"label-sown-on"));
        assert!(keys.contains(&"label-first-harvest"));
        assert!(keys.contains(&"label-last-harvest"));
    }

    #[tokio::test]
    async fn missing_planting_returns_not_found() {
        let repo = fresh_repo().await;
        let fake = uuid::Uuid::new_v4().to_string();
        let err = get_planting_detail(&repo, &fake).await.unwrap_err();
        assert!(matches!(err, AppError::Inconsistent(_)));
    }

    #[tokio::test]
    async fn invalid_uuid_is_rejected_cleanly() {
        let repo = fresh_repo().await;
        let err = get_planting_detail(&repo, "not-a-uuid").await.unwrap_err();
        assert!(matches!(err, AppError::Inconsistent(_)));
    }
}
