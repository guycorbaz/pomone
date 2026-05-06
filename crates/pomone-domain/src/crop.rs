//! Crops (Tomato, Apple, Raspberry…) and their biological lifespan.
//!
//! The `Lifespan` distinction (Annual vs Pluriannual) is the core of the
//! domain: it determines how plantings are scheduled and how harvests are
//! recorded.

use crate::error::{DomainError, DomainResult};
use crate::ids::{CropId, FamilyId, StrataId};
use crate::validation::{normalize_optional, require_name};
use serde::{Deserialize, Serialize};

/// When (if ever) a crop needs pruning.
///
/// This is a small, fixed set of agronomic categories — kept as a Rust enum
/// (not a user-managed table) on purpose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PruningSeason {
    Winter,
    Summer,
    Both,
    None,
}

impl PruningSeason {
    #[must_use]
    pub const fn requires_pruning(self) -> bool {
        !matches!(self, Self::None)
    }
}

/// How a pluriannual crop produces.
///
/// - `SingleCycle`: the plant goes through ONE long production/seed cycle then
///   dies (true biennials kept for seed, monocarpic perennials).
/// - `Recurring`: the plant produces every year for many years (apple tree,
///   raspberry, asparagus…), with a possible delay before first yield.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ProductivePattern {
    SingleCycle,
    Recurring { years_to_first_yield: u8 },
}

/// Biological lifespan of a crop.
///
/// - `Annual`: complete cycle in one season, dies (tomato, lettuce, bean).
/// - `Pluriannual`: lives more than one year (biennial or true perennial).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Lifespan {
    Annual,
    Pluriannual {
        pattern: ProductivePattern,
        lifespan_years: u8,
    },
}

impl Lifespan {
    /// Convenience: a true annual.
    #[must_use]
    pub const fn annual() -> Self {
        Self::Annual
    }

    /// A biennial kept for seed (or any monocarpic plant with a 2-year cycle).
    pub fn biennial() -> DomainResult<Self> {
        Self::pluriannual_single_cycle(2)
    }

    /// A pluriannual crop with a single long cycle (length in years).
    pub fn pluriannual_single_cycle(lifespan_years: u8) -> DomainResult<Self> {
        if lifespan_years < 2 {
            return Err(DomainError::PluriannualLifespanTooShort(lifespan_years));
        }
        Ok(Self::Pluriannual {
            pattern: ProductivePattern::SingleCycle,
            lifespan_years,
        })
    }

    /// A truly perennial crop producing every year.
    ///
    /// `years_to_first_yield` must be strictly less than `lifespan_years`
    /// (otherwise the plant would never produce within its lifespan).
    pub fn perennial(lifespan_years: u8, years_to_first_yield: u8) -> DomainResult<Self> {
        if lifespan_years < 2 {
            return Err(DomainError::PluriannualLifespanTooShort(lifespan_years));
        }
        if years_to_first_yield >= lifespan_years {
            return Err(DomainError::YearsToFirstYieldTooHigh {
                first_yield: years_to_first_yield,
                lifespan: lifespan_years,
            });
        }
        Ok(Self::Pluriannual {
            pattern: ProductivePattern::Recurring {
                years_to_first_yield,
            },
            lifespan_years,
        })
    }

    #[must_use]
    pub const fn is_annual(self) -> bool {
        matches!(self, Self::Annual)
    }

    #[must_use]
    pub const fn is_pluriannual(self) -> bool {
        matches!(self, Self::Pluriannual { .. })
    }

    /// True for true perennials that produce year after year.
    #[must_use]
    pub const fn is_recurring(self) -> bool {
        matches!(
            self,
            Self::Pluriannual {
                pattern: ProductivePattern::Recurring { .. },
                ..
            }
        )
    }

