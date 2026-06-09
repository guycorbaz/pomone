//! Presentation-layer helpers for manual Task CRUD.
//!
//! Wraps the `TaskRepo` and `TaskTypeRepo` traits behind flat string-based
//! DTOs so the Slint UI can create, edit, and delete tasks without touching
//! `Uuid`, `NaiveDate`, or the domain types directly.
//!
//! Auto-generated tasks (sow/transplant/harvest at planting creation) are
//! produced by [`crate::task_autogen`]; this module is the user-facing path
//! for everything else: editing those auto-tasks, adding ad-hoc operations
//! (weeding, irrigation…), and removing them.

use crate::error::{AppError, AppResult};
use crate::plantings_view::parse_id;
use chrono::{Months, NaiveDate};
use pomone_db::Repository;
use pomone_domain::{
    PlantingId, RecurrenceRule, RecurrenceUnit, Task, TaskCategory, TaskId, TaskSeries,
    TaskSeriesId, TaskType, TaskTypeId,
};
use std::collections::{HashMap, HashSet};

/// One task type entry for the form's type dropdown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskTypeOption {
    pub id: String,
    pub name: String,
    /// Hex color (`"#3C6E47"`), forwarded to Slint so the pill matches.
    pub color: String,
    /// Stable category — exposed so the UI can group/filter later. Serialized
    /// as the same snake-case string used by the codec.
    pub category: String,
}

/// One planting entry for the form's "attached to" dropdown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlantingChoice {
    pub id: String,
    /// Compact label: `"<Crop> · <Variety> — <Location>"`.
    pub label: String,
}

/// Pre-filled state for editing an existing task. `completed` collapses the
/// domain's `Option<NaiveDate>` into a bool — the UI keeps the date as a
/// hidden detail; toggling the checkbox sets `completed_on` to today
/// (`save_task`'s caller decides which "today" to use).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskEditForm {
    pub task_id: String,
    pub task_type_id: String,
    /// Empty string when the task isn't anchored to a planting.
    pub planting_id: String,
    /// ISO-8601 (`YYYY-MM-DD`).
    pub planned_on: String,
    pub completed: bool,
    pub notes: String,
    /// `true` when the task was materialized from a recurring series.
    /// The UI uses this to surface a "🔁 part of a series" badge in
    /// edit mode (the in-place form intentionally does NOT let the user
    /// reshape the series — that flow happens elsewhere).
    pub is_part_of_series: bool,
}

/// List all task types as dropdown options, sorted by name (matches the
/// SQL order in `TaskTypeRepo::task_type_list`).
pub async fn list_task_type_options(repo: &dyn Repository) -> AppResult<Vec<TaskTypeOption>> {
    let types = repo.task_type_list().await?;
    Ok(types.into_iter().map(to_type_option).collect())
}

fn to_type_option(t: TaskType) -> TaskTypeOption {
    TaskTypeOption {
        id: t.id.to_string(),
        name: t.name,
        color: t.color,
        category: category_str(t.category).to_owned(),
    }
}

/// Map the domain `TaskCategory` to its codec string. Kept in sync with the
/// `task_category_to_str` helper in `pomone-db`. Shared with
/// `task_types_view` so the catalog editor presents the same labels.
pub(crate) fn category_str(c: TaskCategory) -> &'static str {
    match c {
        TaskCategory::Sow => "sow",
        TaskCategory::Transplant => "transplant",
        TaskCategory::Harvest => "harvest",
        TaskCategory::Weeding => "weeding",
        TaskCategory::Irrigation => "irrigation",
        TaskCategory::Treatment => "treatment",
        TaskCategory::Tillage => "tillage",
        TaskCategory::Other => "other",
    }
}

/// Inverse of [`category_str`]. Returns `None` for unknown strings so the
/// caller can decide whether to error or silently fall back.
pub(crate) fn category_from_str(s: &str) -> Option<TaskCategory> {
    match s {
        "sow" => Some(TaskCategory::Sow),
        "transplant" => Some(TaskCategory::Transplant),
        "harvest" => Some(TaskCategory::Harvest),
        "weeding" => Some(TaskCategory::Weeding),
        "irrigation" => Some(TaskCategory::Irrigation),
        "treatment" => Some(TaskCategory::Treatment),
        "tillage" => Some(TaskCategory::Tillage),
        "other" => Some(TaskCategory::Other),
        _ => None,
    }
}

