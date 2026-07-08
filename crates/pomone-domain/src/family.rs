//! Botanical family (Solanaceae, Rosaceae, Fabaceae…).
//!
//! Families are user-managed: a fresh database is seeded with common families,
//! but the user can add, rename, or remove them.

use crate::error::DomainResult;
use crate::ids::FamilyId;
use crate::validation::{normalize_optional, require_hex_color, require_name};
use serde::{Deserialize, Serialize};

/// Colour a family falls back to when none is picked. A neutral terracotta-grey
/// that reads on both the light and dark surfaces (matches Qrop's `DEFAULT`).
pub const DEFAULT_FAMILY_COLOR: &str = "#6B5D4D";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Family {
    pub id: FamilyId,
    pub name: String,
    pub latin_name: Option<String>,
    pub description: Option<String>,
    /// User-configurable colour (`#RGB` / `#RRGGBB`) used to tint plantings and
    /// the crop map by botanical family, mirroring Qrop's `family.color`.
    pub color: String,
}

impl Family {
    /// Construct a family with the default colour. Kept at three arguments so
    /// the many existing call sites (seed, tests, migration) stay unchanged;
    /// use [`Family::new_with_color`] when a colour is supplied.
    pub fn new(
        name: impl Into<String>,
        latin_name: Option<String>,
        description: Option<String>,
    ) -> DomainResult<Self> {
        Self::new_with_color(name, latin_name, description, DEFAULT_FAMILY_COLOR)
    }

    /// Construct a family with an explicit, validated hex colour.
    pub fn new_with_color(
        name: impl Into<String>,
        latin_name: Option<String>,
        description: Option<String>,
        color: impl Into<String>,
    ) -> DomainResult<Self> {
        Ok(Self {
            id: FamilyId::new(),
            name: require_name(name)?,
            latin_name: normalize_optional(latin_name),
            description: normalize_optional(description),
            color: require_hex_color(color)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::DomainError;

    #[test]
    fn new_trims_and_normalizes() {
        let f = Family::new(
            "  Rosaceae  ",
            Some("  Rosaceae  ".into()),
            Some("  ".into()),
        )
        .unwrap();
        assert_eq!(f.name, "Rosaceae");
        assert_eq!(f.latin_name.as_deref(), Some("Rosaceae"));
        assert_eq!(f.description, None);
        assert_eq!(f.color, DEFAULT_FAMILY_COLOR);
    }

    #[test]
    fn new_with_color_trims_and_validates() {
        let f = Family::new_with_color("Rosaceae", None, None, "  #1A2B3C  ").unwrap();
        assert_eq!(f.color, "#1A2B3C");
    }

    #[test]
    fn new_with_color_rejects_bad_hex() {
        assert_eq!(
            Family::new_with_color("Rosaceae", None, None, "green"),
            Err(DomainError::InvalidHexColor("green".to_owned()))
        );
    }

    #[test]
    fn empty_name_rejected() {
        assert_eq!(Family::new("", None, None), Err(DomainError::EmptyName));
        assert_eq!(Family::new("   ", None, None), Err(DomainError::EmptyName));
    }

    #[test]
    fn each_family_has_a_unique_id() {
        let a = Family::new("Solanaceae", None, None).unwrap();
        let b = Family::new("Solanaceae", None, None).unwrap();
        assert_ne!(a.id, b.id);
    }
}
