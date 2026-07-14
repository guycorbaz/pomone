//! Presentation-layer helpers for managing the task-types catalog.
//!
//! Pomone seeds eight default task types (one per [`TaskCategory`]) on a
//! fresh database; this module is the user-facing path for adding custom
//! types, renaming the defaults, recoloring them, or deleting unused ones.
//!
//! The category of a type is structural — it drives `task_autogen`'s
//! lookup-by-category logic — so this module exposes it as a stable enum
//! string but does NOT let `update_task_type` change it. Renaming "Semis"
//! to "Semis serre" preserves the underlying `Sow` category and keeps
//! the auto-generator working.

use crate::error::{AppError, AppResult};
use crate::plantings_view::parse_id;
use crate::tasks_view::{category_from_str, category_str};
use pomone_db::Repository;
use pomone_domain::{TaskCategory, TaskType, TaskTypeId};
use std::collections::HashSet;

/// One row of the Task Types admin screen — adds an `in_use` flag so the
/// UI can grey out the Delete button for types that have tasks attached.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskTypeAdminRow {
    pub id: String,
    pub name: String,
    /// Stable category string (`"sow"`, `"harvest"`, …). UI side resolves
    /// it to a Fluent label.
    pub category: String,
    pub color: String,
    pub in_use: bool,
}

/// Catalog of the eight `TaskCategory` enum variants, returned as
/// `(stable_key, fluent_label_key)` tuples for the create form's
/// category ComboBox. Order matches the domain's enum declaration so the
/// UI is predictable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskCategoryOption {
    /// `"sow"`, `"transplant"`, … — what we store on the form & send back.
    pub key: String,
    /// `"category-sow"`, `"category-transplant"`, … — Fluent message ID
    /// the UI resolves via `i18n.t()`.
    pub label_key: String,
}

/// Pre-filled state for editing an existing type. `category` is read-only
/// in the edit form (structural), so the UI shows it but disables the
/// ComboBox.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskTypeEditForm {
    pub id: String,
    pub name: String,
    pub category: String,
    pub color: String,
}

/// All categories in enum-declaration order. The UI uses this to populate
/// the create form's ComboBox.
#[must_use]
pub fn list_task_category_options() -> Vec<TaskCategoryOption> {
    const ORDER: &[TaskCategory] = &[
        TaskCategory::Sow,
        TaskCategory::Transplant,
        TaskCategory::Harvest,
        TaskCategory::Weeding,
        TaskCategory::Irrigation,
        TaskCategory::Treatment,
        TaskCategory::Tillage,
        TaskCategory::Other,
    ];
    ORDER
        .iter()
        .copied()
        .map(|c| TaskCategoryOption {
            key: category_str(c).to_owned(),
            label_key: format!("category-{}", category_str(c)),
        })
        .collect()
}

/// List every task type with an `in_use` flag computed from the full task
/// list. Two DB reads (types + tasks) — fine at Pomone's scale.
pub async fn list_task_types_admin(repo: &dyn Repository) -> AppResult<Vec<TaskTypeAdminRow>> {
    let types = repo.task_type_list().await?;
    let tasks = repo.task_list().await?;
    let used: HashSet<TaskTypeId> = tasks.iter().map(|t| t.task_type_id).collect();
    Ok(types
        .into_iter()
        .map(|t| TaskTypeAdminRow {
            id: t.id.to_string(),
            in_use: used.contains(&t.id),
            name: t.name,
            category: category_str(t.category).to_owned(),
            color: t.color,
        })
        .collect())
}

/// Load one type into edit-form shape. `NotFound` if the type was deleted
/// in another window.
pub async fn get_task_type_for_edit(
    repo: &dyn Repository,
    id_str: &str,
) -> AppResult<TaskTypeEditForm> {
    let id: TaskTypeId = parse_id(id_str)?;
    let tt = repo
        .task_type_get(id)
        .await?
        .ok_or_else(|| AppError::NotFound {
            kind: "task_type",
            id: id_str.to_owned(),
        })?;
    Ok(TaskTypeEditForm {
        id: tt.id.to_string(),
        name: tt.name,
        category: category_str(tt.category).to_owned(),
        color: tt.color,
    })
}

/// Create a new task type. Validates name (non-empty after trim) and color
/// (`#RGB` / `#RRGGBB`) via the domain's `TaskType::new`. The category
/// string must be one of the eight known keys.
pub async fn create_task_type(
    repo: &dyn Repository,
    name: &str,
    category_key: &str,
    color: &str,
) -> AppResult<String> {
    let category = category_from_str(category_key)
        .ok_or_else(|| AppError::Inconsistent(format!("unknown category '{category_key}'")))?;
    let tt = TaskType::new(name, category, color)?;
    repo.task_type_create(&tt).await?;
    Ok(tt.id.to_string())
}

/// Rename and/or recolor an existing type. The category is preserved
/// (it's structural — see module docs).
pub async fn update_task_type(
    repo: &dyn Repository,
    id_str: &str,
    name: &str,
    color: &str,
) -> AppResult<()> {
    let id: TaskTypeId = parse_id(id_str)?;
    let existing = repo
        .task_type_get(id)
        .await?
        .ok_or_else(|| AppError::NotFound {
            kind: "task_type",
            id: id_str.to_owned(),
        })?;
    let renamed = existing.rename(name, color)?;
    repo.task_type_update(&renamed).await?;
    Ok(())
}

