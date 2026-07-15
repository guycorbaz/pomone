//! Strongly-typed identifiers (newtype UUIDs).
//!
//! Each entity has its own ID type so they can't be confused at compile time
//! (e.g. passing a `CropId` where a `VarietyId` is expected is a type error).

use uuid::Uuid;

macro_rules! define_id {
    ($name:ident, $doc:expr) => {
        #[doc = $doc]
        #[derive(
            Debug,
            Clone,
            Copy,
            PartialEq,
            Eq,
            Hash,
            PartialOrd,
            Ord,
            serde::Serialize,
            serde::Deserialize,
        )]
        #[repr(transparent)]
        #[serde(transparent)]
        pub struct $name(Uuid);

        impl $name {
            /// Generate a fresh random v4 identifier.
            #[must_use]
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }

            /// Wrap an existing UUID into this typed identifier.
            #[must_use]
            pub const fn from_uuid(uuid: Uuid) -> Self {
                Self(uuid)
            }

            /// Return the underlying UUID.
            #[must_use]
            pub const fn as_uuid(self) -> Uuid {
                self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                std::fmt::Display::fmt(&self.0, f)
            }
        }

        impl From<Uuid> for $name {
            fn from(uuid: Uuid) -> Self {
                Self(uuid)
            }
        }

        impl From<$name> for Uuid {
            fn from(id: $name) -> Self {
                id.0
            }
        }
    };
}

define_id!(
    FamilyId,
    "Identifier for a botanical family (e.g. Solanaceae)."
);
define_id!(
    StrataId,
    "Identifier for an agroforestry stratum (canopy, shrub layer…)."
);
define_id!(
    LocationKindId,
    "Identifier for a kind of location (field, bed, orchard…)."
);
define_id!(
    LocationId,
    "Identifier for a physical location (parcel, bed, orchard row…)."
);
define_id!(
    CropId,
    "Identifier for a crop (Tomato, Apple tree, Raspberry…)."
);
define_id!(VarietyId, "Identifier for a specific variety of a crop.");
define_id!(
    PlantingId,
    "Identifier for a concrete planting (variety + location + schedule)."
);
define_id!(
    TaskTypeId,
    "Identifier for a task type (Sow, Transplant, Harvest, Weeding…)."
);
define_id!(
    TaskMethodId,
    "Identifier for the method used by a task (Manual, Mechanized…)."
);
define_id!(
    TaskImplementId,
    "Identifier for the tool used by a task (Hoe, Tractor, Seeder…)."
);
define_id!(TaskId, "Identifier for a concrete task / operation.");
define_id!(
    TreatmentId,
    "Identifier for a phytosanitary treatment applied to a planting."
);
define_id!(
    TaskSeriesId,
    "Identifier for a recurring task series (template that materializes \
     into one Task per occurrence)."
);
define_id!(
    FieldEventId,
    "Identifier for an append-only field-journal event."
);
define_id!(
    CropPlanLineId,
    "Identifier for a crop-plan line (a planned variety × series × bed-meters, \
     staggered, that later generates plantings)."
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique() {
        let a = CropId::new();
        let b = CropId::new();
        assert_ne!(a, b, "two freshly-generated IDs must differ");
    }

    #[test]
    fn ids_of_different_types_have_distinct_compile_time_types() {
        // This is a compile-time guarantee, but we exercise it.
        let crop_id = CropId::new();
        let variety_id = VarietyId::new();
        // Both have the same internal Uuid representation, but they cannot be
        // compared directly — that's the whole point.
        assert_ne!(crop_id.as_uuid(), variety_id.as_uuid());
    }

    #[test]
    fn from_uuid_roundtrip() {
        let uuid = Uuid::new_v4();
        let id = LocationId::from_uuid(uuid);
        assert_eq!(id.as_uuid(), uuid);
        let back: Uuid = id.into();
        assert_eq!(back, uuid);
    }

    #[test]
    fn serde_roundtrip_is_transparent() {
        let id = PlantingId::new();
        let json = serde_json::to_string(&id).unwrap();
        // Should serialize as a bare UUID string, not wrapped in an object.
        assert!(json.starts_with('"') && json.ends_with('"'));
        let parsed: PlantingId = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, id);
    }

    #[test]
    fn display_matches_uuid() {
        let uuid = Uuid::new_v4();
        let id = FamilyId::from_uuid(uuid);
        assert_eq!(id.to_string(), uuid.to_string());
    }

    #[test]
    fn default_generates_fresh_id() {
        let a: StrataId = StrataId::default();
        let b: StrataId = StrataId::default();
        assert_ne!(a, b);
    }
}
