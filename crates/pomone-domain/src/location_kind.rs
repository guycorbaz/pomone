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
    /// `true` for kinds that grow under cover (greenhouse, tunnel…). Lets the
    /// home-page bed-usage curve split open-field beds from sheltered ones
    /// (issue #51). A bed counts as sheltered when its own kind — or any
    /// ancestor location's kind — is covered.
    pub covered: bool,
}

impl LocationKind {
    /// Build a kind. Defaults to open-field (`covered = false`); chain
    /// [`LocationKind::with_covered`] to mark it as under cover.
    pub fn new(name: impl Into<String>, description: Option<String>) -> DomainResult<Self> {
        Ok(Self {
            id: LocationKindId::new(),
            name: require_name(name)?,
            description: normalize_optional(description),
            covered: false,
        })
    }

    /// Builder: set whether this kind grows under cover. Returns `self` so it
    /// chains off [`LocationKind::new`].
    #[must_use]
    pub fn with_covered(mut self, covered: bool) -> Self {
        self.covered = covered;
        self
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
    fn covered_defaults_false_and_builder_sets_it() {
        let open = LocationKind::new("Parcelle", None).unwrap();
        assert!(!open.covered);
        let sheltered = LocationKind::new("Serre", None).unwrap().with_covered(true);
        assert!(sheltered.covered);
    }

    #[test]
    fn empty_name_rejected() {
        assert_eq!(LocationKind::new("", None), Err(DomainError::EmptyName));
    }
}
