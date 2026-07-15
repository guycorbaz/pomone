//! Presentation-layer helper for the **ITK editor** on the crop detail
//! (Cultures master-detail, Epic 2 story 2.5).
//!
//! A crop's itinéraire technique is an ordered list of activities (a task type
//! at a signed day-offset from establishment, optionally pinned to a method and
//! implement). This module lists them decorated for the grid, and offers the
//! edit operations: add (appends), update, move up/down (reorder) and delete.
//! The crop's [`ItkTemplate`] is created lazily on the first activity add, so a
//! crop with no ITK stays ITK-less (and keeps the variety-profile autogen).
//!
//! Ordering is kept correct **by construction**: adds append at the tail and
//! reorder swaps adjacent positions, so positions stay a contiguous `0..n`
//! sequence with no duplicates — no free-form position editing to validate.

use crate::error::{AppError, AppResult};
use crate::i18n::I18n;
use crate::plantings_view::parse_id;
use pomone_db::Repository;
use pomone_domain::{
    CropId, ItkActivity, ItkActivityId, ItkTemplate, TaskImplementId, TaskMethodId, TaskTypeId,
};
use std::collections::HashMap;

/// A named option (task type / method / implement) for the editor's pickers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItkOption {
    pub id: String,
    pub name: String,
}

/// One ITK activity, decorated for the editor. `*_name` are resolved labels;
/// `method_id`/`implement_id` are empty strings when unset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItkActivityRow {
    pub id: String,
    /// Compact signed offset, e.g. `"J-10"`, `"J+20"`, `"J"` (on the day).
    pub offset_label: String,
    /// The raw signed offset as a string (for the edit form).
    pub offset_days: String,
    pub task_type_id: String,
    pub task_type_name: String,
    pub method_id: String,
    pub method_name: String,
    pub implement_id: String,
    pub implement_name: String,
    pub label: String,
    pub notes: String,
    /// 0-based order within the template (drives up/down affordances).
    pub position: u32,
}

/// Fields for adding or updating an activity from the editor. `id` empty = add.
#[derive(Debug, Clone, Default)]
pub struct ItkActivityInput {
    pub id: String,
    pub crop_id: String,
    pub task_type_id: String,
    /// Signed integer string (`-10`, `20`, `0`).
    pub offset_days: String,
    /// Empty = no method / implement.
    pub method_id: String,
    pub implement_id: String,
    pub label: String,
    pub notes: String,
}

/// Format a signed offset as the editor's `J±N` label.
#[must_use]
pub fn format_offset(days: i32) -> String {
    match days.cmp(&0) {
        std::cmp::Ordering::Equal => "J".to_owned(),
        std::cmp::Ordering::Greater => format!("J+{days}"),
        // The minus sign is already part of the number.
        std::cmp::Ordering::Less => format!("J{days}"),
    }
}

/// List the crop's ITK activities (ordered), decorated with resolved names.
/// Empty when the crop has no template yet.
pub async fn list_itk_activities(
    repo: &dyn Repository,
    crop_id_str: &str,
    _i18n: &I18n,
) -> AppResult<Vec<ItkActivityRow>> {
    let crop_id: CropId = parse_id(crop_id_str)?;
    let Some(template) = repo.itk_template_get_for_crop(crop_id).await? else {
        return Ok(Vec::new());
    };
    let activities = repo.itk_activity_list_for_template(template.id).await?;

    let types: HashMap<_, _> = repo
        .task_type_list()
        .await?
        .into_iter()
        .map(|t| (t.id, t.name))
        .collect();
    let methods: HashMap<_, _> = repo
        .task_method_list()
        .await?
        .into_iter()
        .map(|m| (m.id, m.name))
        .collect();
    let implements: HashMap<_, _> = repo
        .task_implement_list()
        .await?
        .into_iter()
        .map(|i| (i.id, i.name))
        .collect();

    Ok(activities
        .into_iter()
        .map(|a| ItkActivityRow {
            id: a.id.to_string(),
            offset_label: format_offset(a.offset_days),
            offset_days: a.offset_days.to_string(),
            task_type_id: a.task_type_id.to_string(),
            task_type_name: types.get(&a.task_type_id).cloned().unwrap_or_default(),
            method_id: a.method_id.map(|m| m.to_string()).unwrap_or_default(),
            method_name: a
                .method_id
                .and_then(|m| methods.get(&m).cloned())
                .unwrap_or_default(),
            implement_id: a.implement_id.map(|i| i.to_string()).unwrap_or_default(),
            implement_name: a
                .implement_id
                .and_then(|i| implements.get(&i).cloned())
                .unwrap_or_default(),
            label: a.label.clone().unwrap_or_default(),
            notes: a.notes.clone().unwrap_or_default(),
            position: a.position,
        })
        .collect())
}

