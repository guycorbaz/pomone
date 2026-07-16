//! Physical locations (parcels, beds, orchards, agroforestry rows, hedges…).
//!
//! Locations form a hierarchy via `parent_id` (e.g. farm → field → bed).
//! The hierarchy is acyclic — but cycle detection is the repository's job,
//! not the domain's, since the domain has no view of other locations.

use crate::error::DomainResult;
use crate::ids::{LocationId, LocationKindId};
use crate::validation::{normalize_optional, require_name, require_positive_dimension};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// How a location's growing capacity is measured — the discriminant the
/// capacity engine reads when it computes soil occupancy.
///
/// R1 has a single kind, [`OccupationKind::BedMeters`]: the footprint of a
/// planting on a leaf (bed) location is the bed's `length_m` (the running
/// metres along the bed), with `width_m` kept informative only. The column
/// exists so future polymorphism (tree rows counted per-row, orchards counted
/// in hectares) is an *additive* variant here — not another migration.
///
/// Defaults to [`OccupationKind::BedMeters`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OccupationKind {
    /// Capacity measured in running bed-metres (leaf `length_m`). The only R1 kind.
    #[default]
    BedMeters,
}

/// A physical location. Always rectangular at the data level: vegetable beds
/// and greenhouses are naturally so, and irregular orchards / fields can be
/// stored as the rectangle that bounds them. Area is derived from `length_m`
/// × `width_m` rather than stored, which keeps the three values in sync by
/// construction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Location {
    pub id: LocationId,
    pub parent_id: Option<LocationId>,
    pub kind_id: LocationKindId,
    pub name: String,
    pub length_m: Decimal,
    pub width_m: Decimal,
    /// How this location's capacity is counted. See [`OccupationKind`].
    pub occupation_kind: OccupationKind,
    pub notes: Option<String>,
}

impl Location {
    pub fn new(
        kind_id: LocationKindId,
        name: impl Into<String>,
        length_m: Decimal,
        width_m: Decimal,
        parent_id: Option<LocationId>,
        notes: Option<String>,
    ) -> DomainResult<Self> {
        Ok(Self {
            id: LocationId::new(),
            parent_id,
            kind_id,
            name: require_name(name)?,
            length_m: require_positive_dimension(length_m, "length_m")?,
            width_m: require_positive_dimension(width_m, "width_m")?,
            occupation_kind: OccupationKind::default(),
            notes: normalize_optional(notes),
        })
    }

    /// Builder: set how this location's capacity is measured. Returns `self`
    /// so it chains off [`Location::new`], mirroring
    /// [`crate::LocationKind::with_covered`]. R1 only ever uses the default
    /// ([`OccupationKind::BedMeters`]); this exists for forward compatibility.
    #[must_use]
    pub const fn with_occupation_kind(mut self, kind: OccupationKind) -> Self {
        self.occupation_kind = kind;
        self
    }

    /// Derived area in m². Computed on demand from `length_m × width_m`.
    #[must_use]
    pub fn area_m2(&self) -> Decimal {
        self.length_m * self.width_m
    }

    /// The R1 capacity **footprint** of this location, per the bed-meters rule:
    /// a planting placed on a leaf bed occupies the bed's `length_m` (running
    /// metres), `width_m` being informative only. Callers apply this only to
    /// leaf (bed) locations — the capacity engine sums these footprints.
    #[must_use]
    pub const fn bed_meters(&self) -> Decimal {
        self.length_m
    }

    /// True if this location sits at the top of the hierarchy.
    #[must_use]
    pub const fn is_root(&self) -> bool {
        self.parent_id.is_none()
    }
}