/// List plantings as dropdown choices. Reuses the same name resolution as
/// `list_plantings`; the label is intentionally compact so it fits a
/// single-line ComboBox row.
pub async fn list_planting_choices(repo: &dyn Repository) -> AppResult<Vec<PlantingChoice>> {
    let plantings = repo.planting_list().await?;
    let varieties = repo.variety_list().await?;
    let crops = repo.crop_list().await?;
    let locations = repo.location_list().await?;

    let var_by_id: HashMap<_, _> = varieties.iter().map(|v| (v.id, v)).collect();
    let crop_by_id: HashMap<_, _> = crops.iter().map(|c| (c.id, c)).collect();
    let loc_by_id: HashMap<_, _> = locations.iter().map(|l| (l.id, l)).collect();

    let mut out: Vec<PlantingChoice> = plantings
        .iter()
        .map(|p| {
            let variety_label = var_by_id.get(&p.variety_id).map_or_else(
                || "?".to_owned(),
                |v| {
                    let crop_name = crop_by_id.get(&v.crop_id).map_or("?", |c| c.name.as_str());
                    format!("{crop_name} · {}", v.name)
                },
            );
            let location_label = loc_by_id
                .get(&p.location_id)
                .map_or_else(|| "?".to_owned(), |l| l.name.clone());
            PlantingChoice {
                id: p.id.to_string(),
                label: format!("{variety_label} — {location_label}"),
            }
        })
        .collect();
    out.sort_by(|a, b| a.label.cmp(&b.label));
    Ok(out)
}

/// Load one task and shape it for the edit form. `NotFound` if the task is
/// gone (already deleted in another window, stale ID from the calendar…).
pub async fn get_task_for_edit(
    repo: &dyn Repository,
    task_id_str: &str,
) -> AppResult<TaskEditForm> {
    let id: TaskId = parse_id(task_id_str)?;
    let task = repo.task_get(id).await?.ok_or_else(|| AppError::NotFound {
        kind: "task",
        id: task_id_str.to_owned(),
    })?;
    Ok(TaskEditForm {
        task_id: task.id.to_string(),
        task_type_id: task.task_type_id.to_string(),
        planting_id: task.planting_id.map(|p| p.to_string()).unwrap_or_default(),
        planned_on: task.planned_on.format("%Y-%m-%d").to_string(),
        completed: task.completed_on.is_some(),
        notes: task.notes.unwrap_or_default(),
        is_part_of_series: task.series_id.is_some(),
    })
}

/// Create a new task. When `planting_id_str` is non-empty the task inherits
/// that planting's location; pass an empty string for a free-standing task
/// (admin, training, generic farm work).
///
/// `completed_on` is `Some(today)` iff `completed` is true; the caller
/// (Slint side) supplies "today" so this function stays deterministic for
/// tests.
#[allow(clippy::too_many_arguments)]
pub async fn create_task(
    repo: &dyn Repository,
    planting_id_str: &str,
    task_type_id_str: &str,
    planned_on_iso: &str,
    notes: &str,
    completed: bool,
    today: NaiveDate,
) -> AppResult<String> {
    let task_type_id: TaskTypeId = parse_id(task_type_id_str)?;
    let planned_on = parse_iso_date_local(planned_on_iso)?;
    let (planting_id, location_id) = resolve_planting(repo, planting_id_str).await?;
    let completed_on = if completed { Some(today) } else { None };
    let notes_opt = empty_to_none(notes);

    let task = Task::new(
        planting_id,
        location_id,
        task_type_id,
        None,
        None,
        planned_on,
        completed_on,
        None,
        None,
        notes_opt,
    );
    repo.task_create(&task).await?;
    Ok(task.id.to_string())
}

