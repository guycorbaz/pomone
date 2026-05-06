//! Varieties and their cultivation profiles.
//!
//! A `Variety` belongs to exactly one `Crop`. Its `VarietyProfile` must match
//! the crop's `Lifespan` — annual crops carry an `AnnualProfile`, pluriannual
//! crops carry a `PluriannualProfile`. This is enforced at construction time.

use crate::crop::Lifespan;
use crate::error::{DomainError, DomainResult};
use crate::ids::{CropId, VarietyId};
use crate::validation::{normalize_optional, require_name, require_valid_doy};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Cultivation profile of an annual variety (DTT/DTM/harvest window).
///
/// - `days_to_transplant` (DTT): days from sowing to transplanting; `None`
///   means direct sowing.
/// - `days_to_maturity` (DTM): days from sowing (or transplanting if DTT is
///   set) to first harvest.
/// - `harvest_window_days`: number of days during which harvest takes place
///   (1 = single-pick, larger = repeated picks).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnnualProfile {
    pub days_to_transplant: Option<u16>,
    pub days_to_maturity: u16,
    pub harvest_window_days: u16,
}

impl AnnualProfile {
    pub fn new(
        days_to_transplant: Option<u16>,
        days_to_maturity: u16,
        harvest_window_days: u16,
    ) -> DomainResult<Self> {
        if days_to_maturity == 0 {
            return Err(DomainError::NonPositiveDaysToMaturity);
        }
        if harvest_window_days == 0 {
            return Err(DomainError::EmptyHarvestWindow);
        }
        Ok(Self {
            days_to_transplant,
            days_to_maturity,
            harvest_window_days,
        })
    }
}

/// Cultivation profile of a pluriannual variety (phenology + harvest window
/// expressed in days-of-year).
///
/// `harvest_start_doy` and `harvest_end_doy` are stored independently and may
/// "wrap" around year-end (e.g. olive: October..February). Repository and UI
/// layers handle the wrap interpretation; the domain only validates each DOY
/// is in `1..=366`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluriannualProfile {
    pub bud_break_doy: Option<u16>,
    pub flowering_doy: Option<u16>,
    pub harvest_start_doy: u16,
    pub harvest_end_doy: u16,
    pub expected_yield_kg_per_plant: Option<Decimal>,
}

