//! Task types — the taxonomy of operations users can plan.
//!
//! Each `Task` references a `TaskType` (sowing, transplanting, weeding,
//! harvesting, treatment…). Types come pre-seeded with sensible defaults
//! and can be added or renamed by the user, but every type also carries a
//! stable `TaskCategory` so the auto-generator (PR E) and any reporting
//! code can recognize "this is a sow" regardless of what the user named it.

use serde::{Deserialize, Serialize};

use crate::ids::TaskTypeId;
use crate::validation::{normalize_optional, require_name};
use crate::{DomainError, DomainResult};

/// Stable identity of what a task type represents, independent of the
/// user-visible name. Lets PR E's auto-generator pick the right type to
/// instantiate from a planting's dates (e.g. category `Sow` → type "Semis").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskCategory {
    /// Direct sowing in the field or a sow under cover.
    Sow,
    /// Moving young plants from the nursery / greenhouse to the field.
    Transplant,
    /// Harvest pass.
    Harvest,
    /// Weeding (manual or mechanized).
    Weeding,
    /// Irrigation / watering.
    Irrigation,
    /// Plant-protection treatment (organic or otherwise).
    Treatment,
    /// Tillage / soil preparation (plowing, harrowing, false sowing…).
    Tillage,
    /// Generic placeholder for user-defined task types that don't fit any
    /// of the above. The auto-generator never produces these.
    Other,
}

/// A task type entry, user-editable but category-tagged.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskType {
    pub id: TaskTypeId,
    pub name: String,
    pub category: TaskCategory,
    /// Hex color (e.g. "#3C6E47"), shown on the Gantt task overlay and the
    /// task calendar. Stored as a string so the DB layer stays simple.
    pub color: String,
}

impl TaskType {
    /// Construct a fresh `TaskType`. Trims the name, rejects blank names,
    /// and validates the color as `#RGB` or `#RRGGBB`.
    pub fn new(
        name: impl Into<String>,
        category: TaskCategory,
        color: impl Into<String>,
    ) -> DomainResult<Self> {
        Ok(Self {
            id: TaskTypeId::new(),
            name: require_name(name)?,
            category,
            color: require_hex_color(color.into())?,
        })
    }

    /// Update the name and color (the category is structural; rename it via
    /// recreating if you really need to migrate).
    pub fn rename(self, name: impl Into<String>, color: impl Into<String>) -> DomainResult<Self> {
        Ok(Self {
            name: require_name(name)?,
            color: require_hex_color(color.into())?,
            ..self
        })
    }
}

fn require_hex_color(raw: String) -> DomainResult<String> {
    let trimmed = raw.trim();
    let ok = trimmed.starts_with('#')
        && matches!(trimmed.len(), 4 | 7)
        && trimmed[1..].chars().all(|c| c.is_ascii_hexdigit());
    if ok {
        Ok(trimmed.to_owned())
    } else {
        Err(DomainError::InvalidHexColor(raw))
    }
}

/// A method used to carry out a task (manual vs mechanized, broadcast vs
/// drilled, etc.). Loose vocabulary — both `TaskMethod` and
/// [`TaskImplement`] are stored as flat tables the user manages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskMethod {
    pub id: crate::ids::TaskMethodId,
    pub name: String,
    pub notes: Option<String>,
}

impl TaskMethod {
    pub fn new(name: impl Into<String>, notes: Option<String>) -> DomainResult<Self> {
        Ok(Self {
            id: crate::ids::TaskMethodId::new(),
            name: require_name(name)?,
            notes: normalize_optional(notes),
        })
    }
}

/// A tool / implement used for a task (hoe, hand seeder, tractor + plough…).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskImplement {
    pub id: crate::ids::TaskImplementId,
    pub name: String,
    pub notes: Option<String>,
}

impl TaskImplement {
    pub fn new(name: impl Into<String>, notes: Option<String>) -> DomainResult<Self> {
        Ok(Self {
            id: crate::ids::TaskImplementId::new(),
            name: require_name(name)?,
            notes: normalize_optional(notes),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_type_accepts_valid_hex_colors() {
        assert!(TaskType::new("Semis", TaskCategory::Sow, "#3C6E47").is_ok());
        assert!(TaskType::new("Semis", TaskCategory::Sow, "#abc").is_ok()); // 3-digit form
    }

    #[test]
    fn task_type_rejects_bad_colors() {
        assert!(TaskType::new("Semis", TaskCategory::Sow, "green").is_err());
        assert!(TaskType::new("Semis", TaskCategory::Sow, "#xyzxyz").is_err());
        assert!(TaskType::new("Semis", TaskCategory::Sow, "#12345").is_err()); // wrong length
    }

    #[test]
    fn task_type_rejects_blank_name() {
        assert!(TaskType::new("   ", TaskCategory::Sow, "#3C6E47").is_err());
    }

    #[test]
    fn category_serializes_as_snake_case() {
        let s = serde_json::to_string(&TaskCategory::Tillage).unwrap();
        assert_eq!(s, "\"tillage\"");
        // And round-trips:
        let back: TaskCategory = serde_json::from_str("\"transplant\"").unwrap();
        assert_eq!(back, TaskCategory::Transplant);
    }

    #[test]
    fn task_method_and_implement_normalize_notes() {
        let m = TaskMethod::new("Manuel", Some("   ".into())).unwrap();
        assert_eq!(m.notes, None);
        let i = TaskImplement::new("Houe", Some(" pour le sarclage ".into())).unwrap();
        assert_eq!(i.notes.as_deref(), Some("pour le sarclage"));
    }

    #[test]
    fn rename_keeps_id_and_category() {
        let t = TaskType::new("Semis", TaskCategory::Sow, "#3C6E47").unwrap();
        let id = t.id;
        let r = t.rename("Semis direct", "#244529").unwrap();
        assert_eq!(r.id, id);
        assert_eq!(r.category, TaskCategory::Sow);
        assert_eq!(r.name, "Semis direct");
        assert_eq!(r.color, "#244529");
    }
}