/// Delete a task type. The schema has `ON DELETE RESTRICT` on
/// `task.task_type_id`, so deleting a type currently used by any task
/// fails at the DB layer; we surface that as a friendlier `InUse` error
/// the UI can map to a clear status message.
pub async fn delete_task_type(repo: &dyn Repository, id_str: &str) -> AppResult<()> {
    let id: TaskTypeId = parse_id(id_str)?;
    match repo.task_type_delete(id).await {
        Ok(()) => Ok(()),
        // Only a foreign-key violation means "still referenced by tasks"; any
        // other DB error is a real fault and must surface as such rather than
        // being mislabeled as "in use".
        Err(e) if e.is_foreign_key_violation() => Err(AppError::Inconsistent(
            "task_type_in_use".to_owned(), // UI re-keys via Fluent
        )),
        Err(other) => Err(AppError::Db(other)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tasks_view::create_task;
    use chrono::NaiveDate;
    use pomone_db::{seed_defaults, SqliteRepository};

    async fn fresh() -> SqliteRepository {
        let repo = SqliteRepository::in_memory().await.unwrap();
        seed_defaults(&repo).await.unwrap();
        repo
    }

    #[tokio::test]
    async fn list_admin_marks_used_types() {
        let repo = fresh().await;
        let rows_before = list_task_types_admin(&repo).await.unwrap();
        // 8 base categories + the "Plantation" type (Transplant) added for
        // establishment methods.
        assert_eq!(rows_before.len(), 9);
        assert!(rows_before.iter().all(|r| !r.in_use));

        let harvest_id = rows_before
            .iter()
            .find(|r| r.category == "harvest")
            .unwrap()
            .id
            .clone();
        let today = NaiveDate::from_ymd_opt(2026, 5, 24).unwrap();
        create_task(
            &repo,
            "",
            &harvest_id,
            "2026-06-01",
            "",
            false,
            today,
            today.and_hms_opt(12, 0, 0).unwrap(),
        )
        .await
        .unwrap();

        let rows_after = list_task_types_admin(&repo).await.unwrap();
        let harvest_row = rows_after.iter().find(|r| r.category == "harvest").unwrap();
        assert!(harvest_row.in_use);
        // The other 7 stay unused.
        assert_eq!(rows_after.iter().filter(|r| r.in_use).count(), 1);
    }

    #[tokio::test]
    async fn category_options_cover_all_eight_in_declaration_order() {
        let opts = list_task_category_options();
        let keys: Vec<_> = opts.iter().map(|o| o.key.as_str()).collect();
        assert_eq!(
            keys,
            vec![
                "sow",
                "transplant",
                "harvest",
                "weeding",
                "irrigation",
                "treatment",
                "tillage",
                "other"
            ]
        );
        assert!(opts.iter().all(|o| o.label_key.starts_with("category-")));
    }

    #[tokio::test]
    async fn create_then_edit_then_delete_roundtrip() {
        let repo = fresh().await;
        let id = create_task_type(&repo, "Paillage", "other", "#6B5D4D")
            .await
            .unwrap();
        let form = get_task_type_for_edit(&repo, &id).await.unwrap();
        assert_eq!(form.name, "Paillage");
        assert_eq!(form.category, "other");

        update_task_type(&repo, &id, "Paillage organique", "#7A6A5C")
            .await
            .unwrap();
        let after = get_task_type_for_edit(&repo, &id).await.unwrap();
        assert_eq!(after.name, "Paillage organique");
        assert_eq!(after.color, "#7A6A5C");
        assert_eq!(after.category, "other"); // unchanged

        delete_task_type(&repo, &id).await.unwrap();
        assert!(get_task_type_for_edit(&repo, &id).await.is_err());
    }

    #[tokio::test]
    async fn delete_used_type_returns_inconsistent_in_use_sentinel() {
        let repo = fresh().await;
        let rows = list_task_types_admin(&repo).await.unwrap();
        let sow_id = rows
            .iter()
            .find(|r| r.category == "sow")
            .unwrap()
            .id
            .clone();
        let today = NaiveDate::from_ymd_opt(2026, 5, 24).unwrap();
        create_task(
            &repo,
            "",
            &sow_id,
            "2026-04-01",
            "",
            false,
            today,
            today.and_hms_opt(12, 0, 0).unwrap(),
        )
        .await
        .unwrap();

        let err = delete_task_type(&repo, &sow_id).await.unwrap_err();
        match err {
            AppError::Inconsistent(s) => assert_eq!(s, "task_type_in_use"),
            other => panic!("expected InUse sentinel, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn create_rejects_unknown_category() {
        let repo = fresh().await;
        let err = create_task_type(&repo, "Foo", "not-a-category", "#3C6E47")
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::Inconsistent(_)));
    }

    #[tokio::test]
    async fn create_rejects_invalid_color() {
        let repo = fresh().await;
        let err = create_task_type(&repo, "Foo", "other", "green")
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::Domain(_)));
    }

    #[tokio::test]
    async fn create_rejects_blank_name() {
        let repo = fresh().await;
        let err = create_task_type(&repo, "  ", "other", "#3C6E47")
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::Domain(_)));
    }
}
