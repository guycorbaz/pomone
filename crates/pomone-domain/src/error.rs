//! Domain validation errors.
//!
//! These are returned by constructor / `try_*` functions when an invariant
//! would be violated. They are deliberately structured (not stringly typed)
//! so the UI layer can surface localized messages.

use chrono::NaiveDate;
use rust_decimal::Decimal;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DomainError {
    #[error("name cannot be empty or whitespace-only")]
    EmptyName,

    #[error("area must be strictly positive (got {0})")]
    NonPositiveArea(Decimal),

    #[error("count must be strictly positive (got {0})")]
    NonPositiveCount(u32),

    #[error("day-of-year must be in 1..=366 (got {0})")]
    InvalidDayOfYear(u16),

    #[error("date {date} cannot be before {min} ({field})")]
    DateBefore {
        field: &'static str,
        date: NaiveDate,
        min: NaiveDate,
    },

    #[error("date {date} cannot be after {max} ({field})")]
    DateAfter {
        field: &'static str,
        date: NaiveDate,
        max: NaiveDate,
    },

    #[error("date arithmetic overflow")]
    DateOverflow,

    #[error("pluriannual lifespan must be at least 2 years (got {0})")]
    PluriannualLifespanTooShort(u8),

    #[error("years_to_first_yield ({first_yield}) must be strictly less than lifespan_years ({lifespan})")]
    YearsToFirstYieldTooHigh { first_yield: u8, lifespan: u8 },

    #[error("planting schedule kind '{schedule}' is incompatible with crop lifespan '{lifespan}'")]
    ScheduleLifespanMismatch {
        schedule: &'static str,
        lifespan: &'static str,
    },

    #[error("variety profile kind '{profile}' does not match crop lifespan '{lifespan}'")]
    VarietyProfileMismatch {
        profile: &'static str,
        lifespan: &'static str,
    },

    #[error("a planting cycle must span a non-empty harvest window")]
    EmptyHarvestWindow,

    #[error("days_to_maturity must be strictly positive")]
    NonPositiveDaysToMaturity,

    #[error("range minimum {min} cannot exceed maximum {max} ({field})")]
    InvertedRange {
        field: &'static str,
        min: String,
        max: String,
    },

    #[error("value for {field} must be strictly positive (got {value})")]
    NonPositiveValue { field: &'static str, value: Decimal },

    #[error("value for {field} cannot be negative (got {value})")]
    NegativeValue { field: &'static str, value: Decimal },

    #[error("invalid hex color '{0}' — expected #RGB or #RRGGBB")]
    InvalidHexColor(String),

    #[error("value for {field} is too long: {len} > {max} characters")]
    TooLong {
        field: &'static str,
        max: usize,
        len: usize,
    },
}

/// Convenience type for domain results.
pub type DomainResult<T> = Result<T, DomainError>;

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn errors_format_with_useful_messages() {
        let err = DomainError::DateBefore {
            field: "first_harvest_on",
            date: NaiveDate::from_ymd_opt(2026, 5, 1).unwrap(),
            min: NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(),
        };
        let s = err.to_string();
        assert!(s.contains("2026-05-01"));
        assert!(s.contains("2026-06-01"));
        assert!(s.contains("first_harvest_on"));
    }

    #[test]
    fn errors_are_equatable_for_test_assertions() {
        let a = DomainError::EmptyName;
        let b = DomainError::EmptyName;
        assert_eq!(a, b);

        let c = DomainError::NonPositiveCount(0);
        assert_ne!(a, c);
    }
}
