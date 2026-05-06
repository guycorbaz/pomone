//! Pure date arithmetic for plantings.
//!
//! These functions replace the SQL triggers used by Qrop. Putting the logic
//! in Rust means it is:
//!   * Tested independently of any database
//!   * Identical across `SQLite` and `MariaDB` backends (no dialect duplication)
//!   * Property-tested for edge cases (year boundaries, leap years, overflow)

use crate::error::{DomainError, DomainResult};
use crate::variety::{AnnualProfile, PluriannualProfile};
use chrono::{Datelike, Days, NaiveDate};

/// Result of inferring a planting cycle from a sowing date and a variety profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InferredCycle {
    pub sown_on: NaiveDate,
    pub transplanted_on: Option<NaiveDate>,
    pub first_harvest_on: NaiveDate,
    pub last_harvest_on: NaiveDate,
}

/// Date range in a calendar year.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DateRange {
    pub start: NaiveDate,
    pub end: NaiveDate,
}

/// Add `days` calendar days to `date`. Returns `DateOverflow` on overflow.
pub fn add_days(date: NaiveDate, days: u16) -> DomainResult<NaiveDate> {
    date.checked_add_days(Days::new(u64::from(days)))
        .ok_or(DomainError::DateOverflow)
}

/// Build a date from `(year, day-of-year)`. Returns `InvalidDayOfYear` if
/// `doy` is out of range for that year (e.g. 366 in a non-leap year).
pub fn date_from_doy(year: i32, doy: u16) -> DomainResult<NaiveDate> {
    NaiveDate::from_yo_opt(year, u32::from(doy)).ok_or(DomainError::InvalidDayOfYear(doy))
}

/// Day-of-year (1..=366) for a date.
#[must_use]
pub fn doy_of(date: NaiveDate) -> u16 {
    // ordinal() is 1..=366
    u16::try_from(date.ordinal()).unwrap_or(1)
}

/// Compute the full cycle from a sowing date and an annual variety profile.
///
/// - If the profile specifies `days_to_transplant`, the transplant date is
///   sown + DTT.
/// - First harvest is from the transplant date if any, otherwise from sowing,
///   plus DTM.
/// - Last harvest is `first_harvest + (harvest_window - 1)` (so a window of 1
///   means a single-day harvest).
pub fn infer_cycle_from_sowing(
    sown_on: NaiveDate,
    profile: AnnualProfile,
) -> DomainResult<InferredCycle> {
    let transplanted_on = match profile.days_to_transplant {
        Some(dtt) => Some(add_days(sown_on, dtt)?),
        None => None,
    };
    let start = transplanted_on.unwrap_or(sown_on);
    let first_harvest_on = add_days(start, profile.days_to_maturity)?;
    // window=1 ⇒ same day; window=N ⇒ N-1 days later
    let last_harvest_on = add_days(first_harvest_on, profile.harvest_window_days - 1)?;
    Ok(InferredCycle {
        sown_on,
        transplanted_on,
        first_harvest_on,
        last_harvest_on,
    })
}

/// Compute the full cycle from a transplant date when no sowing record exists
/// (e.g. purchased seedlings).
pub fn infer_cycle_from_transplant(
    transplanted_on: NaiveDate,
    profile: AnnualProfile,
) -> DomainResult<InferredCycle> {
    let first_harvest_on = add_days(transplanted_on, profile.days_to_maturity)?;
    let last_harvest_on = add_days(first_harvest_on, profile.harvest_window_days - 1)?;
    Ok(InferredCycle {
        sown_on: transplanted_on, // placeholder; caller may discard via PlantingSchedule
        transplanted_on: Some(transplanted_on),
        first_harvest_on,
        last_harvest_on,
    })
}

