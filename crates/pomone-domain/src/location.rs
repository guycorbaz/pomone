//! Physical locations (parcels, beds, orchards, agroforestry rows, hedges…).
//!
//! Locations form a hierarchy via `parent_id` (e.g. farm → field → bed).
//! The hierarchy is acyclic — but cycle detection is the repository's job,
//! not the domain's, since the domain has no view of other locations.

use crate::error::DomainResult;
use crate::ids::{LocationId, LocationKindId};
use crate::validation::{normalize_optional, require_name, require_positive_area};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Location {
    pub id: LocationId,
    pub parent_id: Option<LocationId>,
    pub kind_id: LocationKindId,
    pub name: String,
    pub area_m2: Decimal,
    pub notes: Option<String>,
}

impl Location {
    pub fn new(
        kind_id: LocationKindId,
        name: impl Into<String>,
        area_m2: Decimal,
        parent_id: Option<LocationId>,
        notes: Option<String>,
    ) -> DomainResult<Self> {
        Ok(Self {
            id: LocationId::new(),
            parent_id,
            kind_id,
            name: require_name(name)?,
            area_m2: require_positive_area(area_m2)?,
            notes: normalize_optional(notes),
        })
    }

    /// True if this location sits at the top of the hierarchy.
    #[must_use]
    pub const fn is_root(&self) -> bool {
        self.parent_id.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::DomainError;
    use rust_decimal_macros::dec;

    fn kind() -> LocationKindId {
        LocationKindId::new()
    }

    #[test]
    fn root_location_has_no_parent() {
        let loc = Location::new(kind(), "Ferme", dec!(50000.0), None, None).unwrap();
        assert!(loc.is_root());
    }

    #[test]
    fn child_location_has_parent() {
        let root = Location::new(kind(), "Ferme", dec!(50000.0), None, None).unwrap();
        let child =
            Location::new(kind(), "Parcelle nord", dec!(2000.0), Some(root.id), None).unwrap();
        assert!(!child.is_root());
        assert_eq!(child.parent_id, Some(root.id));
    }

    #[test]
    fn zero_or_negative_area_rejected() {
        let err = Location::new(kind(), "Vide", dec!(0), None, None).unwrap_err();
        assert_eq!(err, DomainError::NonPositiveArea(dec!(0)));

        let err = Location::new(kind(), "Anomalie", dec!(-5), None, None).unwrap_err();
        assert_eq!(err, DomainError::NonPositiveArea(dec!(-5)));
    }

    #[test]
    fn empty_name_rejected() {
        let res = Location::new(kind(), "  ", dec!(10), None, None);
        assert_eq!(res, Err(DomainError::EmptyName));
    }

    #[test]
    fn notes_normalized() {
        let loc = Location::new(kind(), "P1", dec!(10), None, Some("  ".to_owned())).unwrap();
        assert!(loc.notes.is_none());

        let loc =
            Location::new(kind(), "P2", dec!(10), None, Some("  hello  ".to_owned())).unwrap();
        assert_eq!(loc.notes.as_deref(), Some("hello"));
    }
}