/// Update an existing task's editable fields. The planting attachment is
/// *not* editable here — moving a task between plantings is rare and would
/// require reloading the form; we'd rather force a delete + recreate to
/// keep the contract narrow.
pub async fn update_task(
    repo: &dyn Repository,
    task_id_str: &str,
    task_type_id_str: &str,
    planned_on_iso: &str,
    notes: &str,
    completed: bool,
    today: NaiveDate,
) -> AppResult<()> {
    let id: TaskId = parse_id(task_id_str)?;
    let task_type_id: TaskTypeId = parse_id(task_type_id_str)?;
    let planned_on = parse_iso_date_local(planned_on_iso)?;
    let notes_opt = empty_to_none(notes);

    let existing = repo.task_get(id).await?.ok_or_else(|| AppError::NotFound {
        kind: "task",
        id: task_id_str.to_owned(),
    })?;
    // Preserve a non-today completion date if the user keeps the checkbox
    // on (so re-saving a task already marked done last week doesn't shift
    // the date). Only flip None ↔ Some(today) when the bool actually changed.
    let completed_on = match (existing.completed_on, completed) {
        (Some(prev), true) => Some(prev),
        (None, true) => Some(today),
        (_, false) => None,
    };

    let updated = Task {
        id: existing.id,
        planting_id: existing.planting_id,
        location_id: existing.location_id,
        task_type_id,
        task_method_id: existing.task_method_id,
        implement_id: existing.implement_id,
        series_id: existing.series_id,
        planned_on,
        completed_on,
        duration_min: existing.duration_min,
        labor_hours: existing.labor_hours,
        notes: notes_opt,
    };
    repo.task_update(&updated).await?;
    Ok(())
}

/// Delete a task. Maps the underlying DB `NotFound` to the app-level variant
/// so the UI can show a coherent message even if the task vanished between
/// the click and the call.
pub async fn delete_task(repo: &dyn Repository, task_id_str: &str) -> AppResult<()> {
    let id: TaskId = parse_id(task_id_str)?;
    repo.task_delete(id).await?;
    Ok(())
}

/// Stable string for [`RecurrenceUnit`] — mirrors the DB codec so the UI
/// can ship the user's choice as a plain key.
#[must_use]
pub fn recurrence_unit_str(u: RecurrenceUnit) -> &'static str {
    match u {
        RecurrenceUnit::Days => "days",
        RecurrenceUnit::Weeks => "weeks",
        RecurrenceUnit::Months => "months",
    }
}

fn recurrence_unit_from(key: &str) -> Option<RecurrenceUnit> {
    match key {
        "days" => Some(RecurrenceUnit::Days),
        "weeks" => Some(RecurrenceUnit::Weeks),
        "months" => Some(RecurrenceUnit::Months),
        _ => None,
    }
}

/// Rolling horizon used to cap open-ended series (no `end_on`). One year
/// past the supplied "today" — enough to keep the calendar populated for
/// the foreseeable usage horizon without exploding row counts.
///
/// Uses `checked_add_months`, which clamps an overflowing day to the last day
/// of the target month (so a leap-day "today" like 2024-02-29 maps to
/// 2025-02-28). The naive `from_ymd_opt(year+1, month, day)` it replaced
/// returned `None` on a leap day and collapsed the horizon to `today`,
/// silently halting occurrence materialization for that day (issue #60).
fn extend_horizon(today: NaiveDate) -> NaiveDate {
    today.checked_add_months(Months::new(12)).unwrap_or(today)
}

/// Create a new recurring task series and materialize every occurrence
/// between `first_planned_on` and `min(end_on, extend_horizon(today))`.
///
/// The series carries the recurrence rule + shared metadata ; each
/// occurrence is a regular [`Task`] row whose `series_id` points back
/// to the series, so the calendar / filter / CRUD code paths keep
/// working unchanged.
#[allow(clippy::too_many_arguments)]
pub async fn create_recurring_task(
    repo: &dyn Repository,
    planting_id_str: &str,
    task_type_id_str: &str,
    planned_on_iso: &str,
    notes: &str,
    interval: u32,
    unit_key: &str,
    end_on_iso: Option<&str>,
    today: NaiveDate,
) -> AppResult<String> {
    let task_type_id: TaskTypeId = parse_id(task_type_id_str)?;
    let first_planned_on = parse_iso_date_local(planned_on_iso)?;
    let (planting_id, location_id) = resolve_planting(repo, planting_id_str).await?;
    let unit = recurrence_unit_from(unit_key)
        .ok_or_else(|| AppError::Inconsistent(format!("unknown recurrence unit '{unit_key}'")))?;
    let end_on = match end_on_iso {
        Some(s) if !s.trim().is_empty() => Some(parse_iso_date_local(s)?),
        _ => None,
    };
    let rule = RecurrenceRule::new(unit, interval, end_on)?;
    let notes_opt = empty_to_none(notes);

    let series = TaskSeries::new(
        planting_id,
        location_id,
        task_type_id,
        None,
        None,
        rule,
        first_planned_on,
        notes_opt.clone(),
    );
    repo.task_series_create(&series).await?;
    materialize_occurrences(repo, &series, today, &HashSet::new()).await?;
    Ok(series.id.to_string())
}

