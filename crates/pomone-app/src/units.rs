//! Display units for areas and masses (issue #29 — field-crop support).
//!
//! The domain stores areas in **m²** and masses in **kg**, always. These
//! types only change how values are *displayed and entered*: a market
//! gardener thinks in m² and kg, a field-crop farm in ha and t. The chosen
//! units live in the app config (`area_unit` / `mass_unit`) and every
//! `*_view` formatter takes them as a parameter.

use rust_decimal::Decimal;

/// Display/input unit for areas. Storage stays m².
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AreaUnit {
    #[default]
    SquareMeters,
    Hectares,
}

impl AreaUnit {
    pub const ALL: [Self; 2] = [Self::SquareMeters, Self::Hectares];

    /// Stable config code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::SquareMeters => "m2",
            Self::Hectares => "ha",
        }
    }

    /// Unit suffix shown after values and in field labels.
    #[must_use]
    pub const fn suffix(self) -> &'static str {
        match self {
            Self::SquareMeters => "m²",
            Self::Hectares => "ha",
        }
    }

    /// Parse a persisted code, defaulting to m² for unknown values so a
    /// hand-edited config can't break startup.
    #[must_use]
    pub fn parse(code: &str) -> Self {
        Self::ALL
            .into_iter()
            .find(|u| u.code() == code)
            .unwrap_or_default()
    }

    /// Stored m² → display value in this unit.
    #[must_use]
    pub fn to_display(self, area_m2: Decimal) -> Decimal {
        match self {
            Self::SquareMeters => area_m2,
            Self::Hectares => area_m2 / Decimal::from(10_000),
        }
    }

    /// User input in this unit → stored m².
    #[must_use]
    pub fn to_m2(self, value: Decimal) -> Decimal {
        match self {
            Self::SquareMeters => value,
            Self::Hectares => value * Decimal::from(10_000),
        }
    }

    /// Format a stored m² value for display, e.g. `"250 m²"` / `"8.5 ha"`.
    #[must_use]
    pub fn format(self, area_m2: Decimal) -> String {
        format!("{} {}", self.to_display(area_m2).normalize(), self.suffix())
    }
}

/// Display/input unit for masses (yields). Storage stays kg.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MassUnit {
    #[default]
    Kilograms,
    Tonnes,
}

impl MassUnit {
    pub const ALL: [Self; 2] = [Self::Kilograms, Self::Tonnes];

    /// Stable config code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Kilograms => "kg",
            Self::Tonnes => "t",
        }
    }

    /// Unit suffix shown after values and in field labels.
    #[must_use]
    pub const fn suffix(self) -> &'static str {
        self.code()
    }

    /// Parse a persisted code, defaulting to kg for unknown values.
    #[must_use]
    pub fn parse(code: &str) -> Self {
        Self::ALL
            .into_iter()
            .find(|u| u.code() == code)
            .unwrap_or_default()
    }

    /// Stored kg → display value in this unit.
    #[must_use]
    pub fn to_display(self, mass_kg: Decimal) -> Decimal {
        match self {
            Self::Kilograms => mass_kg,
            Self::Tonnes => mass_kg / Decimal::from(1000),
        }
    }

    /// User input in this unit → stored kg.
    #[must_use]
    pub fn to_kg(self, value: Decimal) -> Decimal {
        match self {
            Self::Kilograms => value,
            Self::Tonnes => value * Decimal::from(1000),
        }
    }

    /// Format a stored kg value for display, e.g. `"120 kg"` / `"3.2 t"`.
    #[must_use]
    pub fn format(self, mass_kg: Decimal) -> String {
        format!("{} {}", self.to_display(mass_kg).normalize(), self.suffix())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn area_roundtrips_between_display_and_storage() {
        // 80 ha field-crop scenario (issue #29): no precision loss.
        let stored = AreaUnit::Hectares.to_m2(dec!(80));
        assert_eq!(stored, dec!(800000));
        assert_eq!(AreaUnit::Hectares.to_display(stored), dec!(80));
        assert_eq!(AreaUnit::SquareMeters.to_display(dec!(42.5)), dec!(42.5));
    }

    #[test]
    fn area_formats_with_suffix() {
        assert_eq!(AreaUnit::SquareMeters.format(dec!(250)), "250 m²");
        assert_eq!(AreaUnit::Hectares.format(dec!(800000)), "80 ha");
        assert_eq!(AreaUnit::Hectares.format(dec!(2500)), "0.25 ha");
    }

    #[test]
    fn mass_roundtrips_and_formats() {
        assert_eq!(MassUnit::Tonnes.to_kg(dec!(3.2)), dec!(3200));
        assert_eq!(MassUnit::Tonnes.format(dec!(3200)), "3.2 t");
        assert_eq!(MassUnit::Kilograms.format(dec!(15.5)), "15.5 kg");
    }

    #[test]
    fn parse_defaults_to_base_units_on_unknown_codes() {
        assert_eq!(AreaUnit::parse("ha"), AreaUnit::Hectares);
        assert_eq!(AreaUnit::parse("acres"), AreaUnit::SquareMeters);
        assert_eq!(AreaUnit::parse(""), AreaUnit::SquareMeters);
        assert_eq!(MassUnit::parse("t"), MassUnit::Tonnes);
        assert_eq!(MassUnit::parse("lbs"), MassUnit::Kilograms);
    }

    #[test]
    fn codes_roundtrip() {
        for u in AreaUnit::ALL {
            assert_eq!(AreaUnit::parse(u.code()), u);
        }
        for u in MassUnit::ALL {
            assert_eq!(MassUnit::parse(u.code()), u);
        }
    }
}