/// Method options for the picker (empty ⇒ the UI hides the method picker).
pub async fn itk_method_options(repo: &dyn Repository) -> AppResult<Vec<ItkOption>> {
    Ok(repo
        .task_method_list()
        .await?
        .into_iter()
        .map(|m| ItkOption {
            id: m.id.to_string(),
            name: m.name,
        })
        .collect())
}

/// Implement options for the picker (empty ⇒ the UI hides the implement picker).
pub async fn itk_implement_options(repo: &dyn Repository) -> AppResult<Vec<ItkOption>> {
    Ok(repo
        .task_implement_list()
        .await?
        .into_iter()
        .map(|i| ItkOption {
            id: i.id.to_string(),
            name: i.name,
        })
        .collect())
}

/// Add or update an activity. On add the crop's template is created lazily and
/// the activity is appended (position = current count). On update the position
/// is preserved. Returns the activity id.
pub async fn save_itk_activity(
    repo: &dyn Repository,
    input: &ItkActivityInput,
) -> AppResult<String> {
    let task_type_id: TaskTypeId = parse_id(&input.task_type_id)?;
    let offset_days: i32 =
        input.offset_days.trim().parse().map_err(|_| {
            AppError::Inconsistent(format!("invalid offset: '{}'", input.offset_days))
        })?;
    let method_id = parse_optional_id::<TaskMethodId>(&input.method_id)?;
    let implement_id = parse_optional_id::<TaskImplementId>(&input.implement_id)?;
    let label = empty_to_none(&input.label);
    let notes = empty_to_none(&input.notes);

    if input.id.trim().is_empty() {
        let crop_id: CropId = parse_id(&input.crop_id)?;
        let template = get_or_create_template(repo, crop_id).await?;
        let position = u32::try_from(
            repo.itk_activity_list_for_template(template.id)
                .await?
                .len(),
        )
        .unwrap_or(u32::MAX);
        let activity = ItkActivity::new(
            template.id,
            task_type_id,
            offset_days,
            method_id,
            implement_id,
            label,
            position,
            notes,
        );
        let id = activity.id.to_string();
        repo.itk_activity_create(&activity).await?;
        Ok(id)
    } else {
        let id: ItkActivityId = parse_id(&input.id)?;
        let existing = find_activity(repo, id).await?;
        let updated = existing.with_updates(
            task_type_id,
            offset_days,
            method_id,
            implement_id,
            label,
            // Position is not editable via the form — reorder does that.
            existing_position(repo, id).await?,
            notes,
        );
        repo.itk_activity_update(&updated).await?;
        Ok(id.to_string())
    }
}

/// Move an activity one slot up (`up = true`) or down within its template,
/// swapping positions with the adjacent activity. A no-op at the ends.
pub async fn move_itk_activity(
    repo: &dyn Repository,
    activity_id_str: &str,
    up: bool,
) -> AppResult<()> {
    let id: ItkActivityId = parse_id(activity_id_str)?;
    let activity = find_activity(repo, id).await?;
    let mut siblings = repo
        .itk_activity_list_for_template(activity.template_id)
        .await?;
    // siblings come back position-ordered.
    let Some(idx) = siblings.iter().position(|a| a.id == id) else {
        return Ok(());
    };
    let other = if up {
        if idx == 0 {
            return Ok(());
        }
        idx - 1
    } else {
        if idx + 1 >= siblings.len() {
            return Ok(());
        }
        idx + 1
    };
    // Swap the two positions and persist both.
    let a_pos = siblings[idx].position;
    let b_pos = siblings[other].position;
    siblings[idx].position = b_pos;
    siblings[other].position = a_pos;
    repo.itk_activity_update(&siblings[idx]).await?;
    repo.itk_activity_update(&siblings[other]).await?;
    Ok(())
}