/// Walk every active series and top up its occurrences up to
/// `extend_horizon(today)`. Idempotent — each call dedupes against the
/// dates already present in the DB.
///
/// Called from the UI shell at app start and on each navigation into
/// the Task Calendar, so an open-ended series materialized last week
/// gradually grows its 1-year window forward without user action.
pub async fn extend_series_if_needed(repo: &dyn Repository, today: NaiveDate) -> AppResult<()> {
    let series_list = repo.task_series_list().await?;
    if series_list.is_empty() {
        return Ok(());
    }
    let existing_tasks = repo.task_list().await?;
    // Per-series set of dates that are already materialized, so the
    // generator skips them instead of writing duplicates.
    let mut by_series: HashMap<TaskSeriesId, HashSet<NaiveDate>> = HashMap::new();
    for t in &existing_tasks {
        if let Some(sid) = t.series_id {
            by_series.entry(sid).or_default().insert(t.planned_on);
        }
    }
    for s in series_list {
        let existing = by_series.remove(&s.id).unwrap_or_default();
        materialize_occurrences(repo, &s, today, &existing).await?;
    }
    Ok(())
}

/// Insert every occurrence of `series` that falls in
/// `[first, min(end_on, extend_horizon(today))]` and isn't already in
/// `existing`. Pure helper — the dedup set lets `extend_series_if_needed`
/// reuse it without re-querying the DB.
async fn materialize_occurrences(
    repo: &dyn Repository,
    series: &TaskSeries,
    today: NaiveDate,
    existing: &HashSet<NaiveDate>,
) -> AppResult<()> {
    let horizon = extend_horizon(today);
    for date in series.occurrences_until(horizon) {
        if existing.contains(&date) {
            continue;
        }
        let occ = Task::new_in_series(
            series.id,
            series.planting_id,
            series.location_id,
            series.task_type_id,
            series.task_method_id,
            series.implement_id,
            date,
            series.notes.clone(),
        );
        repo.task_create(&occ).await?;
    }
    Ok(())
}

/// Resolve the optional planting reference into the `(planting_id, location_id)`
/// pair stored on the task. An empty string means "free-standing task".
async fn resolve_planting(
    repo: &dyn Repository,
    planting_id_str: &str,
) -> AppResult<(Option<PlantingId>, Option<pomone_domain::LocationId>)> {
    if planting_id_str.trim().is_empty() {
        return Ok((None, None));
    }
    let id: PlantingId = parse_id(planting_id_str)?;
    let planting = repo
        .planting_get(id)
        .await?
        .ok_or_else(|| AppError::NotFound {
            kind: "planting",
            id: planting_id_str.to_owned(),
        })?;
    Ok((Some(planting.id), Some(planting.location_id)))
}

fn empty_to_none(s: &str) -> Option<String> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_owned())
    }
}

