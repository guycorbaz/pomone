//! Plantings (a concrete planting of a Variety in a Location).
//!
//! A `Planting` is scheduled either as a `Cycle` (one plantation = one harvest
//! cycle, used for annuals AND biennials kept for seed) or as a `Perennial`
//! (long-lived productive plant tracked by yearly harvests).

use crate::crop::Lifespan;
use crate::error::{DomainError, DomainResult};
use crate::ids::{LocationId, PlantingId, VarietyId};
use crate::validation::{
    normalize_optional, require_name, require_positive_area, require_positive_count,
};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// How a planting unfolds in time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PlantingSchedule {
    /// A single production cycle: annuals (3-12 months) and biennials kept
    /// for seed (~24 months). May span multiple calendar years.
    Cycle {
        sown_on: Option<NaiveDate>,
        transplanted_on: Option<NaiveDate>,
        first_harvest_on: NaiveDate,
        last_harvest_on: NaiveDate,
    },
    /// A long-lived planting that produces year after year. Yearly yields
    /// are recorded separately via `YearlyHarvest`.
    Perennial {
        established_on: NaiveDate,
        expected_removal_on: Option<NaiveDate>,
    },
}

impl PlantingSchedule {
    pub fn cycle(
        sown_on: Option<NaiveDate>,
        transplanted_on: Option<NaiveDate>,
        first_harvest_on: NaiveDate,
        last_harvest_on: NaiveDate,
    ) -> DomainResult<Self> {
        if let (Some(s), Some(t)) = (sown_on, transplanted_on) {
            if s > t {
                return Err(DomainError::DateAfter {
                    field: "sown_on",
                    date: s,
                    max: t,
                });
            }
        }
        if let Some(t) = transplanted_on {
            if t > first_harvest_on {
                return Err(DomainError::DateAfter {
                    field: "transplanted_on",
                    date: t,
                    max: first_harvest_on,
                });
            }
        }
        if let Some(s) = sown_on {
            if s > first_harvest_on {
                return Err(DomainError::DateAfter {
                    field: "sown_on",
                    date: s,
                    max: first_harvest_on,
                });
            }
        }
        if first_harvest_on > last_harvest_on {
            return Err(DomainError::DateAfter {
                field: "first_harvest_on",
                date: first_harvest_on,
                max: last_harvest_on,
            });
        }
        Ok(Self::Cycle {
            sown_on,
            transplanted_on,
            first_harvest_on,
            last_harvest_on,
        })
    }

    pub fn perennial(
        established_on: NaiveDate,
        expected_removal_on: Option<NaiveDate>,
    ) -> DomainResult<Self> {
        if let Some(r) = expected_removal_on {
            if established_on > r {
                return Err(DomainError::DateAfter {
                    field: "established_on",
                    date: established_on,
                    max: r,
                });
            }
        }
        Ok(Self::Perennial {
            established_on,
            expected_removal_on,
        })
    }

    /// Check this schedule is compatible with the parent crop's lifespan.
    pub fn check_compatible(&self, lifespan: Lifespan) -> DomainResult<()> {
        match (self, lifespan.is_recurring()) {
            (Self::Cycle { .. }, false) | (Self::Perennial { .. }, true) => Ok(()),
            (Self::Cycle { .. }, true) => Err(DomainError::ScheduleLifespanMismatch {
                schedule: "cycle",
                lifespan: "pluriannual_recurring",
            }),
            (Self::Perennial { .. }, false) => Err(DomainError::ScheduleLifespanMismatch {
                schedule: "perennial",
                lifespan: if lifespan.is_annual() {
                    "annual"
                } else {
                    "pluriannual_single_cycle"
                },
            }),
        }
    }