/// Convert a pluriannual variety's harvest window (DOY range) into actual
/// dates for a given calendar year.
///
/// Wraparound: if `harvest_end_doy < harvest_start_doy`, the harvest spans
/// two calendar years (e.g. olive Oct..Feb). The returned `end` date is in
/// `year + 1`.
pub fn perennial_harvest_window_in_year(
    profile: &PluriannualProfile,
    year: i32,
) -> DomainResult<DateRange> {
    let start = date_from_doy(year, profile.harvest_start_doy)?;
    let end = if profile.harvest_end_doy >= profile.harvest_start_doy {
        date_from_doy(year, profile.harvest_end_doy)?
    } else {
        // wrap into next year
        date_from_doy(year + 1, profile.harvest_end_doy)?
    };
    Ok(DateRange { start, end })
}

/// True if `date` falls within the half-open year-window `[start, end]`,
/// taking wrap into account (start and end may be in different years for
/// wraparound harvests).
#[must_use]
pub fn date_in_range(date: NaiveDate, range: DateRange) -> bool {
    date >= range.start && date <= range.end
}

/// Years for which a perennial planting is expected to be productive (years
/// from establishment + `years_to_first_yield` up to the year before
/// `expected_removal_on`, inclusive of the removal year if it is set).
pub fn productive_years(
    established_on: NaiveDate,
    years_to_first_yield: u8,
    expected_removal_on: Option<NaiveDate>,
    fallback_lifespan_years: u8,
) -> Vec<i32> {
    let establish_year = established_on.year();
    let first_yield_year = establish_year + i32::from(years_to_first_yield);
    let last_year = match expected_removal_on {
        Some(d) => d.year(),
        None => establish_year + i32::from(fallback_lifespan_years).saturating_sub(1).max(0),
    };
    if first_yield_year > last_year {
        return Vec::new();
    }
    (first_yield_year..=last_year).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use rust_decimal_macros::dec;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    #[test]
    fn add_days_basic() {
        assert_eq!(add_days(d(2026, 3, 1), 30).unwrap(), d(2026, 3, 31));
        assert_eq!(add_days(d(2026, 12, 31), 1).unwrap(), d(2027, 1, 1));
    }

    #[test]
    fn add_days_handles_leap_year() {
        // 2028 is a leap year
        assert_eq!(add_days(d(2028, 2, 28), 1).unwrap(), d(2028, 2, 29));
        assert_eq!(add_days(d(2028, 2, 28), 2).unwrap(), d(2028, 3, 1));
        // 2027 is not
        assert_eq!(add_days(d(2027, 2, 28), 1).unwrap(), d(2027, 3, 1));
    }

    #[test]
    fn add_days_overflow_returns_error() {
        let near_max = NaiveDate::MAX;
        let res = add_days(near_max, u16::MAX);
        assert_eq!(res, Err(DomainError::DateOverflow));
    }

    #[test]
    fn date_from_doy_basic() {
        assert_eq!(date_from_doy(2026, 1).unwrap(), d(2026, 1, 1));
        // 2027 not leap → DOY 365 = Dec 31
        assert_eq!(date_from_doy(2027, 365).unwrap(), d(2027, 12, 31));
        // 2028 is leap → DOY 366 = Dec 31
        assert_eq!(date_from_doy(2028, 366).unwrap(), d(2028, 12, 31));
    }

    #[test]
    fn date_from_doy_366_in_non_leap_rejected() {
        let res = date_from_doy(2027, 366);
        assert_eq!(res, Err(DomainError::InvalidDayOfYear(366)));
    }

    #[test]
    fn doy_of_roundtrips() {
        let date = d(2026, 5, 15);
        let doy = doy_of(date);
        assert_eq!(date_from_doy(2026, doy).unwrap(), date);
    }

    #[test]
    fn infer_cycle_with_transplant_and_window() {
        // Tomato: sown March 1, DTT=35, DTM=70 (from transplant), window=60
        let profile = AnnualProfile::new(Some(35), 70, 60).unwrap();
        let cycle = infer_cycle_from_sowing(d(2026, 3, 1), profile).unwrap();
        assert_eq!(cycle.sown_on, d(2026, 3, 1));
        assert_eq!(cycle.transplanted_on, Some(d(2026, 4, 5)));
        assert_eq!(cycle.first_harvest_on, d(2026, 6, 14));
        // window of 60 ⇒ first..first+59 = 60 days inclusive
        assert_eq!(cycle.last_harvest_on, d(2026, 8, 12));
    }

    #[test]
    fn infer_cycle_direct_sown() {
        // Carrot: direct-sown, DTM=80 from sowing, single 30-day window
        let profile = AnnualProfile::new(None, 80, 30).unwrap();
        let cycle = infer_cycle_from_sowing(d(2026, 4, 1), profile).unwrap();
        assert!(cycle.transplanted_on.is_none());
        assert_eq!(cycle.first_harvest_on, d(2026, 6, 20));
    }

    #[test]
    fn infer_cycle_single_day_window() {
        let profile = AnnualProfile::new(None, 60, 1).unwrap();
        let cycle = infer_cycle_from_sowing(d(2026, 4, 1), profile).unwrap();
        assert_eq!(cycle.first_harvest_on, cycle.last_harvest_on);
    }

    #[test]
    fn infer_cycle_from_transplant_skips_sowing_chain() {
        let profile = AnnualProfile::new(Some(35), 70, 60).unwrap();
        let cycle = infer_cycle_from_transplant(d(2026, 5, 1), profile).unwrap();
        assert_eq!(cycle.transplanted_on, Some(d(2026, 5, 1)));
        assert_eq!(cycle.first_harvest_on, d(2026, 7, 10));
    }

    #[test]
    fn perennial_harvest_window_no_wrap() {
        // Apple harvest: DOY 220 (Aug 8) to DOY 280 (Oct 7)
        let profile = PluriannualProfile::new(None, None, 220, 280, None).unwrap();
        let range = perennial_harvest_window_in_year(&profile, 2030).unwrap();
        assert_eq!(range.start, d(2030, 8, 8));
        assert_eq!(range.end, d(2030, 10, 7));
    }

    #[test]
    fn perennial_harvest_window_with_wrap() {
        // Olive harvest: DOY 280 (Oct 7) to DOY 50 (Feb 19)
        let profile = PluriannualProfile::new(None, None, 280, 50, None).unwrap();
        let range = perennial_harvest_window_in_year(&profile, 2030).unwrap();
        assert_eq!(range.start, d(2030, 10, 7));
        assert_eq!(range.end, d(2031, 2, 19));
        assert!(range.end > range.start);
    }

    #[test]
    fn date_in_range_inclusive() {
        let r = DateRange {
            start: d(2030, 6, 1),
            end: d(2030, 6, 30),
        };
        assert!(date_in_range(d(2030, 6, 1), r));
        assert!(date_in_range(d(2030, 6, 30), r));
        assert!(date_in_range(d(2030, 6, 15), r));
        assert!(!date_in_range(d(2030, 5, 31), r));
        assert!(!date_in_range(d(2030, 7, 1), r));
    }

    #[test]
    fn productive_years_with_explicit_removal() {
        // Apple planted 2026, first yield year 3 (2029), removed 2056
        let years = productive_years(d(2026, 3, 15), 3, Some(d(2056, 12, 31)), 30);
        assert_eq!(years.first(), Some(&2029));
        assert_eq!(years.last(), Some(&2056));
        assert_eq!(years.len(), 28);
    }

    #[test]
    fn productive_years_uses_fallback_lifespan_when_no_removal() {
        // Raspberry planted 2026, first yield same year (0 wait), 12y lifespan
        let years = productive_years(d(2026, 4, 1), 0, None, 12);
        assert_eq!(years.first(), Some(&2026));
        assert_eq!(years.last(), Some(&2037));
        assert_eq!(years.len(), 12);
    }

    #[test]
    fn productive_years_empty_when_first_yield_after_removal() {
        // Slow-growing, removed before first yield
        let years = productive_years(d(2026, 3, 15), 5, Some(d(2027, 12, 31)), 30);
        assert!(years.is_empty());
    }

    // === Property-based tests ===

    proptest! {
        #[test]
        fn add_days_is_monotonic(
            year in 1900i32..2200,
            month in 1u32..=12,
            day in 1u32..=28,
            d1 in 0u16..=1000,
            d2 in 0u16..=1000,
        ) {
            let date = NaiveDate::from_ymd_opt(year, month, day).unwrap();
            let r1 = add_days(date, d1);
            let r2 = add_days(date, d2);
            if let (Ok(a), Ok(b)) = (r1, r2) {
                if d1 <= d2 {
                    prop_assert!(a <= b);
                } else {
                    prop_assert!(a >= b);
                }
            }
        }

        #[test]
        fn add_days_is_pure_translation(
            year in 1900i32..2200,
            month in 1u32..=12,
            day in 1u32..=28,
            n in 0u16..=10000,
        ) {
            let date = NaiveDate::from_ymd_opt(year, month, day).unwrap();
            if let Ok(later) = add_days(date, n) {
                let diff = later.signed_duration_since(date).num_days();
                prop_assert_eq!(diff, i64::from(n));
            }
        }

        #[test]
        fn doy_roundtrip_preserves_date(
            year in 1900i32..2200,
            month in 1u32..=12,
            day in 1u32..=28,
        ) {
            let date = NaiveDate::from_ymd_opt(year, month, day).unwrap();
            let doy = doy_of(date);
            prop_assert!((1..=366).contains(&doy));
            let roundtrip = date_from_doy(year, doy).unwrap();
            prop_assert_eq!(roundtrip, date);
        }

        #[test]
        fn infer_cycle_orders_dates_correctly(
            year in 1990i32..2200,
            month in 1u32..=12,
            day in 1u32..=28,
            dtt in 1u16..=200,
            dtm in 1u16..=400,
            window in 1u16..=200,
        ) {
            let sown = NaiveDate::from_ymd_opt(year, month, day).unwrap();
            let profile = AnnualProfile::new(Some(dtt), dtm, window).unwrap();
            if let Ok(cycle) = infer_cycle_from_sowing(sown, profile) {
                prop_assert_eq!(cycle.sown_on, sown);
                let transplant = cycle.transplanted_on.unwrap();
                prop_assert!(transplant > sown);
                prop_assert!(cycle.first_harvest_on > transplant);
                prop_assert!(cycle.last_harvest_on >= cycle.first_harvest_on);
                let window_days = cycle.last_harvest_on
                    .signed_duration_since(cycle.first_harvest_on)
                    .num_days();
                prop_assert_eq!(window_days, i64::from(window) - 1);
            }
        }

        #[test]
        fn perennial_window_end_after_or_equal_start(
            year in 1990i32..2200,
            start_doy in 1u16..=365,
            end_doy in 1u16..=365,
        ) {
            let profile = PluriannualProfile::new(None, None, start_doy, end_doy, None)
                .unwrap();
            let range = perennial_harvest_window_in_year(&profile, year).unwrap();
            prop_assert!(range.end >= range.start);
        }

        #[test]
        fn productive_years_count_matches_expected_when_removal_set(
            establish_year in 2000i32..2100,
            first_yield in 0u8..=10,
            extra_years in 0u8..=40,
        ) {
            let lifespan = u8::saturating_add(first_yield, extra_years);
            if lifespan < 2 || u8::saturating_add(first_yield, 1) > lifespan {
                // skip invalid combos
                return Ok(());
            }
            let establish = NaiveDate::from_ymd_opt(establish_year, 3, 15).unwrap();
            let removal_year = establish_year + i32::from(lifespan) - 1;
            let removal = NaiveDate::from_ymd_opt(removal_year, 12, 31).unwrap();
            let years = productive_years(establish, first_yield, Some(removal), lifespan);
            let expected_count = i32::from(lifespan) - i32::from(first_yield);
            prop_assert_eq!(years.len(), usize::try_from(expected_count).unwrap());
        }
    }

    // Sanity: rust_decimal usage is exercised elsewhere; just keep it imported
    // so a future regression with the dec! macro is caught here too.
    #[test]
    fn _decimal_macro_smoke() {
        let _ = dec!(1.0);
    }
}
