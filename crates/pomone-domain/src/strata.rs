//! Vegetation stratum (canopy, shrub, herbaceous, ground cover…).
//!
//! Strata are user-managed. A fresh database is seeded with the seven classic
//! agroforestry / permaculture strata, but the user can edit them.

use crate::error::{DomainError, DomainResult};
use crate::ids::StrataId;
use crate::validation::{normalize_optional, require_name};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Strata {
    pub id: StrataId,
    pub name: String,
    pub description: Option<String>,
    pub min_height_m: Option<Decimal>,
    pub max_height_m: Option<Decimal>,
    /// Display order in the UI; lower values appear first.
    pub sort_order: i32,
}

impl Strata {
    pub fn new(
        name: impl Into<String>,
        description: Option<String>,
        min_height_m: Option<Decimal>,
        max_height_m: Option<Decimal>,
        sort_order: i32,
    ) -> DomainResult<Self> {
        if let (Some(min), Some(max)) = (min_height_m, max_height_m) {
            if min > max {
                return Err(DomainError::InvertedRange {
                    field: "height_m",
                    min: min.to_string(),
                    max: max.to_string(),
                });
            }
        }
        Ok(Self {
            id: StrataId::new(),
            name: require_name(name)?,
            description: normalize_optional(description),
            min_height_m,
            max_height_m,
            sort_order,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn new_normalizes_fields() {
        let s = Strata::new(
            "  Canopée  ",
            Some("  arbres > 6m  ".into()),
            Some(dec!(6.0)),
            Some(dec!(40.0)),
            10,
        )
        .unwrap();
        assert_eq!(s.name, "Canopée");
        assert_eq!(s.description.as_deref(), Some("arbres > 6m"));
        assert_eq!(s.min_height_m, Some(dec!(6.0)));
        assert_eq!(s.max_height_m, Some(dec!(40.0)));
        assert_eq!(s.sort_order, 10);
    }

    #[test]
    fn heights_can_be_omitted() {
        let s = Strata::new("Racinaire", None, None, None, 60).unwrap();
        assert!(s.min_height_m.is_none() && s.max_height_m.is_none());
    }

    #[test]
    fn inverted_height_range_rejected() {
        let res = Strata::new("Bizarre", None, Some(dec!(10.0)), Some(dec!(2.0)), 0);
        assert!(matches!(
            res,
            Err(crate::error::DomainError::InvertedRange {
                field: "height_m",
                ..
            })
        ));
    }

    #[test]
    fn equal_min_max_height_accepted() {
        let s = Strata::new("Pile", None, Some(dec!(2.0)), Some(dec!(2.0)), 0).unwrap();
        assert_eq!(s.min_height_m, Some(dec!(2.0)));
        assert_eq!(s.max_height_m, Some(dec!(2.0)));
    }
}