fn parse_iso_date_local(s: &str) -> AppResult<NaiveDate> {
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
    use pomone_db::{
        seed_defaults, LocationRepo, SqliteRepository, StrataRepo, TaskRepo, VarietyRepo,
    };
    use rust_decimal_macros::dec;

    async fn fresh_repo_with_data() -> (SqliteRepository, String) {
        let repo = SqliteRepository::in_memory().await.unwrap();
        seed_defaults(&repo).await.unwrap();
        seed_test_data(&repo).await.unwrap();
        let varieties = repo.variety_list().await.unwrap();
        let locations = repo.location_list().await.unwrap();
        let bed = locations.iter().find(|l| l.parent_id.is_some()).unwrap();
        let strata = repo.strata_list().await.unwrap()[0].id;
        let planting = create_annual_planting_from_sowing(
            &repo,
            varieties[0].id,
            bed.id,
            strata,
            NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
            dec!(20),
            100,
            None,
            None,
        )
        .await
        .unwrap();
        (repo, planting.id.to_string())
    }

    #[tokio::test]
    async fn list_task_type_options_returns_all_seeded_types() {
        let repo = SqliteRepository::in_memory().await.unwrap();
        seed_defaults(&repo).await.unwrap();
        let opts = list_task_type_options(&repo).await.unwrap();
        // 8 base categories + the extra "Plantation" type (Transplant category).
        assert_eq!(opts.len(), 9);
        assert!(opts.iter().any(|o| o.name == "Semis"));
        assert!(opts.iter().all(|o| o.color.starts_with('#')));
        assert!(opts.iter().any(|o| o.category == "sow"));
    }

    #[tokio::test]
    async fn create_then_edit_then_delete_roundtrip() {
        let (repo, pid) = fresh_repo_with_data().await;
        let weeding = list_task_type_options(&repo)
            .await
            .unwrap()
            .into_iter()
            .find(|o| o.category == "weeding")
            .unwrap();
        let today = NaiveDate::from_ymd_opt(2026, 5, 24).unwrap();

        let task_id = create_task(
            &repo,
            &pid,
            &weeding.id,
            "2026-05-20",
            "premier passage",
            false,
            today,
        )
        .await
        .unwrap();

        // Form reflects what we just created.
        let form = get_task_for_edit(&repo, &task_id).await.unwrap();
        assert_eq!(form.planting_id, pid);
        assert_eq!(form.task_type_id, weeding.id);
        assert_eq!(form.planned_on, "2026-05-20");
        assert!(!form.completed);
        assert_eq!(form.notes, "premier passage");

        // Edit: change date + mark done.
        update_task(
            &repo,
            &task_id,
            &weeding.id,
            "2026-05-22",
            "passage final",
            true,
            today,
        )
        .await
        .unwrap();

        let after = get_task_for_edit(&repo, &task_id).await.unwrap();
        assert_eq!(after.planned_on, "2026-05-22");
        assert!(after.completed);
        assert_eq!(after.notes, "passage final");

        // Toggle off → completed_on cleared.
        update_task(
            &repo,
            &task_id,
            &weeding.id,
            "2026-05-22",
            "passage final",
            false,
            today,
        )
        .await
        .unwrap();
        let toggled = get_task_for_edit(&repo, &task_id).await.unwrap();
        assert!(!toggled.completed);

        // Delete.
        delete_task(&repo, &task_id).await.unwrap();
        let id: TaskId = parse_id(&task_id).unwrap();
        assert!(repo.task_get(id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn standalone_task_has_no_planting_nor_location() {
        let repo = SqliteRepository::in_memory().await.unwrap();
        seed_defaults(&repo).await.unwrap();
        let other = list_task_type_options(&repo)
            .await
            .unwrap()
            .into_iter()
            .find(|o| o.category == "other")
            .unwrap();
        let today = NaiveDate::from_ymd_opt(2026, 5, 24).unwrap();
        let id = create_task(&repo, "", &other.id, "2026-06-01", "", false, today)
            .await
            .unwrap();
        let task_id: TaskId = parse_id(&id).unwrap();
        let task = repo.task_get(task_id).await.unwrap().unwrap();
        assert!(task.planting_id.is_none());
        assert!(task.location_id.is_none());
        assert!(task.notes.is_none());
    }

    #[tokio::test]
    async fn editing_keeps_previous_completion_date_when_still_done() {
        let (repo, pid) = fresh_repo_with_data().await;
        let harvest = list_task_type_options(&repo)
            .await
            .unwrap()
            .into_iter()
            .find(|o| o.category == "harvest")
            .unwrap();
        let day1 = NaiveDate::from_ymd_opt(2026, 5, 24).unwrap();
        let day2 = NaiveDate::from_ymd_opt(2026, 5, 26).unwrap();

        let id = create_task(&repo, &pid, &harvest.id, "2026-05-20", "", true, day1)
            .await
            .unwrap();
        // Re-save without changing the completed bool: previous date must survive.
        update_task(
            &repo,
            &id,
            &harvest.id,
            "2026-05-20",
            "updated note",
            true,
            day2,
        )
        .await
        .unwrap();
        let tid: TaskId = parse_id(&id).unwrap();
        let task = repo.task_get(tid).await.unwrap().unwrap();
        assert_eq!(task.completed_on, Some(day1));
    }

    #[tokio::test]
    async fn planting_choices_are_sorted_and_carry_compact_labels() {
        let (repo, _) = fresh_repo_with_data().await;
        let choices = list_planting_choices(&repo).await.unwrap();
        assert_eq!(choices.len(), 1);
        assert!(choices[0].label.contains(" — "));
    }

    #[tokio::test]
    async fn delete_unknown_id_returns_not_found() {
        let repo = SqliteRepository::in_memory().await.unwrap();
        let fake = uuid::Uuid::new_v4().to_string();
        let err = delete_task(&repo, &fake).await.unwrap_err();
        assert!(matches!(
            err,
            AppError::Db(pomone_db::DbError::NotFound { .. })
        ));
    }

    #[tokio::test]
    async fn create_recurring_materializes_bounded_set() {
        use pomone_db::TaskRepo;
        let repo = SqliteRepository::in_memory().await.unwrap();
        pomone_db::seed_defaults(&repo).await.unwrap();
        let weeding = list_task_type_options(&repo)
            .await
            .unwrap()
            .into_iter()
            .find(|o| o.category == "weeding")
            .unwrap();
        let today = NaiveDate::from_ymd_opt(2026, 5, 24).unwrap();
        // Weekly, ending 2026-06-30 → starts 2026-05-24, occurrences on
        // 24, 31, 07, 14, 21, 28 = 6 tasks.
        let series_id = create_recurring_task(
            &repo,
            "",
            &weeding.id,
            "2026-05-24",
            "désherbage du dimanche",
            1,
            "weeks",
            Some("2026-06-30"),
            today,
        )
        .await
        .unwrap();
        let tasks = repo.task_list().await.unwrap();
        assert_eq!(tasks.len(), 6);
        let series_uuid: pomone_domain::TaskSeriesId = parse_id(&series_id).unwrap();
        assert!(tasks.iter().all(|t| t.series_id == Some(series_uuid)));
    }

    #[tokio::test]
    async fn extend_series_is_idempotent_and_open_ended() {
        use pomone_db::TaskRepo;
        let repo = SqliteRepository::in_memory().await.unwrap();
        pomone_db::seed_defaults(&repo).await.unwrap();
        let irrig = list_task_type_options(&repo)
            .await
            .unwrap()
            .into_iter()
            .find(|o| o.category == "irrigation")
            .unwrap();
        let today = NaiveDate::from_ymd_opt(2026, 5, 24).unwrap();
        // Open-ended monthly series → materializes 13 occurrences over the
        // 1-year horizon (months 5,6,7,8,9,10,11,12,1,2,3,4,5 — both ends inclusive).
        create_recurring_task(
            &repo,
            "",
            &irrig.id,
            "2026-05-24",
            "",
            1,
            "months",
            None,
            today,
        )
        .await
        .unwrap();
        let first_count = repo.task_list().await.unwrap().len();
        assert_eq!(first_count, 13);

        // Re-running the extender at the same `today` is a no-op.
        extend_series_if_needed(&repo, today).await.unwrap();
        assert_eq!(repo.task_list().await.unwrap().len(), first_count);

        // Pushing today 2 months forward should add 2 new occurrences
        // (months 6 and 7 of 2027, beyond the original horizon).
        let later = NaiveDate::from_ymd_opt(2026, 7, 24).unwrap();
        extend_series_if_needed(&repo, later).await.unwrap();
        let new_count = repo.task_list().await.unwrap().len();
        assert_eq!(new_count, first_count + 2);
    }

    #[test]
    fn extend_horizon_handles_leap_day() {
        // 2024-02-29 has no Feb-29 in 2025 → clamps to 2025-02-28, never `today`.
        let leap = NaiveDate::from_ymd_opt(2024, 2, 29).unwrap();
        assert_eq!(
            extend_horizon(leap),
            NaiveDate::from_ymd_opt(2025, 2, 28).unwrap()
        );
        // A normal day maps to exactly one year later.
        let normal = NaiveDate::from_ymd_opt(2026, 5, 24).unwrap();
        assert_eq!(
            extend_horizon(normal),
            NaiveDate::from_ymd_opt(2027, 5, 24).unwrap()
        );
    }

    #[tokio::test]
    async fn open_ended_series_created_on_leap_day_fills_a_full_year() {
        use pomone_db::TaskRepo;
        let repo = SqliteRepository::in_memory().await.unwrap();
        pomone_db::seed_defaults(&repo).await.unwrap();
        let irrig = list_task_type_options(&repo)
            .await
            .unwrap()
            .into_iter()
            .find(|o| o.category == "irrigation")
            .unwrap();
        // Open-ended weekly series created ON a leap day. With the old
        // horizon the window collapsed to `today` → a single occurrence;
        // now it spans the full ~year (issue #60).
        let leap = NaiveDate::from_ymd_opt(2024, 2, 29).unwrap();
        create_recurring_task(
            &repo,
            "",
            &irrig.id,
            "2024-02-29",
            "",
            1,
            "weeks",
            None,
            leap,
        )
        .await
        .unwrap();
        let count = repo.task_list().await.unwrap().len();
        assert!(
            count >= 50,
            "expected a full year of weekly occurrences, got {count}"
        );
    }
}