/// Delete an activity. The count of generated tasks referencing it is reported
/// by [`count_generated_tasks_for_activity`] (0 until generation wires the
/// back-link in story 2.6); the UI shows a data-danger warning with that count.
pub async fn delete_itk_activity(repo: &dyn Repository, activity_id_str: &str) -> AppResult<()> {
    let id: ItkActivityId = parse_id(activity_id_str)?;
    repo.itk_activity_delete(id).await?;
    Ok(())
}

/// How many generated tasks currently reference this activity. Always 0 until
/// story 2.6 adds the task ← itk_activity back-link; kept as the seam the
/// editor's data-danger delete warning reads.
#[allow(clippy::unused_async)]
pub async fn count_generated_tasks_for_activity(
    _repo: &dyn Repository,
    _activity_id_str: &str,
) -> AppResult<usize> {
    Ok(0)
}

// --- internals ---

async fn get_or_create_template(repo: &dyn Repository, crop_id: CropId) -> AppResult<ItkTemplate> {
    if let Some(t) = repo.itk_template_get_for_crop(crop_id).await? {
        return Ok(t);
    }
    let template = ItkTemplate::new(crop_id);
    repo.itk_template_create(&template).await?;
    Ok(template)
}

async fn find_activity(repo: &dyn Repository, id: ItkActivityId) -> AppResult<ItkActivity> {
    // Activities are only reachable per-template; walk the crop templates. A
    // direct getter would be cheaper, but ITKs are tiny.
    for template in itk_templates(repo).await? {
        for a in repo.itk_activity_list_for_template(template.id).await? {
            if a.id == id {
                return Ok(a);
            }
        }
    }
    Err(AppError::NotFound {
        kind: "itk_activity",
        id: id.to_string(),
    })
}

async fn existing_position(repo: &dyn Repository, id: ItkActivityId) -> AppResult<u32> {
    Ok(find_activity(repo, id).await?.position)
}

async fn itk_templates(repo: &dyn Repository) -> AppResult<Vec<ItkTemplate>> {
    let mut out = Vec::new();
    for crop in repo.crop_list().await? {
        if let Some(t) = repo.itk_template_get_for_crop(crop.id).await? {
            out.push(t);
        }
    }
    Ok(out)
}

fn parse_optional_id<T: From<uuid::Uuid>>(s: &str) -> AppResult<Option<T>> {
    let t = s.trim();
    if t.is_empty() {
        Ok(None)
    } else {
        Ok(Some(parse_id(t)?))
    }
}

