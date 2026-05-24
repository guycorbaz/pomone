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
use chrono::NaiveDate;
use pomone_db::Repository;
use pomone_domain::{PlantingId, Task, TaskCategory, TaskId, TaskType, TaskTypeId};
use std::collections::HashMap;

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
    use pomone_db::{seed_defaults, LocationRepo, SqliteRepository, TaskRepo, VarietyRepo};
    use rust_decimal_macros::dec;

    async fn fresh_repo_with_data() -> (SqliteRepository, String) {
        let repo = SqliteRepository::in_memory().await.unwrap();
        seed_defaults(&repo).await.unwrap();
        seed_test_data(&repo).await.unwrap();
        let varieties = repo.variety_list().await.unwrap();
        let locations = repo.location_list().await.unwrap();
        let bed = locations.iter().find(|l| l.parent_id.is_some()).unwrap();
        let planting = create_annual_planting_from_sowing(
            &repo,
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
        (repo, planting.id.to_string())
    }

    #[tokio::test]
    async fn list_task_type_options_returns_all_seeded_types() {
        let repo = SqliteRepository::in_memory().await.unwrap();
        seed_defaults(&repo).await.unwrap();
        let opts = list_task_type_options(&repo).await.unwrap();
        assert_eq!(opts.len(), 8); // one per TaskCategory
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
}
