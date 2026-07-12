//! Form validation, parsing, error rendering and error localization.
//! Extracted from `main.rs` (story 0.4); re-exported from the crate root so
//! `crate::…` paths keep working everywhere.

use anyhow::Result;
use chrono::{Local, NaiveDate};
use fluent::FluentArgs;
use pomone_app::AppError;
use rust_decimal::Decimal;
use slint::SharedString;
use std::str::FromStr;

pub(crate) fn today_iso() -> String {
    Local::now().date_naive().format("%Y-%m-%d").to_string()
}

/// Either a localized client-validation message or a service error that
/// still needs translation. Lets create handlers branch on prefix
/// ("Validation:" vs "Creation failed:") instead of mixing the two.
pub(crate) enum FormError {
    /// Already-localized text from a pre-submit validator.
    Validation(String),
    /// Service-level error; rendered via the existing `status-…-failed`
    /// template that prefixes "Échec :" / "Failed:".
    Service(AppError),
}

impl From<AppError> for FormError {
    fn from(e: AppError) -> Self {
        Self::Service(e)
    }
}

/// Trim and require a non-empty string. Returns the trimmed copy on success
/// or a localized "name required" message on failure.
pub(crate) fn validate_required_name(
    value: &str,
    i18n: &pomone_app::I18n,
) -> Result<String, FormError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        Err(FormError::Validation(i18n.t("error-name-required")))
    } else {
        Ok(trimmed.to_owned())
    }
}

/// Parse a `YYYY-MM-DD` date. Returns a localized "invalid date" message on
/// any parse failure (empty string included).
pub(crate) fn validate_iso_date(
    value: &str,
    i18n: &pomone_app::I18n,
) -> Result<NaiveDate, FormError> {
    NaiveDate::parse_from_str(value.trim(), "%Y-%m-%d")
        .map_err(|_| FormError::Validation(i18n.t("error-date-invalid")))
}

/// Parse a strictly-positive decimal. Empty or zero/negative input yields a
/// localized "positive required" message.
pub(crate) fn validate_positive_decimal(
    value: &str,
    i18n: &pomone_app::I18n,
) -> Result<Decimal, FormError> {
    let parsed = Decimal::from_str(value.trim())
        .map_err(|_| FormError::Validation(i18n.t("error-number-invalid")))?;
    if parsed <= Decimal::ZERO {
        return Err(FormError::Validation(i18n.t("error-positive-required")));
    }
    Ok(parsed)
}

/// Parse a strictly-positive `u32` count.
pub(crate) fn validate_positive_count(
    value: &str,
    i18n: &pomone_app::I18n,
) -> Result<u32, FormError> {
    let parsed = value
        .trim()
        .parse::<u32>()
        .map_err(|_| FormError::Validation(i18n.t("error-number-invalid")))?;
    if parsed == 0 {
        return Err(FormError::Validation(i18n.t("error-positive-required")));
    }
    Ok(parsed)
}

/// Parse a calendar year (required). Anything that doesn't fit `i32` or is
/// blank gets the localized "year required" message.
pub(crate) fn validate_year(value: &str, i18n: &pomone_app::I18n) -> Result<i32, FormError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(FormError::Validation(i18n.t("error-year-required")));
    }
    trimmed
        .parse::<i32>()
        .map_err(|_| FormError::Validation(i18n.t("error-year-required")))
}

/// Parse an optional decimal (empty → `None`). Errors are localized.
pub(crate) fn validate_optional_decimal(
    value: &str,
    i18n: &pomone_app::I18n,
) -> Result<Option<Decimal>, FormError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    Decimal::from_str(trimmed)
        .map(Some)
        .map_err(|_| FormError::Validation(i18n.t("error-number-invalid")))
}

/// Localize an [`AppError`] for display, mapping structured variants to the
/// `error-*` Fluent keys instead of leaking the English `Display` string
/// (issue #62). Unmapped internal errors fall back to `error-unexpected`,
/// which still carries the raw text so nothing is lost.
pub(crate) fn localize_app_error(i18n: &pomone_app::I18n, err: &AppError) -> String {
    match err {
        AppError::Domain(d) => localize_domain_error(i18n, d),
        AppError::Db(_) => i18n.t("error-database"),
        AppError::Config(message) => {
            let mut args = FluentArgs::new();
            args.set("message", message.clone());
            i18n.t_args("error-config", &args)
        }
        AppError::NotFound { kind, id } => {
            let mut args = FluentArgs::new();
            args.set("kind", (*kind).to_string());
            args.set("id", id.clone());
            i18n.t_args("error-not-found", &args)
        }
        AppError::MigrationTargetNotEmpty => i18n.t("settings-migrate-target-not-empty"),
        AppError::PlantingHasActivity => i18n.t("error-planting-has-activity"),
        AppError::Inconsistent(_)
        | AppError::Io(_)
        | AppError::TomlSerialize(_)
        | AppError::TomlDeserialize(_) => {
            let mut args = FluentArgs::new();
            args.set("message", err.to_string());
            i18n.t_args("error-unexpected", &args)
        }
    }
}