    /// First date relevant to this planting (sowing if known, transplanting,
    /// first harvest, or establishment).
    #[must_use]
    pub fn start_date(&self) -> NaiveDate {
        match self {
            Self::Cycle {
                sown_on: Some(d), ..
            }
            | Self::Cycle {
                transplanted_on: Some(d),
                ..
            } => *d,
            Self::Cycle {
                first_harvest_on, ..
            } => *first_harvest_on,
            Self::Perennial { established_on, .. } => *established_on,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Planting {
    pub id: PlantingId,
    pub variety_id: VarietyId,
    pub location_id: LocationId,
    pub area_m2: Decimal,
    pub plants_count: u32,
    pub schedule: PlantingSchedule,
    pub name: Option<String>,
    pub notes: Option<String>,
}

impl Planting {
    pub fn new(
        variety_id: VarietyId,
        location_id: LocationId,
        crop_lifespan: Lifespan,
        area_m2: Decimal,
        plants_count: u32,
        schedule: PlantingSchedule,
        name: Option<String>,
        notes: Option<String>,
    ) -> DomainResult<Self> {
        schedule.check_compatible(crop_lifespan)?;
        let name = match name {
            Some(s) => {
                let trimmed = s.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(require_name(trimmed)?)
                }
            }
            None => None,
        };
        Ok(Self {
            id: PlantingId::new(),
            variety_id,
            location_id,
            area_m2: require_positive_area(area_m2)?,
            plants_count: require_positive_count(plants_count)?,
            schedule,
            name,
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
    fn cycle_with_full_dates() {
        let s = PlantingSchedule::cycle(
            Some(d(2026, 3, 1)),
            Some(d(2026, 5, 1)),
            d(2026, 7, 1),
            d(2026, 10, 1),
        )
        .unwrap();
        match s {
            PlantingSchedule::Cycle {
                sown_on,
                transplanted_on,
                first_harvest_on,
                last_harvest_on,
            } => {
                assert_eq!(sown_on, Some(d(2026, 3, 1)));
                assert_eq!(transplanted_on, Some(d(2026, 5, 1)));
                assert_eq!(first_harvest_on, d(2026, 7, 1));
                assert_eq!(last_harvest_on, d(2026, 10, 1));
            }
            PlantingSchedule::Perennial { .. } => panic!("wrong variant"),
        }
    }

    #[test]
    fn cycle_direct_sow_no_transplant() {
        // Carrots: direct sown, no transplant
        let s = PlantingSchedule::cycle(Some(d(2026, 4, 1)), None, d(2026, 7, 1), d(2026, 8, 15))
            .unwrap();
        if let PlantingSchedule::Cycle {
            transplanted_on, ..
        } = s
        {
            assert!(transplanted_on.is_none());
        }
    }

    #[test]
    fn cycle_purchased_plugs_no_sow_date() {
        // Bought seedlings, no sowing record
        let s = PlantingSchedule::cycle(None, Some(d(2026, 5, 1)), d(2026, 7, 1), d(2026, 10, 1))
            .unwrap();
        if let PlantingSchedule::Cycle { sown_on, .. } = s {
            assert!(sown_on.is_none());
        }
    }

    #[test]
    fn cycle_long_biennial_seed_cycle() {
        // Carrot for seed: sown March year 1, seed harvested July year 2
        let s = PlantingSchedule::cycle(Some(d(2026, 3, 15)), None, d(2027, 7, 1), d(2027, 7, 31))
            .unwrap();
        assert!(matches!(s, PlantingSchedule::Cycle { .. }));
    }

    #[test]
    fn cycle_rejects_sow_after_transplant() {
        let res = PlantingSchedule::cycle(
            Some(d(2026, 6, 1)),
            Some(d(2026, 5, 1)),
            d(2026, 8, 1),
            d(2026, 9, 1),
        );
        assert!(matches!(
            res,
            Err(DomainError::DateAfter {
                field: "sown_on",
                ..
            })
        ));
    }

    #[test]
    fn cycle_rejects_transplant_after_first_harvest() {
        let res = PlantingSchedule::cycle(None, Some(d(2026, 8, 1)), d(2026, 7, 1), d(2026, 9, 1));
        assert!(matches!(
            res,
            Err(DomainError::DateAfter {
                field: "transplanted_on",
                ..
            })
        ));
    }

    #[test]
    fn cycle_rejects_sow_after_first_harvest_when_no_transplant() {
        let res = PlantingSchedule::cycle(Some(d(2026, 8, 1)), None, d(2026, 7, 1), d(2026, 9, 1));
        assert!(matches!(
            res,
            Err(DomainError::DateAfter {
                field: "sown_on",
                ..
            })
        ));
    }

    #[test]
    fn cycle_rejects_first_harvest_after_last_harvest() {
        let res = PlantingSchedule::cycle(None, None, d(2026, 9, 1), d(2026, 8, 1));
        assert!(matches!(
            res,
            Err(DomainError::DateAfter {
                field: "first_harvest_on",
                ..
            })
        ));
    }

    #[test]
    fn cycle_accepts_equal_first_and_last_harvest_for_single_pick() {
        let s = PlantingSchedule::cycle(None, None, d(2026, 7, 1), d(2026, 7, 1));
        assert!(s.is_ok());
    }

    #[test]
    fn perennial_with_removal_date() {
        let s = PlantingSchedule::perennial(d(2026, 3, 15), Some(d(2056, 12, 31))).unwrap();
        assert!(matches!(s, PlantingSchedule::Perennial { .. }));
    }

    #[test]
    fn perennial_open_ended() {
        let s = PlantingSchedule::perennial(d(2026, 3, 15), None).unwrap();
        if let PlantingSchedule::Perennial {
            expected_removal_on,
            ..
        } = s
        {
            assert!(expected_removal_on.is_none());
        }
    }

    #[test]
    fn perennial_rejects_removal_before_establishment() {
        let res = PlantingSchedule::perennial(d(2026, 3, 15), Some(d(2025, 3, 15)));
        assert!(matches!(res, Err(DomainError::DateAfter { .. })));
    }

    #[test]
    fn cycle_compatible_with_annual_or_single_cycle_lifespan() {
        let s = PlantingSchedule::cycle(None, None, d(2026, 7, 1), d(2026, 8, 1)).unwrap();
        assert!(s.check_compatible(Lifespan::Annual).is_ok());
        assert!(s.check_compatible(Lifespan::biennial().unwrap()).is_ok());
        assert!(s
            .check_compatible(Lifespan::pluriannual_single_cycle(3).unwrap())
            .is_ok());
    }

    #[test]
    fn cycle_incompatible_with_recurring_lifespan() {
        let s = PlantingSchedule::cycle(None, None, d(2026, 7, 1), d(2026, 8, 1)).unwrap();
        let res = s.check_compatible(Lifespan::perennial(30, 3).unwrap());
        assert!(matches!(
            res,
            Err(DomainError::ScheduleLifespanMismatch {
                schedule: "cycle",
                ..
            })
        ));
    }

    #[test]
    fn perennial_compatible_only_with_recurring_lifespan() {
        let s = PlantingSchedule::perennial(d(2026, 3, 15), None).unwrap();
        assert!(s
            .check_compatible(Lifespan::perennial(30, 3).unwrap())
            .is_ok());
        assert!(matches!(
            s.check_compatible(Lifespan::Annual),
            Err(DomainError::ScheduleLifespanMismatch { .. })
        ));
        assert!(matches!(
            s.check_compatible(Lifespan::biennial().unwrap()),
            Err(DomainError::ScheduleLifespanMismatch { .. })
        ));
    }

    #[test]
    fn schedule_start_date_uses_earliest_known_event() {
        let cycle = PlantingSchedule::cycle(
            Some(d(2026, 3, 1)),
            Some(d(2026, 5, 1)),
            d(2026, 7, 1),
            d(2026, 10, 1),
        )
        .unwrap();
        assert_eq!(cycle.start_date(), d(2026, 3, 1));

        let cycle_no_sow =
            PlantingSchedule::cycle(None, Some(d(2026, 5, 1)), d(2026, 7, 1), d(2026, 10, 1))
                .unwrap();
        assert_eq!(cycle_no_sow.start_date(), d(2026, 5, 1));

        let cycle_just_harvest =
            PlantingSchedule::cycle(None, None, d(2026, 7, 1), d(2026, 10, 1)).unwrap();
        assert_eq!(cycle_just_harvest.start_date(), d(2026, 7, 1));

        let perennial = PlantingSchedule::perennial(d(2026, 3, 15), None).unwrap();
        assert_eq!(perennial.start_date(), d(2026, 3, 15));
    }

    #[test]
    fn planting_construction_validates_everything() {
        let schedule = PlantingSchedule::cycle(
            Some(d(2026, 3, 1)),
            Some(d(2026, 5, 1)),
            d(2026, 7, 1),
            d(2026, 10, 1),
        )
        .unwrap();
        let p = Planting::new(
            VarietyId::new(),
            LocationId::new(),
            Lifespan::Annual,
            dec!(20.0),
            100,
            schedule,
            Some("  Tomates Marmande  ".to_owned()),
            Some("  ".to_owned()),
        )
        .unwrap();
        assert_eq!(p.area_m2, dec!(20.0));
        assert_eq!(p.plants_count, 100);
        assert_eq!(p.name.as_deref(), Some("Tomates Marmande"));
        assert!(p.notes.is_none());
    }

    #[test]
    fn planting_rejects_zero_area_or_count() {
        let schedule = PlantingSchedule::cycle(None, None, d(2026, 7, 1), d(2026, 8, 1)).unwrap();

        let res = Planting::new(
            VarietyId::new(),
            LocationId::new(),
            Lifespan::Annual,
            dec!(0),
            10,
            schedule,
            None,
            None,
        );
        assert_eq!(res.unwrap_err(), DomainError::NonPositiveArea(dec!(0)));

        let res = Planting::new(
            VarietyId::new(),
            LocationId::new(),
            Lifespan::Annual,
            dec!(10),
            0,
            schedule,
            None,
            None,
        );
        assert_eq!(res.unwrap_err(), DomainError::NonPositiveCount(0));
    }

    #[test]
    fn planting_rejects_schedule_lifespan_mismatch() {
        let schedule = PlantingSchedule::perennial(d(2026, 3, 15), None).unwrap();
        let res = Planting::new(
            VarietyId::new(),
            LocationId::new(),
            Lifespan::Annual,
            dec!(10),
            5,
            schedule,
            None,
            None,
        );
        assert!(matches!(
            res,
            Err(DomainError::ScheduleLifespanMismatch { .. })
        ));
    }
}
