//! Shared validation helpers used by entity constructors.

use crate::error::{DomainError, DomainResult};
use rust_decimal::Decimal;

/// Trim a name and reject it if empty (or whitespace-only).
pub(crate) fn require_name(raw: impl Into<String>) -> DomainResult<String> {
    let s: String = raw.into();
    let trimmed = s.trim();
    if trimmed.is_empty() {
        Err(DomainError::EmptyName)
    } else {
        Ok(trimmed.to_owned())
    }
}

/// Trim an optional free-text field; map empty strings to `None`.
pub(crate) fn normalize_optional(raw: Option<String>) -> Option<String> {
    raw.and_then(|s| {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_owned())
        }
    })
}

/// Reject zero or negative areas.
pub(crate) fn require_positive_area(area: Decimal) -> DomainResult<Decimal> {
    if area > Decimal::ZERO {
        Ok(area)
    } else {
        Err(DomainError::NonPositiveArea(area))
    }
}

/// Reject zero or negative dimensions (length or width in meters). Returns a
/// structured error tagged with the field name so the UI can localize.
pub(crate) fn require_positive_dimension(
    value: Decimal,
    field: &'static str,
) -> DomainResult<Decimal> {
    if value > Decimal::ZERO {
        Ok(value)
    } else {
        Err(DomainError::NonPositiveValue { field, value })
    }
}

/// Reject zero counts.
pub(crate) fn require_positive_count(count: u32) -> DomainResult<u32> {
    if count > 0 {
        Ok(count)
    } else {
        Err(DomainError::NonPositiveCount(count))
    }
}

/// Reject day-of-year values outside 1..=366.
pub(crate) fn require_valid_doy(doy: u16) -> DomainResult<u16> {
    if (1..=366).contains(&doy) {
        Ok(doy)
    } else {
        Err(DomainError::InvalidDayOfYear(doy))
    }
}

/// Trim and validate a hex colour string (`#RGB` or `#RRGGBB`). Shared by the
/// entities that carry a user-picked colour (task types, families).
pub(crate) fn require_hex_color(raw: impl Into<String>) -> DomainResult<String> {
    let raw: String = raw.into();
    let trimmed = raw.trim();
    let ok = trimmed.starts_with('#')
        && matches!(trimmed.len(), 4 | 7)
        && trimmed[1..].chars().all(|c| c.is_ascii_hexdigit());
    if ok {
        Ok(trimmed.to_owned())
    } else {
        Err(DomainError::InvalidHexColor(raw))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn require_name_trims_and_accepts_non_empty() {
        assert_eq!(require_name("  hello  ").unwrap(), "hello");
        assert_eq!(require_name("a").unwrap(), "a");
    }

    #[test]
    fn require_name_rejects_empty_or_whitespace() {
        assert_eq!(require_name(""), Err(DomainError::EmptyName));
        assert_eq!(require_name("   "), Err(DomainError::EmptyName));
        assert_eq!(require_name("\t\n"), Err(DomainError::EmptyName));
    }

    #[test]
    fn normalize_optional_collapses_blanks_to_none() {
        assert_eq!(normalize_optional(None), None);
        assert_eq!(normalize_optional(Some(String::new())), None);
        assert_eq!(normalize_optional(Some("   ".to_owned())), None);
        assert_eq!(
            normalize_optional(Some("  text  ".to_owned())),
            Some("text".to_owned())
        );
    }

    #[test]
    fn require_positive_area_accepts_positive() {
        assert_eq!(require_positive_area(dec!(1.5)).unwrap(), dec!(1.5));
        assert_eq!(require_positive_area(dec!(0.0001)).unwrap(), dec!(0.0001));
    }

    #[test]
    fn require_positive_area_rejects_zero_and_negative() {
        assert!(require_positive_area(dec!(0)).is_err());
        assert!(require_positive_area(dec!(-1)).is_err());
    }

    #[test]
    fn require_positive_count_zero_rejected() {
        assert!(require_positive_count(0).is_err());
        assert_eq!(require_positive_count(1).unwrap(), 1);
    }

    #[test]
    fn doy_bounds() {
        assert!(require_valid_doy(0).is_err());
        assert_eq!(require_valid_doy(1).unwrap(), 1);
        assert_eq!(require_valid_doy(366).unwrap(), 366);
        assert!(require_valid_doy(367).is_err());
    }
}
