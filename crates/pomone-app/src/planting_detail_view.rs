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
use std::collections::HashMap;
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

/// One task attached to a planting, ready to render in the detail screen's
/// task list. Mirrors the calendar's `TaskCalendarRow` shape but trades the
/// typed `TaskId`/`category` for plain strings the UI can forward directly,
/// and adds an `overdue` flag the calendar derives from cell position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlantingTaskRow {
    pub task_id: String,
    /// ISO-8601 (`YYYY-MM-DD`) planned date.
    pub planned_on: String,
    /// Localized name of the task type (`"Semis"`, `"Désherbage"`…).
    pub type_name: String,
    /// Hex color from the `TaskType` (e.g. `"#3C6E47"`), for the row's dot.
    pub color: String,
    /// `true` if the task has a `completed_on` date set.
    pub completed: bool,
    /// `true` if still pending and planned in the past (relative to `today`).
    pub overdue: bool,
    /// Free-text notes, empty when unset.
    pub notes: String,
}

/// List the tasks anchored to one planting, decorated with their type
/// metadata and sorted pending-first then by planned date. `today` drives the
/// `overdue` flag so callers stay deterministic in tests.
///
/// Returns an empty vec for a planting with no tasks; an unknown/invalid id is
/// surfaced as `Inconsistent` (same contract as [`get_planting_detail`]).
pub async fn list_planting_tasks(
    repo: &dyn Repository,
    id_str: &str,
    today: NaiveDate,
) -> AppResult<Vec<PlantingTaskRow>> {
    let uuid = Uuid::from_str(id_str)
        .map_err(|e| AppError::Inconsistent(format!("invalid PlantingId '{id_str}': {e}")))?;
    let planting_id = PlantingId::from(uuid);

    let tasks = repo.task_list_for_planting(planting_id).await?;
    let types = repo.task_type_list().await?;
    let types_by_id: HashMap<_, _> = types.iter().map(|t| (t.id, t)).collect();

    let mut rows: Vec<PlantingTaskRow> = tasks
        .into_iter()
        .filter_map(|task| {
            // Orphan task whose type was deleted — skip rather than crash.
            // (FK is RESTRICT so this shouldn't happen, but stay defensive.)
            let tt = types_by_id.get(&task.task_type_id)?;
            Some(PlantingTaskRow {
                task_id: task.id.to_string(),
                planned_on: task.planned_on.format("%Y-%m-%d").to_string(),
                type_name: tt.name.clone(),
                color: tt.color.clone(),
                completed: task.completed_on.is_some(),
                overdue: task.is_overdue(today),
                notes: task.notes.unwrap_or_default(),
            })
        })
        .collect();
    // Pending tasks first (false < true), each group by ascending date.
    rows.sort_by(|a, b| {
        a.completed
            .cmp(&b.completed)
            .then(a.planned_on.cmp(&b.planned_on))
    });
    Ok(rows)
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
    async fn planting_tasks_are_decorated_sorted_and_flag_overdue() {
        use crate::tasks_view::create_task;
        use pomone_db::TaskTypeRepo;
        use pomone_domain::TaskCategory;

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
            None,
            None,
        )
        .await
        .unwrap();
        let pid = planting.id.to_string();

        // Auto-gen already created sow/transplant/harvest tasks; add a manual
        // weeding pending in the past so we can assert the overdue flag.
        let weeding = repo
            .task_type_list()
            .await
            .unwrap()
            .into_iter()
            .find(|t| t.category == TaskCategory::Weeding)
            .unwrap();
        create_task(
            &repo,
            &pid,
            &weeding.id.to_string(),
            "2026-04-15",
            "binage",
            false,
            d(2026, 5, 1),
        )
        .await
        .unwrap();

        let today = d(2026, 5, 1);
        let rows = list_planting_tasks(&repo, &pid, today).await.unwrap();

        // At least the manual weeding + the auto-gen tasks are present.
        assert!(rows.len() >= 2);
        // Pending tasks come before completed ones.
        let first_completed = rows.iter().position(|r| r.completed);
        if let Some(idx) = first_completed {
            assert!(rows[..idx].iter().all(|r| !r.completed));
        }
        let weed = rows.iter().find(|r| r.type_name == "Désherbage").unwrap();
        assert_eq!(weed.planned_on, "2026-04-15");
        assert!(weed.overdue, "past pending task should be overdue");
        assert!(weed.color.starts_with('#'));
        assert_eq!(weed.notes, "binage");
    }

    #[tokio::test]
    async fn planting_tasks_rejects_invalid_id() {
        let repo = fresh_repo().await;
        let err = list_planting_tasks(&repo, "not-a-uuid", d(2026, 5, 1))
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::Inconsistent(_)));
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