fn empty_to_none(s: &str) -> Option<String> {
    let t = s.trim();
    (!t.is_empty()).then(|| t.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pomone_db::{seed_defaults, SqliteRepository};

    fn i18n() -> I18n {
        I18n::new(crate::i18n::Lang::Fr).unwrap()
    }

    async fn repo_with_crop() -> (SqliteRepository, String, String) {
        use pomone_db::{CropRepo, FamilyRepo, TaskTypeRepo};
        use pomone_domain::{Crop, Family, Lifespan, PruningSeason, TaskCategory, TaskType};
        let repo = SqliteRepository::in_memory().await.unwrap();
        seed_defaults(&repo).await.unwrap();
        let f = Family::new("Asteraceae", None, None).unwrap();
        repo.family_create(&f).await.unwrap();
        let crop = Crop::new(f.id, "Laitue", None, Lifespan::Annual, PruningSeason::None).unwrap();
        repo.crop_create(&crop).await.unwrap();
        let tt = TaskType::new("Préparation", TaskCategory::Tillage, "#8a8a8a").unwrap();
        repo.task_type_create(&tt).await.unwrap();
        (repo, crop.id.to_string(), tt.id.to_string())
    }

    fn input(crop: &str, tt: &str, offset: &str) -> ItkActivityInput {
        ItkActivityInput {
            crop_id: crop.to_owned(),
            task_type_id: tt.to_owned(),
            offset_days: offset.to_owned(),
            ..Default::default()
        }
    }

    #[test]
    fn offset_formats_signed() {
        assert_eq!(format_offset(-10), "J-10");
        assert_eq!(format_offset(20), "J+20");
        assert_eq!(format_offset(0), "J");
    }

    #[tokio::test]
    async fn add_lazily_creates_template_and_appends_in_order() {
        let (repo, crop, tt) = repo_with_crop().await;
        // No template initially.
        assert!(list_itk_activities(&repo, &crop, &i18n())
            .await
            .unwrap()
            .is_empty());

        save_itk_activity(&repo, &input(&crop, &tt, "-10"))
            .await
            .unwrap();
        save_itk_activity(&repo, &input(&crop, &tt, "20"))
            .await
            .unwrap();

        let rows = list_itk_activities(&repo, &crop, &i18n()).await.unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].offset_label, "J-10");
        assert_eq!(rows[0].position, 0);
        assert_eq!(rows[1].offset_label, "J+20");
        assert_eq!(rows[1].position, 1);
        assert_eq!(rows[0].task_type_name, "Préparation");
    }

    #[tokio::test]
    async fn move_swaps_adjacent_positions() {
        let (repo, crop, tt) = repo_with_crop().await;
        save_itk_activity(&repo, &input(&crop, &tt, "-10"))
            .await
            .unwrap();
        save_itk_activity(&repo, &input(&crop, &tt, "20"))
            .await
            .unwrap();
        let rows = list_itk_activities(&repo, &crop, &i18n()).await.unwrap();
        let second = rows[1].id.clone();

        move_itk_activity(&repo, &second, true).await.unwrap();
        let rows = list_itk_activities(&repo, &crop, &i18n()).await.unwrap();
        assert_eq!(rows[0].offset_label, "J+20", "moved up to the top");
        assert_eq!(rows[1].offset_label, "J-10");

        // Moving the top item up is a no-op.
        move_itk_activity(&repo, &rows[0].id, true).await.unwrap();
        let again = list_itk_activities(&repo, &crop, &i18n()).await.unwrap();
        assert_eq!(again[0].offset_label, "J+20");
    }

    #[tokio::test]
    async fn update_changes_fields_keeps_position_and_delete_removes() {
        let (repo, crop, tt) = repo_with_crop().await;
        save_itk_activity(&repo, &input(&crop, &tt, "-10"))
            .await
            .unwrap();
        let id = list_itk_activities(&repo, &crop, &i18n()).await.unwrap()[0]
            .id
            .clone();

        let mut edit = input(&crop, &tt, "5");
        edit.id = id.clone();
        edit.label = "  préparation planche ".to_owned();
        save_itk_activity(&repo, &edit).await.unwrap();
        let rows = list_itk_activities(&repo, &crop, &i18n()).await.unwrap();
        assert_eq!(rows[0].offset_label, "J+5");
        assert_eq!(rows[0].label, "préparation planche");
        assert_eq!(rows[0].position, 0);

        assert_eq!(
            count_generated_tasks_for_activity(&repo, &id)
                .await
                .unwrap(),
            0
        );
        delete_itk_activity(&repo, &id).await.unwrap();
        assert!(list_itk_activities(&repo, &crop, &i18n())
            .await
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn method_and_implement_options_reflect_catalog() {
        use pomone_db::{TaskImplementRepo, TaskMethodRepo};
        use pomone_domain::{TaskImplement, TaskMethod};
        let (repo, _crop, _tt) = repo_with_crop().await;
        // Seed defaults may already include some; assert our adds show up.
        repo.task_method_create(&TaskMethod::new("Manuel", None).unwrap())
            .await
            .unwrap();
        repo.task_implement_create(&TaskImplement::new("Grelinette", None).unwrap())
            .await
            .unwrap();
        assert!(itk_method_options(&repo)
            .await
            .unwrap()
            .iter()
            .any(|o| o.name == "Manuel"));
        assert!(itk_implement_options(&repo)
            .await
            .unwrap()
            .iter()
            .any(|o| o.name == "Grelinette"));
    }
}
