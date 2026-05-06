//! Kind of location (parcel, bed, greenhouse, orchard, agroforestry row, hedge…).
//!
//! User-managed: a fresh database is seeded with common kinds, but the user
//! can rename, add or remove them.

use crate::error::DomainResult;
use crate::ids::LocationKindId;
use crate::validation::{normalize_optional, require_name};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocationKind {
    pub id: LocationKindId,
    pub name: String,
    pub description: Option<String>,
}

impl LocationKind {
    pub fn new(name: impl Into<String>, description: Option<String>) -> DomainResult<Self> {
        Ok(Self {
            id: LocationKindId::new(),
            name: require_name(name)?,
            description: normalize_optional(description),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::DomainError;

    #[test]
    fn new_trims_name_and_description() {
        let k = LocationKind::new("  Verger  ", Some("  arbres fruitiers  ".into())).unwrap();
        assert_eq!(k.name, "Verger");
        assert_eq!(k.description.as_deref(), Some("arbres fruitiers"));
    }

    #[test]
    fn description_optional() {
        let k = LocationKind::new("Haie", None).unwrap();
        assert!(k.description.is_none());
    }

    #[test]
    fn empty_name_rejected() {
        assert_eq!(LocationKind::new("", None), Err(DomainError::EmptyName));
    }
}