impl PluriannualProfile {
    pub fn new(
        bud_break_doy: Option<u16>,
        flowering_doy: Option<u16>,
        harvest_start_doy: u16,
        harvest_end_doy: u16,
        expected_yield_kg_per_plant: Option<Decimal>,
    ) -> DomainResult<Self> {
        if let Some(d) = bud_break_doy {
            require_valid_doy(d)?;
        }
        if let Some(d) = flowering_doy {
            require_valid_doy(d)?;
        }
        require_valid_doy(harvest_start_doy)?;
        require_valid_doy(harvest_end_doy)?;
        if let Some(y) = expected_yield_kg_per_plant {
            if y <= Decimal::ZERO {
                return Err(DomainError::NonPositiveValue {
                    field: "expected_yield_kg_per_plant",
                    value: y,
                });
            }
        }
        Ok(Self {
            bud_break_doy,
            flowering_doy,
            harvest_start_doy,
            harvest_end_doy,
            expected_yield_kg_per_plant,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum VarietyProfile {
    Annual(AnnualProfile),
    Pluriannual(PluriannualProfile),
}

impl VarietyProfile {
    /// Verify this profile is compatible with the parent crop's lifespan.
    pub fn check_compatible(&self, lifespan: Lifespan) -> DomainResult<()> {
        match (self, lifespan) {
            (Self::Annual(_), Lifespan::Annual)
            | (Self::Pluriannual(_), Lifespan::Pluriannual { .. }) => Ok(()),
            (Self::Annual(_), Lifespan::Pluriannual { .. }) => {
                Err(DomainError::VarietyProfileMismatch {
                    profile: "annual",
                    lifespan: "pluriannual",
                })
            }
            (Self::Pluriannual(_), Lifespan::Annual) => Err(DomainError::VarietyProfileMismatch {
                profile: "pluriannual",
                lifespan: "annual",
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Variety {
    pub id: VarietyId,
    pub crop_id: CropId,
    pub name: String,
    pub description: Option<String>,
    pub profile: VarietyProfile,
}

impl Variety {
    pub fn new(
        crop_id: CropId,
        crop_lifespan: Lifespan,
        name: impl Into<String>,
        description: Option<String>,
        profile: VarietyProfile,
    ) -> DomainResult<Self> {
        profile.check_compatible(crop_lifespan)?;
        Ok(Self {
            id: VarietyId::new(),
            crop_id,
            name: require_name(name)?,
            description: normalize_optional(description),
            profile,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn annual_profile_basic() {
        let p = AnnualProfile::new(Some(35), 70, 60).unwrap();
        assert_eq!(p.days_to_transplant, Some(35));
        assert_eq!(p.days_to_maturity, 70);
        assert_eq!(p.harvest_window_days, 60);
    }

    #[test]
    fn annual_profile_direct_sow_has_no_transplant() {
        let p = AnnualProfile::new(None, 60, 30).unwrap();
        assert!(p.days_to_transplant.is_none());
    }

    #[test]
    fn annual_profile_zero_dtm_rejected() {
        assert_eq!(
            AnnualProfile::new(None, 0, 30),
            Err(DomainError::NonPositiveDaysToMaturity)
        );
    }

    #[test]
    fn annual_profile_zero_harvest_window_rejected() {
        assert_eq!(
            AnnualProfile::new(None, 70, 0),
            Err(DomainError::EmptyHarvestWindow)
        );
    }

    #[test]
    fn pluriannual_profile_basic() {
        let p = PluriannualProfile::new(
            Some(80),  // bud break around late march
            Some(110), // flowering around late april
            220,       // harvest from early August
            280,       // through early October
            Some(dec!(15.5)),
        )
        .unwrap();
        assert_eq!(p.bud_break_doy, Some(80));
        assert_eq!(p.harvest_start_doy, 220);
        assert_eq!(p.harvest_end_doy, 280);
    }

    #[test]
    fn pluriannual_profile_allows_harvest_wrap() {
        // Olive-style: harvest from Oct (DOY 280) to Feb (DOY 50)
        let p = PluriannualProfile::new(None, None, 280, 50, None).unwrap();
        assert_eq!(p.harvest_start_doy, 280);
        assert_eq!(p.harvest_end_doy, 50);
    }

    #[test]
    fn pluriannual_profile_invalid_doy_rejected() {
        assert_eq!(
            PluriannualProfile::new(None, None, 0, 100, None),
            Err(DomainError::InvalidDayOfYear(0))
        );
        assert_eq!(
            PluriannualProfile::new(None, None, 100, 367, None),
            Err(DomainError::InvalidDayOfYear(367))
        );
    }

    #[test]
    fn pluriannual_profile_negative_yield_rejected() {
        let res = PluriannualProfile::new(None, None, 100, 200, Some(dec!(-1)));
        assert!(matches!(
            res,
            Err(DomainError::NonPositiveValue {
                field: "expected_yield_kg_per_plant",
                ..
            })
        ));
    }

    #[test]
    fn variety_profile_compatibility_matches_lifespan() {
        let annual = VarietyProfile::Annual(AnnualProfile::new(None, 60, 30).unwrap());
        let pluri = VarietyProfile::Pluriannual(
            PluriannualProfile::new(None, None, 100, 200, None).unwrap(),
        );
        assert!(annual.check_compatible(Lifespan::Annual).is_ok());
        assert!(pluri
            .check_compatible(Lifespan::perennial(20, 3).unwrap())
            .is_ok());

        assert!(matches!(
            annual.check_compatible(Lifespan::perennial(20, 3).unwrap()),
            Err(DomainError::VarietyProfileMismatch { .. })
        ));
        assert!(matches!(
            pluri.check_compatible(Lifespan::Annual),
            Err(DomainError::VarietyProfileMismatch { .. })
        ));
    }

    #[test]
    fn variety_construction_validates_profile_against_lifespan() {
        let crop_id = CropId::new();
        let pluri = VarietyProfile::Pluriannual(
            PluriannualProfile::new(None, None, 200, 280, Some(dec!(15))).unwrap(),
        );
        // Wrong: pluriannual profile on annual crop
        let res = Variety::new(crop_id, Lifespan::Annual, "Reine", None, pluri);
        assert!(res.is_err());

        // Right: pluriannual profile on perennial crop
        let v = Variety::new(
            crop_id,
            Lifespan::perennial(40, 3).unwrap(),
            "  Reine des Reinettes  ",
            Some("  pomme à couteau  ".to_owned()),
            pluri,
        )
        .unwrap();
        assert_eq!(v.name, "Reine des Reinettes");
        assert_eq!(v.description.as_deref(), Some("pomme à couteau"));
    }
}
