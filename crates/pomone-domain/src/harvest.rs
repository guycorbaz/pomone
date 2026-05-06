//! Yearly harvest records for pluriannual recurring plantings.
//!
//! A `YearlyHarvest` row exists per (planting, year) pair for perennial
//! plantings only — annuals and single-cycle pluriannuals don't need them
//! since their entire production fits in their `PlantingSchedule::Cycle`.
//!
//! The point of this entity is to track the **evolution** of yield across
//! years for the same planting (ramp-up, plateau, decline).

use crate::error::{DomainError, DomainResult};
use crate::ids::PlantingId;
use crate::validation::normalize_optional;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct YearlyHarvest {
    pub planting_id: PlantingId,
    pub year: i32,
    pub expected_yield_kg: Option<Decimal>,
    pub actual_yield_kg: Option<Decimal>,
    pub notes: Option<String>,
}

impl YearlyHarvest {
    pub fn new(
        planting_id: PlantingId,
        year: i32,
        expected_yield_kg: Option<Decimal>,
        actual_yield_kg: Option<Decimal>,
        notes: Option<String>,
    ) -> DomainResult<Self> {
        if let Some(v) = expected_yield_kg {
            if v < Decimal::ZERO {
                return Err(DomainError::NegativeValue {
                    field: "expected_yield_kg",
                    value: v,
                });
            }
        }
        if let Some(v) = actual_yield_kg {
            if v < Decimal::ZERO {
                return Err(DomainError::NegativeValue {
                    field: "actual_yield_kg",
                    value: v,
                });
            }
        }
        Ok(Self {
            planting_id,
            year,
            expected_yield_kg,
            actual_yield_kg,
            notes: normalize_optional(notes),
        })
    }

    /// Difference between expected and actual yield, when both are recorded.
    /// Positive means we exceeded expectations.
    #[must_use]
    pub fn variance_kg(&self) -> Option<Decimal> {
        match (self.expected_yield_kg, self.actual_yield_kg) {
            (Some(exp), Some(act)) => Some(act - exp),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn new_accepts_zero_yield() {
        // total failure: predicted 100 kg, harvested 0 kg
        let h = YearlyHarvest::new(
            PlantingId::new(),
            2030,
            Some(dec!(100)),
            Some(dec!(0)),
            Some("late frost".into()),
        )
        .unwrap();
        assert_eq!(h.actual_yield_kg, Some(dec!(0)));
        assert_eq!(h.variance_kg(), Some(dec!(-100)));
    }

    #[test]
    fn new_accepts_only_expected() {
        let h = YearlyHarvest::new(PlantingId::new(), 2027, Some(dec!(20)), None, None).unwrap();
        assert!(h.actual_yield_kg.is_none());
        assert!(h.variance_kg().is_none());
    }

    #[test]
    fn new_accepts_only_actual() {
        let h = YearlyHarvest::new(PlantingId::new(), 2027, None, Some(dec!(35)), None).unwrap();
        assert!(h.expected_yield_kg.is_none());
        assert!(h.variance_kg().is_none());
    }

    #[test]
    fn new_rejects_negative_expected() {
        let res = YearlyHarvest::new(PlantingId::new(), 2027, Some(dec!(-1)), None, None);
        assert!(matches!(
            res,
            Err(DomainError::NegativeValue {
                field: "expected_yield_kg",
                ..
            })
        ));
    }

    #[test]
    fn new_rejects_negative_actual() {
        let res = YearlyHarvest::new(PlantingId::new(), 2027, None, Some(dec!(-0.5)), None);
        assert!(matches!(
            res,
            Err(DomainError::NegativeValue {
                field: "actual_yield_kg",
                ..
            })
        ));
    }

    #[test]
    fn variance_positive_when_exceeded() {
        let h = YearlyHarvest::new(
            PlantingId::new(),
            2030,
            Some(dec!(100)),
            Some(dec!(125)),
            None,
        )
        .unwrap();
        assert_eq!(h.variance_kg(), Some(dec!(25)));
    }

    #[test]
    fn notes_normalized() {
        let h = YearlyHarvest::new(
            PlantingId::new(),
            2027,
            None,
            Some(dec!(10)),
            Some("   ".to_owned()),
        )
        .unwrap();
        assert!(h.notes.is_none());
    }
}
