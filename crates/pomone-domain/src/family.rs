//! Botanical family (Solanaceae, Rosaceae, Fabaceae…).
//!
//! Families are user-managed: a fresh database is seeded with common families,
//! but the user can add, rename, or remove them.

use crate::error::DomainResult;
use crate::ids::FamilyId;
use crate::validation::{normalize_optional, require_name};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Family {
    pub id: FamilyId,
    pub name: String,
    pub latin_name: Option<String>,
    pub description: Option<String>,
}

impl Family {
    pub fn new(
        name: impl Into<String>,
        latin_name: Option<String>,
        description: Option<String>,
    ) -> DomainResult<Self> {
        Ok(Self {
            id: FamilyId::new(),
            name: require_name(name)?,
            latin_name: normalize_optional(latin_name),
            description: normalize_optional(description),
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
