//! Itinéraires techniques (ITK) — how a crop is grown, as ordered activities.
//!
//! An [`ItkTemplate`] belongs to a **crop** (one per crop) and carries an
//! ordered list of [`ItkActivity`] rows: each is a task type at a **signed
//! day-offset from establishment** (`J-10` = 10 days before, `J+20` = 20 days
//! after), optionally pinned to a method and implement (the *dormant*
//! `task_method`/`task_implement` FKs, revived here — no parallel columns) and
//! annotated with a label and notes.
//!
//! Story 2.2 is persistence only: generation from an ITK arrives with story 2.6,
//! and a crop **without** an ITK keeps the shipped variety-profile autogen
//! (fallback). No doses in R1 (planned treatments are R2).

use crate::error::{DomainError, DomainResult};
use crate::ids::{CropId, ItkActivityId, ItkTemplateId, TaskImplementId, TaskMethodId, TaskTypeId};
use crate::validation::normalize_optional;
use serde::{Deserialize, Serialize};

/// A crop's ITK — the per-crop container its activities hang off. One per crop
/// (enforced by a unique index on `crop_id`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItkTemplate {
    pub id: ItkTemplateId,
    pub crop_id: CropId,
}

impl ItkTemplate {
    /// Build a fresh template for `crop_id`.
    #[must_use]
    pub fn new(crop_id: CropId) -> Self {
        Self {
            id: ItkTemplateId::new(),
            crop_id,
        }
    }
}

/// One ordered activity of an ITK template.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItkActivity {
    pub id: ItkActivityId,
    pub template_id: ItkTemplateId,
    /// The kind of operation (existing FK). Required.
    pub task_type_id: TaskTypeId,
    /// Signed day-offset from establishment: negative = before (`J-10`),
    /// positive = after (`J+20`), zero = on the day.
    pub offset_days: i32,
    /// Optional method (dormant FK revived) — manual vs mechanized, etc.
    pub method_id: Option<TaskMethodId>,
    /// Optional implement / tool (dormant FK revived).
    pub implement_id: Option<TaskImplementId>,
    /// Optional human label (e.g. "préparation planche"). Blank collapses to
    /// `None`.
    pub label: Option<String>,
    /// Explicit ordering within the template (the editor reorders on this;
    /// two activities may share an `offset_days`). Stable, 0-based.
    pub position: u32,
    /// Optional free-form notes.
    pub notes: Option<String>,
}

impl ItkActivity {
    /// Build a fresh activity. Nothing to reject beyond the type system:
    /// `offset_days` is intentionally free (signed), `position` is any `u32`,
    /// and the label/notes normalize blanks to `None`. Callers pass a valid
    /// `task_type_id`; the FK enforces referential integrity at persistence.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        template_id: ItkTemplateId,
        task_type_id: TaskTypeId,
        offset_days: i32,
        method_id: Option<TaskMethodId>,
        implement_id: Option<TaskImplementId>,
        label: Option<String>,
        position: u32,
        notes: Option<String>,
    ) -> Self {
        Self {
            id: ItkActivityId::new(),
            template_id,
            task_type_id,
            offset_days,
            method_id,
            implement_id,
            label: normalize_optional(label),
            position,
            notes: normalize_optional(notes),
        }
    }

    /// Rebuild an activity keeping its identity — the edit path (story 2.5).
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn with_updates(
        self,
        task_type_id: TaskTypeId,
        offset_days: i32,
        method_id: Option<TaskMethodId>,
        implement_id: Option<TaskImplementId>,
        label: Option<String>,
        position: u32,
        notes: Option<String>,
    ) -> Self {
        Self {
            id: self.id,
            template_id: self.template_id,
            task_type_id,
            offset_days,
            method_id,
            implement_id,
            label: normalize_optional(label),
            position,
            notes: normalize_optional(notes),
        }
    }
}

/// Validate that a set of activities forms a coherent ITK template.
///
/// The only cross-row rule today is that `position`s are unique within a
/// template — two activities must not claim the same slot, or reordering and
/// deterministic rendering break. Returns [`DomainError::DuplicatePosition`]
/// on the first collision.
pub fn check_positions_unique(activities: &[ItkActivity]) -> DomainResult<()> {
    let mut seen = std::collections::BTreeSet::new();
    for a in activities {
        if !seen.insert(a.position) {
            return Err(DomainError::DuplicatePosition(a.position));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activity_normalizes_label_and_notes() {
        let a = ItkActivity::new(
            ItkTemplateId::new(),
            TaskTypeId::new(),
            -10,
            None,
            None,
            Some("  préparation planche ".to_owned()),
            0,
            Some("   ".to_owned()),
        );
        assert_eq!(a.label.as_deref(), Some("préparation planche"));
        assert_eq!(a.notes, None);
        assert_eq!(a.offset_days, -10);
    }

    #[test]
    fn offset_may_be_negative_zero_or_positive() {
        for off in [-10, 0, 20] {
            let a = ItkActivity::new(
                ItkTemplateId::new(),
                TaskTypeId::new(),
                off,
                None,
                None,
                None,
                0,
                None,
            );
            assert_eq!(a.offset_days, off);
        }
    }

    #[test]
    fn with_updates_keeps_identity_and_template() {
        let a = ItkActivity::new(
            ItkTemplateId::new(),
            TaskTypeId::new(),
            5,
            None,
            None,
            None,
            0,
            None,
        );
        let id = a.id;
        let tpl = a.template_id;
        let new_type = TaskTypeId::new();
        let updated = a.with_updates(new_type, 12, None, None, Some("désherbage".into()), 1, None);
        assert_eq!(updated.id, id);
        assert_eq!(updated.template_id, tpl);
        assert_eq!(updated.task_type_id, new_type);
        assert_eq!(updated.offset_days, 12);
        assert_eq!(updated.position, 1);
    }

    #[test]
    fn positions_unique_accepts_distinct_rejects_duplicate() {
        let tpl = ItkTemplateId::new();
        let mk = |pos| ItkActivity::new(tpl, TaskTypeId::new(), 0, None, None, None, pos, None);
        assert!(check_positions_unique(&[mk(0), mk(1), mk(2)]).is_ok());
        assert!(matches!(
            check_positions_unique(&[mk(0), mk(1), mk(1)]),
            Err(DomainError::DuplicatePosition(1))
        ));
    }

    #[test]
    fn template_new_targets_the_crop() {
        let crop = CropId::new();
        let t = ItkTemplate::new(crop);
        assert_eq!(t.crop_id, crop);
    }
}