/// Localize a [`pomone_domain::DomainError`]: the common validation variants
/// map to dedicated keys; rarer structural mismatches use the generic
/// fallback (wrapping their `Display` text).
pub(crate) fn localize_domain_error(
    i18n: &pomone_app::I18n,
    err: &pomone_domain::DomainError,
) -> String {
    use pomone_domain::DomainError as D;
    match err {
        D::EmptyName => i18n.t("error-empty-name"),
        D::NonPositiveArea(v) => {
            let mut args = FluentArgs::new();
            args.set("value", v.to_string());
            i18n.t_args("error-non-positive-area", &args)
        }
        D::NonPositiveCount(_) => i18n.t("error-count-positive"),
        D::NonPositiveValue { .. } | D::NegativeValue { .. } | D::NonPositiveDaysToMaturity => {
            i18n.t("error-positive-required")
        }
        D::InvertedRange { .. } => i18n.t("error-height-range"),
        D::EmptyHarvestWindow => i18n.t("error-harvest-window"),
        D::DateBefore { .. } | D::DateAfter { .. } | D::DateOverflow => i18n.t("error-date-range"),
        _ => {
            let mut args = FluentArgs::new();
            args.set("message", err.to_string());
            i18n.t_args("error-unexpected", &args)
        }
    }
}

/// Push a `FormError` onto a status banner with the appropriate Fluent
/// template (validation errors get the no-prefix template; service errors
/// keep the legacy "Échec :" prefix, now with a localized detail).
pub(crate) fn render_form_error(i18n: &pomone_app::I18n, err: FormError) -> (SharedString, bool) {
    let msg = match err {
        FormError::Validation(text) => {
            let mut args = FluentArgs::new();
            args.set("message", text);
            i18n.t_args("status-validation-failed", &args)
        }
        FormError::Service(app_err) => {
            let mut args = FluentArgs::new();
            args.set("message", localize_app_error(i18n, &app_err));
            i18n.t_args("status-planting-failed", &args)
        }
    };
    (SharedString::from(msg), true)
}

/// Render a Families form error, special-casing the `family_in_use` sentinel.
pub(crate) fn render_family_form_error(
    i18n: &pomone_app::I18n,
    err: FormError,
) -> (SharedString, bool) {
    let msg = match err {
        FormError::Validation(text) => {
            let mut args = FluentArgs::new();
            args.set("message", text);
            i18n.t_args("status-validation-failed", &args)
        }
        FormError::Service(AppError::Inconsistent(ref code)) if code == "family_in_use" => {
            i18n.t("error-family-in-use")
        }
        FormError::Service(app_err) => {
            let mut args = FluentArgs::new();
            args.set("message", app_err.to_string());
            i18n.t_args("status-family-failed", &args)
        }
    };
    (SharedString::from(msg), true)
}

pub(crate) fn parse_u8(s: &str, field: &'static str) -> Result<u8, AppError> {
    s.trim()
        .parse::<u8>()
        .map_err(|e| AppError::Inconsistent(format!("invalid {field} '{s}': {e}")))
}

pub(crate) fn parse_u16(s: &str, field: &'static str) -> Result<u16, AppError> {
    s.trim()
        .parse::<u16>()
        .map_err(|e| AppError::Inconsistent(format!("invalid {field} '{s}': {e}")))
}

pub(crate) fn parse_optional_u16(s: &str, field: &'static str) -> Result<Option<u16>, AppError> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    parse_u16(trimmed, field).map(Some)
}

pub(crate) fn parse_optional_decimal(
    s: &str,
    field: &'static str,
) -> Result<Option<Decimal>, AppError> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    Decimal::from_str(trimmed)
        .map(Some)
        .map_err(|e| AppError::Inconsistent(format!("invalid {field} '{s}': {e}")))
}

pub(crate) fn parse_i32(s: &str, field: &'static str) -> Result<i32, AppError> {
    s.trim()
        .parse::<i32>()
        .map_err(|e| AppError::Inconsistent(format!("invalid {field} '{s}': {e}")))
}

pub(crate) fn optional_text(s: &str) -> Option<String> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_owned())
    }
}

/// Parse a `#RGB` or `#RRGGBB` string into a Slint `Color`. Invalid input
/// falls back to mid-grey so a malformed seed never crashes the UI.
pub(crate) fn parse_hex_color(s: &str) -> slint::Color {
    let hex = s.strip_prefix('#').unwrap_or(s);
    // Byte-slicing below assumes one byte per char; bail on any non-ASCII input
    // so a multi-byte UTF-8 char can't land us on a char boundary and panic.
    // (The domain validates colors on write, so this is purely defensive.)
    if !hex.is_ascii() {
        return slint::Color::from_rgb_u8(128, 128, 128);
    }
    let (r, g, b) = match hex.len() {
        3 => (
            u8::from_str_radix(&hex[0..1], 16).map(|v| v * 17),
            u8::from_str_radix(&hex[1..2], 16).map(|v| v * 17),
            u8::from_str_radix(&hex[2..3], 16).map(|v| v * 17),
        ),
        6 => (
            u8::from_str_radix(&hex[0..2], 16),
            u8::from_str_radix(&hex[2..4], 16),
            u8::from_str_radix(&hex[4..6], 16),
        ),
        _ => return slint::Color::from_rgb_u8(128, 128, 128),
    };
    match (r, g, b) {
        (Ok(r), Ok(g), Ok(b)) => slint::Color::from_rgb_u8(r, g, b),
        _ => slint::Color::from_rgb_u8(128, 128, 128),
    }
}

pub(crate) fn usize_to_i32(n: usize) -> i32 {
    i32::try_from(n).unwrap_or(i32::MAX)
}

/// Saturating cast of a signed `i32` index from Slint into a `usize`. A
/// negative value (Slint's "no current item") clamps to 0.
pub(crate) fn i32_to_usize(n: i32) -> usize {
    usize::try_from(n).unwrap_or(0)
}
