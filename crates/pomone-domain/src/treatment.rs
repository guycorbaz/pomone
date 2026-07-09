//! Phytosanitary treatments applied to plantings.
//!
//! A `Treatment` is a traceability record: on a given date, a product
//! (commercial brand carrying an active substance) was applied at a given
//! dose on a planting. The dose unit is deliberately free text ("L/ha",
//! "g/m²", "mL/L"…) — regulations vary and users know their own labels.
//!
//! Unlike [`crate::harvest::YearlyHarvest`], several treatments can target
//! the same planting on the same day, so each record has its own identity.

use crate::error::{DomainError, DomainResult};
use crate::ids::{PlantingId, TreatmentId};
use crate::validation::{normalize_optional, require_name};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Treatment {
    pub id: TreatmentId,
    pub planting_id: PlantingId,
    pub applied_on: NaiveDate,
    /// Active substance (e.g. "cuivre", "soufre", "spinosad").
    pub active_substance: String,
    /// Commercial product / brand name.
    pub product_name: String,
    /// Quantity applied, interpreted in `dose_unit`.
    pub dose: Decimal,
    /// Unit the dose is expressed in (e.g. "L/ha", "g/m²").
    pub dose_unit: String,
    pub notes: Option<String>,
}

impl Treatment {
    pub fn new(
        planting_id: PlantingId,
        applied_on: NaiveDate,
        active_substance: impl Into<String>,
        product_name: impl Into<String>,
        dose: Decimal,
        dose_unit: impl Into<String>,
        notes: Option<String>,
    ) -> DomainResult<Self> {
        if dose <= Decimal::ZERO {
            return Err(DomainError::NonPositiveValue {
                field: "dose",
                value: dose,
            });
        }
        Ok(Self {
            id: TreatmentId::new(),
            planting_id,
            applied_on,
            active_substance: require_name(active_substance)?,
            product_name: require_name(product_name)?,
            dose,
            dose_unit: require_name(dose_unit)?,
            notes: normalize_optional(notes),
        })
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
    fn new_builds_a_valid_treatment() {
        let t = Treatment::new(
            PlantingId::new(),
            d(2026, 6, 10),
            "  cuivre ",
            "Bouillie bordelaise",
            dec!(1.25),
            "kg/ha",
            Some("  avant pluie  ".to_owned()),
        )
        .unwrap();
        assert_eq!(t.active_substance, "cuivre");
        assert_eq!(t.product_name, "Bouillie bordelaise");
        assert_eq!(t.dose, dec!(1.25));
        assert_eq!(t.dose_unit, "kg/ha");
        assert_eq!(t.notes.as_deref(), Some("avant pluie"));
    }

    #[test]
    fn new_rejects_zero_or_negative_dose() {
        for dose in [dec!(0), dec!(-0.5)] {
            let res = Treatment::new(
                PlantingId::new(),
                d(2026, 6, 10),
                "soufre",
                "Thiovit",
                dose,
                "g/m²",
                None,
            );
            assert!(matches!(
                res,
                Err(DomainError::NonPositiveValue { field: "dose", .. })
            ));
        }
    }

    #[test]
    fn new_rejects_blank_substance_product_or_unit() {
        let pid = PlantingId::new();
        let date = d(2026, 6, 10);
        assert_eq!(
            Treatment::new(pid, date, "  ", "Thiovit", dec!(1), "g/m²", None),
            Err(DomainError::EmptyName)
        );
        assert_eq!(
            Treatment::new(pid, date, "soufre", "", dec!(1), "g/m²", None),
            Err(DomainError::EmptyName)
        );
        assert_eq!(
            Treatment::new(pid, date, "soufre", "Thiovit", dec!(1), " \t", None),
            Err(DomainError::EmptyName)
        );
    }

    #[test]
    fn each_treatment_gets_its_own_id() {
        let pid = PlantingId::new();
        let date = d(2026, 6, 10);
        let a = Treatment::new(pid, date, "soufre", "Thiovit", dec!(1), "g/m²", None).unwrap();
        let b = Treatment::new(pid, date, "soufre", "Thiovit", dec!(1), "g/m²", None).unwrap();
        assert_ne!(a.id, b.id);
    }

    #[test]
    fn blank_notes_are_normalized_to_none() {
        let t = Treatment::new(
            PlantingId::new(),
            d(2026, 6, 10),
            "spinosad",
            "Success 4",
            dec!(0.02),
            "L/ha",
            Some("   ".to_owned()),
        )
        .unwrap();
        assert!(t.notes.is_none());
    }
}