    /// Total years this crop is expected to live (1 for annuals).
    #[must_use]
    pub const fn lifespan_years(self) -> u8 {
        match self {
            Self::Annual => 1,
            Self::Pluriannual { lifespan_years, .. } => lifespan_years,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Crop {
    pub id: CropId,
    pub family_id: FamilyId,
    pub strata_id: StrataId,
    pub name: String,
    pub latin_name: Option<String>,
    pub lifespan: Lifespan,
    pub pruning_season: PruningSeason,
}

impl Crop {
    pub fn new(
        family_id: FamilyId,
        strata_id: StrataId,
        name: impl Into<String>,
        latin_name: Option<String>,
        lifespan: Lifespan,
        pruning_season: PruningSeason,
    ) -> DomainResult<Self> {
        Ok(Self {
            id: CropId::new(),
            family_id,
            strata_id,
            name: require_name(name)?,
            latin_name: normalize_optional(latin_name),
            lifespan,
            pruning_season,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pruning_season_requires_pruning_correctly() {
        assert!(PruningSeason::Winter.requires_pruning());
        assert!(PruningSeason::Summer.requires_pruning());
        assert!(PruningSeason::Both.requires_pruning());
        assert!(!PruningSeason::None.requires_pruning());
    }

    #[test]
    fn annual_lifespan() {
        let l = Lifespan::annual();
        assert!(l.is_annual());
        assert!(!l.is_pluriannual());
        assert!(!l.is_recurring());
        assert_eq!(l.lifespan_years(), 1);
    }

    #[test]
    fn biennial_helper_yields_two_year_single_cycle() {
        let l = Lifespan::biennial().unwrap();
        assert!(l.is_pluriannual());
        assert!(!l.is_recurring());
        assert_eq!(l.lifespan_years(), 2);
    }

    #[test]
    fn perennial_with_first_yield_year_three() {
        let l = Lifespan::perennial(30, 3).unwrap();
        assert!(l.is_pluriannual());
        assert!(l.is_recurring());
        assert_eq!(l.lifespan_years(), 30);
    }

    #[test]
    fn pluriannual_single_cycle_short_lifespan_rejected() {
        assert_eq!(
            Lifespan::pluriannual_single_cycle(1),
            Err(DomainError::PluriannualLifespanTooShort(1))
        );
        assert_eq!(
            Lifespan::pluriannual_single_cycle(0),
            Err(DomainError::PluriannualLifespanTooShort(0))
        );
    }

    #[test]
    fn perennial_first_yield_must_be_strictly_less_than_lifespan() {
        // 0 first yield = produces immediately, OK
        assert!(Lifespan::perennial(5, 0).is_ok());
        // first yield = lifespan - 1, OK
        assert!(Lifespan::perennial(5, 4).is_ok());
        // first yield == lifespan, rejected
        assert_eq!(
            Lifespan::perennial(5, 5),
            Err(DomainError::YearsToFirstYieldTooHigh {
                first_yield: 5,
                lifespan: 5,
            })
        );
        // first yield > lifespan, rejected
        assert!(matches!(
            Lifespan::perennial(5, 10),
            Err(DomainError::YearsToFirstYieldTooHigh { .. })
        ));
    }

    #[test]
    fn perennial_lifespan_must_be_at_least_two() {
        assert_eq!(
            Lifespan::perennial(1, 0),
            Err(DomainError::PluriannualLifespanTooShort(1))
        );
    }

    #[test]
    fn crop_construction_normalizes_fields() {
        let crop = Crop::new(
            FamilyId::new(),
            StrataId::new(),
            "  Pommier  ",
            Some("  Malus domestica  ".to_owned()),
            Lifespan::perennial(40, 3).unwrap(),
            PruningSeason::Winter,
        )
        .unwrap();
        assert_eq!(crop.name, "Pommier");
        assert_eq!(crop.latin_name.as_deref(), Some("Malus domestica"));
        assert!(crop.lifespan.is_recurring());
    }

    #[test]
    fn crop_with_empty_name_rejected() {
        let res = Crop::new(
            FamilyId::new(),
            StrataId::new(),
            "",
            None,
            Lifespan::annual(),
            PruningSeason::None,
        );
        assert_eq!(res, Err(DomainError::EmptyName));
    }

    #[test]
    fn lifespan_serde_uses_kind_tag() {
        let l = Lifespan::perennial(30, 3).unwrap();
        let json = serde_json::to_string(&l).unwrap();
        assert!(json.contains("\"kind\":\"pluriannual\""));
        assert!(json.contains("\"kind\":\"recurring\""));
        let parsed: Lifespan = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, l);
    }
}