/// Resolve the covered/open split for a bed, per the geometry rule: a leaf bed
/// is **sheltered** iff its own location-kind *or any ancestor location's kind*
/// is `covered`. `covered_root_to_leaf` is the `covered` flag of the leaf and
/// every ancestor (order irrelevant — any `true` wins). A pure fold, kept here
/// so the engine and the future placement view resolve shelter identically.
#[must_use]
pub fn is_sheltered<I: IntoIterator<Item = bool>>(covered_of_leaf_and_ancestors: I) -> bool {
    covered_of_leaf_and_ancestors
        .into_iter()
        .any(|covered| covered)
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
        let loc = Location::new(kind(), "Ferme", dec!(250), dec!(200), None, None).unwrap();
        assert!(loc.is_root());
    }

    #[test]
    fn child_location_has_parent() {
        let root = Location::new(kind(), "Ferme", dec!(250), dec!(200), None, None).unwrap();
        let child = Location::new(
            kind(),
            "Parcelle nord",
            dec!(40),
            dec!(50),
            Some(root.id),
            None,
        )
        .unwrap();
        assert!(!child.is_root());
        assert_eq!(child.parent_id, Some(root.id));
    }

    #[test]
    fn area_is_derived_from_dimensions() {
        let loc = Location::new(kind(), "Planche A", dec!(25), dec!(0.8), None, None).unwrap();
        assert_eq!(loc.area_m2(), dec!(20.0));
    }

    #[test]
    fn zero_or_negative_length_rejected() {
        let err = Location::new(kind(), "Vide", dec!(0), dec!(1), None, None).unwrap_err();
        assert_eq!(
            err,
            DomainError::NonPositiveValue {
                field: "length_m",
                value: dec!(0),
            }
        );

        let err = Location::new(kind(), "Anomalie", dec!(-5), dec!(1), None, None).unwrap_err();
        assert_eq!(
            err,
            DomainError::NonPositiveValue {
                field: "length_m",
                value: dec!(-5),
            }
        );
    }

    #[test]
    fn zero_or_negative_width_rejected() {
        let err = Location::new(kind(), "Vide", dec!(1), dec!(0), None, None).unwrap_err();
        assert_eq!(
            err,
            DomainError::NonPositiveValue {
                field: "width_m",
                value: dec!(0),
            }
        );
    }

    #[test]
    fn empty_name_rejected() {
        let res = Location::new(kind(), "  ", dec!(10), dec!(1), None, None);
        assert_eq!(res, Err(DomainError::EmptyName));
    }

    #[test]
    fn occupation_kind_defaults_to_bed_meters() {
        let loc = Location::new(kind(), "Planche A", dec!(25), dec!(0.8), None, None).unwrap();
        assert_eq!(loc.occupation_kind, OccupationKind::BedMeters);
    }

    #[test]
    fn with_occupation_kind_builder_sets_it() {
        let loc = Location::new(kind(), "Planche A", dec!(25), dec!(0.8), None, None)
            .unwrap()
            .with_occupation_kind(OccupationKind::BedMeters);
        assert_eq!(loc.occupation_kind, OccupationKind::BedMeters);
    }

    #[test]
    fn bed_meters_footprint_is_length_not_area() {
        // Bed-meters rule: footprint = length_m (running metres), NOT area.
        let loc = Location::new(kind(), "Planche A", dec!(25), dec!(0.8), None, None).unwrap();
        assert_eq!(loc.bed_meters(), dec!(25));
        assert_ne!(loc.bed_meters(), loc.area_m2());
    }

    #[test]
    fn sheltered_when_leaf_or_any_ancestor_covered() {
        // Leaf open, one ancestor covered → sheltered.
        assert!(is_sheltered([false, true, false]));
        // Leaf itself covered → sheltered.
        assert!(is_sheltered([true, false]));
        // Nothing covered anywhere → open field.
        assert!(!is_sheltered([false, false, false]));
        // Empty chain (defensive) → open field.
        assert!(!is_sheltered(std::iter::empty()));
    }

    #[test]
    fn notes_normalized() {
        let loc =
            Location::new(kind(), "P1", dec!(10), dec!(1), None, Some("  ".to_owned())).unwrap();
        assert!(loc.notes.is_none());

        let loc = Location::new(
            kind(),
            "P2",
            dec!(10),
            dec!(1),
            None,
            Some("  hello  ".to_owned()),
        )
        .unwrap();
        assert_eq!(loc.notes.as_deref(), Some("hello"));
    }
}
