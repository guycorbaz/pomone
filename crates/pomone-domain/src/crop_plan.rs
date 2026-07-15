//! Crop-plan lines — the winter plan, one line per intention.
//!
//! A [`CropPlanLine`] is a durable planning intent the grower enters
//! spreadsheet-style (Epic 2): "grow this variety as `series` staggered
//! successions of `bed_meters` each, `stagger_days` apart". It is **not** a
//! planting yet — story 2.6 generates the N staggered plantings from a line;
//! story 2.7 aggregates the seed/plant needs from all non-draft lines even
//! before any placement.
//!
//! The line carries a `draft` flag that is **orthogonal to validity**: a line
//! can be fully valid (positive series and meters) yet still a draft the grower
//! is refining, and drafts are excluded from generation and the needs list
//! until promoted. Quantity is modeled as *series × bed-geometry* (bed-meters
//! here in R1; a polymorphic occupancy discriminant is deferred).

use crate::error::{DomainError, DomainResult};
use crate::ids::{CropPlanLineId, VarietyId};
use crate::validation::normalize_optional;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// One line of the winter crop plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CropPlanLine {
    pub id: CropPlanLineId,
    /// The planned variety (a variety belongs to a crop, so the crop is
    /// derivable). Needs aggregation (story 2.7) groups by this.
    pub variety_id: VarietyId,
    /// Number of staggered successions. Must be ≥ 1.
    pub series: u32,
    /// Bed-meters occupied by **each** succession. Must be > 0.
    pub bed_meters: Decimal,
    /// Days between successive successions (0 = all on the same date). ≥ 0 is
    /// guaranteed by the unsigned type.
    pub stagger_days: u32,
    /// Whether the line is still a draft — excluded from generation and needs
    /// until promoted. Orthogonal to validity.
    pub draft: bool,
    /// Free-form notes (normalized: blank-only collapses to `None`).
    pub notes: Option<String>,
}

impl CropPlanLine {
    /// Build a new plan line with a fresh id, validating the quantity model.
    ///
    /// Enforces `series ≥ 1` and `bed_meters > 0`. `stagger_days` needs no
    /// check (unsigned). `draft` is orthogonal — a draft line is still
    /// validated, so an invalid draft can't be saved.
    pub fn new(
        variety_id: VarietyId,
        series: u32,
        bed_meters: Decimal,
        stagger_days: u32,
        draft: bool,
        notes: Option<String>,
    ) -> DomainResult<Self> {
        if series == 0 {
            return Err(DomainError::NonPositiveCount(series));
        }
        if bed_meters <= Decimal::ZERO {
            return Err(DomainError::NonPositiveValue {
                field: "bed_meters",
                value: bed_meters,
            });
        }
        Ok(Self {
            id: CropPlanLineId::new(),
            variety_id,
            series,
            bed_meters,
            stagger_days,
            draft,
            notes: normalize_optional(notes),
        })
    }

    /// Rebuild a line keeping its identity — the edit path (story 2.3+). Same
    /// invariants as [`CropPlanLine::new`].
    pub fn with_updates(
        self,
        variety_id: VarietyId,
        series: u32,
        bed_meters: Decimal,
        stagger_days: u32,
        draft: bool,
        notes: Option<String>,
    ) -> DomainResult<Self> {
        let mut updated = Self::new(variety_id, series, bed_meters, stagger_days, draft, notes)?;
        updated.id = self.id;
        Ok(updated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn new_builds_a_valid_line_and_normalizes_notes() {
        let line = CropPlanLine::new(
            VarietyId::new(),
            6,
            dec!(15),
            14,
            true,
            Some("  laitue batavia  ".to_owned()),
        )
        .unwrap();
        assert_eq!(line.series, 6);
        assert_eq!(line.bed_meters, dec!(15));
        assert_eq!(line.stagger_days, 14);
        assert!(line.draft);
        assert_eq!(line.notes.as_deref(), Some("laitue batavia"));
    }

    #[test]
    fn stagger_of_zero_is_allowed() {
        let line = CropPlanLine::new(VarietyId::new(), 1, dec!(10), 0, false, None).unwrap();
        assert_eq!(line.stagger_days, 0);
    }

    #[test]
    fn zero_series_is_rejected() {
        let res = CropPlanLine::new(VarietyId::new(), 0, dec!(15), 14, false, None);
        assert!(matches!(res, Err(DomainError::NonPositiveCount(0))));
    }

    #[test]
    fn non_positive_bed_meters_is_rejected() {
        for m in [dec!(0), dec!(-1.5)] {
            let res = CropPlanLine::new(VarietyId::new(), 3, m, 7, false, None);
            assert!(matches!(
                res,
                Err(DomainError::NonPositiveValue {
                    field: "bed_meters",
                    ..
                })
            ));
        }
    }

    #[test]
    fn draft_is_orthogonal_to_validity() {
        // A draft line must still be valid to be built.
        assert!(CropPlanLine::new(VarietyId::new(), 0, dec!(15), 0, true, None).is_err());
        assert!(CropPlanLine::new(VarietyId::new(), 2, dec!(15), 0, true, None).is_ok());
    }

    #[test]
    fn with_updates_keeps_identity() {
        let line = CropPlanLine::new(VarietyId::new(), 3, dec!(10), 7, true, None).unwrap();
        let id = line.id;
        let new_variety = VarietyId::new();
        let updated = line
            .with_updates(new_variety, 5, dec!(20), 10, false, Some("promu".into()))
            .unwrap();
        assert_eq!(updated.id, id);
        assert_eq!(updated.variety_id, new_variety);
        assert_eq!(updated.series, 5);
        assert!(!updated.draft);
    }

    #[test]
    fn each_line_gets_its_own_id() {
        let a = CropPlanLine::new(VarietyId::new(), 1, dec!(1), 0, false, None).unwrap();
        let b = CropPlanLine::new(VarietyId::new(), 1, dec!(1), 0, false, None).unwrap();
        assert_ne!(a.id, b.id);
    }
}
