//! Planned (generated, not-yet-placed) plantings.
//!
//! A [`PlannedPlanting`] is one succession materialized from a
//! [`crate::CropPlanLine`] (Epic 2, story 2.6): the plan says "6 × 15 m, 14 days
//! apart", so generation produces 6 planned plantings, stagger-dated and
//! line-linked, **without a placement** — no bed, no strata yet. Placement
//! (Epic 3) turns a planned planting into a real [`crate::Planting`] on a bed;
//! until then it feeds the two-phase decoupling (the needs list prints from
//! unplaced lines before any bed is chosen).
//!
//! The pair `(crop_plan_line_id, series_index)` is the natural key: regeneration
//! after a line edit updates the row in place, so anything downstream that later
//! links to a planned planting survives a re-generate (non-destructive).

use crate::error::{DomainError, DomainResult};
use crate::ids::{CropPlanLineId, PlannedPlantingId, PlantingId, VarietyId};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// One generated succession of a crop-plan line, not yet placed on a bed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannedPlanting {
    pub id: PlannedPlantingId,
    /// The plan line this succession was generated from.
    pub crop_plan_line_id: CropPlanLineId,
    /// The variety (snapshotted from the line at generation).
    pub variety_id: VarietyId,
    /// Which succession (0-based) of the line's `series` this is.
    pub series_index: u32,
    /// The staggered date of this succession (`first_on + stagger·index`).
    pub planned_on: NaiveDate,
    /// Bed-meters this succession needs (snapshotted from the line). > 0.
    pub bed_meters: Decimal,
    /// Set once this succession is **placed** on a bed (Epic 3, story 3.2): the
    /// id of the real [`crate::Planting`] it became. `None` while unplaced. The
    /// placement screen lists exactly the rows where this is `None`; un-placing
    /// clears it (and deletes the planting), returning the row to that list.
    pub placed_planting_id: Option<PlantingId>,
}

impl PlannedPlanting {
    /// Build a planned planting with a fresh id. `bed_meters` must be > 0
    /// (generation only ever passes a validated line's value).
    pub fn new(
        crop_plan_line_id: CropPlanLineId,
        variety_id: VarietyId,
        series_index: u32,
        planned_on: NaiveDate,
        bed_meters: Decimal,
    ) -> DomainResult<Self> {
        if bed_meters <= Decimal::ZERO {
            return Err(DomainError::NonPositiveValue {
                field: "bed_meters",
                value: bed_meters,
            });
        }
        Ok(Self {
            id: PlannedPlantingId::new(),
            crop_plan_line_id,
            variety_id,
            series_index,
            planned_on,
            bed_meters,
            placed_planting_id: None,
        })
    }

    /// True while this succession has not been placed on a bed — i.e. it still
    /// belongs on the placement screen's unplaced list.
    #[must_use]
    pub const fn is_placed(&self) -> bool {
        self.placed_planting_id.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    #[test]
    fn new_builds_and_rejects_non_positive_bed_meters() {
        let ok = PlannedPlanting::new(
            CropPlanLineId::new(),
            VarietyId::new(),
            0,
            d(2026, 4, 1),
            dec!(15),
        )
        .unwrap();
        assert_eq!(ok.series_index, 0);
        assert_eq!(ok.bed_meters, dec!(15));
        // A freshly generated succession is unplaced.
        assert_eq!(ok.placed_planting_id, None);
        assert!(!ok.is_placed());

        assert!(matches!(
            PlannedPlanting::new(
                CropPlanLineId::new(),
                VarietyId::new(),
                0,
                d(2026, 4, 1),
                dec!(0)
            ),
            Err(DomainError::NonPositiveValue {
                field: "bed_meters",
                ..
            })
        ));
    }

    #[test]
    fn each_planned_planting_gets_its_own_id() {
        let a = PlannedPlanting::new(
            CropPlanLineId::new(),
            VarietyId::new(),
            0,
            d(2026, 4, 1),
            dec!(1),
        )
        .unwrap();
        let b = PlannedPlanting::new(
            CropPlanLineId::new(),
            VarietyId::new(),
            0,
            d(2026, 4, 1),
            dec!(1),
        )
        .unwrap();
        assert_ne!(a.id, b.id);
    }
}
