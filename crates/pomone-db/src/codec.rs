//! Encoding/decoding of domain sum types (`Lifespan`, `VarietyProfile`,
//! `PlantingSchedule`, `PruningSeason`) to/from SQL columns.
//!
//! Each `decode_*` function returns `DbResult` because data may be malformed
//! if the database was tampered with externally; the schema's CHECK
//! constraints make that nearly impossible in practice but we still handle it.

use crate::error::{DbError, DbResult};
use chrono::NaiveDate;
use pomone_domain::{
    AnnualProfile, Lifespan, PlantingSchedule, PluriannualProfile, ProductivePattern,
    PruningSeason, VarietyProfile,
};
use rust_decimal::Decimal;

// ============================================================
// PruningSeason
// ============================================================

pub(crate) fn encode_pruning(season: PruningSeason) -> &'static str {
    match season {
        PruningSeason::Winter => "winter",
        PruningSeason::Summer => "summer",
        PruningSeason::Both => "both",
        PruningSeason::None => "none",
    }
}

pub(crate) fn decode_pruning(s: &str) -> DbResult<PruningSeason> {
    match s {
        "winter" => Ok(PruningSeason::Winter),
        "summer" => Ok(PruningSeason::Summer),
        "both" => Ok(PruningSeason::Both),
        "none" => Ok(PruningSeason::None),
        other => Err(DbError::Malformed(format!("pruning_season={other}"))),
    }
}

// ============================================================
// Lifespan
// ============================================================

#[derive(Debug, Default)]
pub(crate) struct LifespanColumns {
    pub kind: &'static str,
    pub lifespan_years: Option<i64>,
    pub pattern: Option<&'static str>,
    pub years_to_first_yield: Option<i64>,
}

pub(crate) fn encode_lifespan(l: Lifespan) -> LifespanColumns {
    match l {
        Lifespan::Annual => LifespanColumns {
            kind: "annual",
            ..Default::default()
        },
        Lifespan::Pluriannual {
            pattern: ProductivePattern::SingleCycle,
            lifespan_years,
        } => LifespanColumns {
            kind: "pluriannual",
            lifespan_years: Some(i64::from(lifespan_years)),
            pattern: Some("single_cycle"),
            years_to_first_yield: None,
        },
        Lifespan::Pluriannual {
            pattern:
                ProductivePattern::Recurring {
                    years_to_first_yield,
                },
            lifespan_years,
        } => LifespanColumns {
            kind: "pluriannual",
            lifespan_years: Some(i64::from(lifespan_years)),
            pattern: Some("recurring"),
            years_to_first_yield: Some(i64::from(years_to_first_yield)),
        },
    }
}

pub(crate) fn decode_lifespan(
    kind: &str,
    lifespan_years: Option<i64>,
    pattern: Option<&str>,
    years_to_first_yield: Option<i64>,
) -> DbResult<Lifespan> {
    match kind {
        "annual" => Ok(Lifespan::Annual),
        "pluriannual" => {
            let years = lifespan_years
                .ok_or_else(|| DbError::Malformed("pluriannual without lifespan_years".into()))?;
            let years_u8 = u8::try_from(years)
                .map_err(|_| DbError::Malformed(format!("lifespan_years out of range: {years}")))?;
            match pattern {
                Some("single_cycle") => {
                    Lifespan::pluriannual_single_cycle(years_u8).map_err(DbError::Domain)
                }
                Some("recurring") => {
                    let yfty = years_to_first_yield.ok_or_else(|| {
                        DbError::Malformed("recurring without years_to_first_yield".into())
                    })?;
                    let yfty_u8 = u8::try_from(yfty).map_err(|_| {
                        DbError::Malformed(format!("years_to_first_yield out of range: {yfty}"))
                    })?;
                    Lifespan::perennial(years_u8, yfty_u8).map_err(DbError::Domain)
                }
                other => Err(DbError::Malformed(format!(
                    "invalid productive_pattern: {other:?}"
                ))),
            }
        }
        other => Err(DbError::Malformed(format!("lifespan_kind={other}"))),
    }
}

// ============================================================
// VarietyProfile
// ============================================================

#[derive(Debug, Default)]
pub(crate) struct VarietyProfileColumns {
    pub kind: &'static str,
    pub days_to_transplant: Option<i64>,
    pub days_to_maturity: Option<i64>,
    pub harvest_window_days: Option<i64>,
    pub bud_break_doy: Option<i64>,
    pub flowering_doy: Option<i64>,
    pub harvest_start_doy: Option<i64>,
    pub harvest_end_doy: Option<i64>,
    pub expected_yield_kg_per_plant: Option<Decimal>,
}

pub(crate) fn encode_variety_profile(p: VarietyProfile) -> VarietyProfileColumns {
    match p {
        VarietyProfile::Annual(a) => VarietyProfileColumns {
            kind: "annual",
            days_to_transplant: a.days_to_transplant.map(i64::from),
            days_to_maturity: Some(i64::from(a.days_to_maturity)),
            harvest_window_days: Some(i64::from(a.harvest_window_days)),
            ..Default::default()
        },
        VarietyProfile::Pluriannual(p) => VarietyProfileColumns {
            kind: "pluriannual",
            bud_break_doy: p.bud_break_doy.map(i64::from),
            flowering_doy: p.flowering_doy.map(i64::from),
            harvest_start_doy: Some(i64::from(p.harvest_start_doy)),
            harvest_end_doy: Some(i64::from(p.harvest_end_doy)),
            expected_yield_kg_per_plant: p.expected_yield_kg_per_plant,
            ..Default::default()
        },
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn decode_variety_profile(
    kind: &str,
    days_to_transplant: Option<i64>,
    days_to_maturity: Option<i64>,
    harvest_window_days: Option<i64>,
    bud_break_doy: Option<i64>,
    flowering_doy: Option<i64>,
    harvest_start_doy: Option<i64>,
    harvest_end_doy: Option<i64>,
    expected_yield_kg_per_plant: Option<Decimal>,
) -> DbResult<VarietyProfile> {
    match kind {
        "annual" => {
            let dtm = days_to_maturity
                .ok_or_else(|| DbError::Malformed("annual without days_to_maturity".into()))?;
            let window = harvest_window_days
                .ok_or_else(|| DbError::Malformed("annual without harvest_window_days".into()))?;
            let dtt = days_to_transplant
                .map(|v| u16::try_from(v).map_err(|_| out_of_range("days_to_transplant", v)))
                .transpose()?;
            let dtm = u16::try_from(dtm).map_err(|_| out_of_range("days_to_maturity", dtm))?;
            let window =
                u16::try_from(window).map_err(|_| out_of_range("harvest_window_days", window))?;
            AnnualProfile::new(dtt, dtm, window)
                .map(VarietyProfile::Annual)
                .map_err(DbError::Domain)
        }
        "pluriannual" => {
            let bud = bud_break_doy
                .map(|v| u16::try_from(v).map_err(|_| out_of_range("bud_break_doy", v)))
                .transpose()?;
            let flw = flowering_doy
                .map(|v| u16::try_from(v).map_err(|_| out_of_range("flowering_doy", v)))
                .transpose()?;
            let hs = harvest_start_doy.ok_or_else(|| {
                DbError::Malformed("pluriannual without harvest_start_doy".into())
            })?;
            let he = harvest_end_doy
                .ok_or_else(|| DbError::Malformed("pluriannual without harvest_end_doy".into()))?;
            let hs = u16::try_from(hs).map_err(|_| out_of_range("harvest_start_doy", hs))?;
            let he = u16::try_from(he).map_err(|_| out_of_range("harvest_end_doy", he))?;
            PluriannualProfile::new(bud, flw, hs, he, expected_yield_kg_per_plant)
                .map(VarietyProfile::Pluriannual)
                .map_err(DbError::Domain)
        }
        other => Err(DbError::Malformed(format!("profile_kind={other}"))),
    }
}

// ============================================================
// PlantingSchedule
// ============================================================

#[derive(Debug, Default)]
pub(crate) struct PlantingScheduleColumns {
    pub kind: &'static str,
    pub sown_on: Option<NaiveDate>,
    pub transplanted_on: Option<NaiveDate>,
    pub first_harvest_on: Option<NaiveDate>,
    pub last_harvest_on: Option<NaiveDate>,
    pub established_on: Option<NaiveDate>,
    pub expected_removal_on: Option<NaiveDate>,
}

pub(crate) fn encode_planting_schedule(s: PlantingSchedule) -> PlantingScheduleColumns {
    match s {
        PlantingSchedule::Cycle {
            sown_on,
            transplanted_on,
            first_harvest_on,
            last_harvest_on,
        } => PlantingScheduleColumns {
            kind: "cycle",
            sown_on,
            transplanted_on,
            first_harvest_on: Some(first_harvest_on),
            last_harvest_on: Some(last_harvest_on),
            ..Default::default()
        },
        PlantingSchedule::Perennial {
            established_on,
            expected_removal_on,
        } => PlantingScheduleColumns {
            kind: "perennial",
            established_on: Some(established_on),
            expected_removal_on,
            ..Default::default()
        },
    }
}

/// String form of a `TaskCategory`, mirroring its `#[serde(rename_all = "snake_case")]`
/// Used to bind the value into the DB and to decode it back.
pub(crate) fn task_category_to_str(c: pomone_domain::TaskCategory) -> &'static str {
    match c {
        pomone_domain::TaskCategory::Sow => "sow",
        pomone_domain::TaskCategory::Transplant => "transplant",
        pomone_domain::TaskCategory::Harvest => "harvest",
        pomone_domain::TaskCategory::Weeding => "weeding",
        pomone_domain::TaskCategory::Irrigation => "irrigation",
        pomone_domain::TaskCategory::Treatment => "treatment",
        pomone_domain::TaskCategory::Tillage => "tillage",
        pomone_domain::TaskCategory::Other => "other",
    }
}

pub(crate) fn task_category_from_str(s: &str) -> DbResult<pomone_domain::TaskCategory> {
    Ok(match s {
        "sow" => pomone_domain::TaskCategory::Sow,
        "transplant" => pomone_domain::TaskCategory::Transplant,
        "harvest" => pomone_domain::TaskCategory::Harvest,
        "weeding" => pomone_domain::TaskCategory::Weeding,
        "irrigation" => pomone_domain::TaskCategory::Irrigation,
        "treatment" => pomone_domain::TaskCategory::Treatment,
        "tillage" => pomone_domain::TaskCategory::Tillage,
        "other" => pomone_domain::TaskCategory::Other,
        other => return Err(DbError::Malformed(format!("task category={other}"))),
    })
}

/// `RecurrenceUnit` ↔ DB string, mirroring `#[serde(rename_all = "snake_case")]`.
pub(crate) fn recurrence_unit_to_str(u: pomone_domain::RecurrenceUnit) -> &'static str {
    match u {
        pomone_domain::RecurrenceUnit::Days => "days",
        pomone_domain::RecurrenceUnit::Weeks => "weeks",
        pomone_domain::RecurrenceUnit::Months => "months",
    }
}

pub(crate) fn recurrence_unit_from_str(s: &str) -> DbResult<pomone_domain::RecurrenceUnit> {
    Ok(match s {
        "days" => pomone_domain::RecurrenceUnit::Days,
        "weeks" => pomone_domain::RecurrenceUnit::Weeks,
        "months" => pomone_domain::RecurrenceUnit::Months,
        other => return Err(DbError::Malformed(format!("recurrence unit={other}"))),
    })
}

pub(crate) fn decode_planting_schedule(
    kind: &str,
    sown_on: Option<NaiveDate>,
    transplanted_on: Option<NaiveDate>,
    first_harvest_on: Option<NaiveDate>,
    last_harvest_on: Option<NaiveDate>,
    established_on: Option<NaiveDate>,
    expected_removal_on: Option<NaiveDate>,
) -> DbResult<PlantingSchedule> {
    match kind {
        "cycle" => {
            let first = first_harvest_on
                .ok_or_else(|| DbError::Malformed("cycle without first_harvest_on".into()))?;
            let last = last_harvest_on
                .ok_or_else(|| DbError::Malformed("cycle without last_harvest_on".into()))?;
            PlantingSchedule::cycle(sown_on, transplanted_on, first, last).map_err(DbError::Domain)
        }
        "perennial" => {
            let established = established_on
                .ok_or_else(|| DbError::Malformed("perennial without established_on".into()))?;
            PlantingSchedule::perennial(established, expected_removal_on).map_err(DbError::Domain)
        }
        other => Err(DbError::Malformed(format!("schedule_kind={other}"))),
    }
}

// ============================================================
// Decimal <-> TEXT (sqlx-sqlite has no native Decimal support)
// ============================================================

pub(crate) fn decimal_to_text(d: Decimal) -> String {
    d.to_string()
}

pub(crate) fn decimal_from_text(s: &str) -> DbResult<Decimal> {
    use std::str::FromStr;
    Decimal::from_str(s).map_err(|e| DbError::Malformed(format!("invalid decimal {s:?}: {e}")))
}

pub(crate) fn opt_decimal_to_text(d: Option<Decimal>) -> Option<String> {
    d.map(decimal_to_text)
}

pub(crate) fn opt_decimal_from_text(s: Option<String>) -> DbResult<Option<Decimal>> {
    s.map(|s| decimal_from_text(&s)).transpose()
}

// ============================================================
// helpers
// ============================================================

fn out_of_range(field: &str, value: i64) -> DbError {
    DbError::Malformed(format!("{field} out of range: {value}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn pruning_roundtrip() {
        for s in [
            PruningSeason::Winter,
            PruningSeason::Summer,
            PruningSeason::Both,
            PruningSeason::None,
        ] {
            let encoded = encode_pruning(s);
            assert_eq!(decode_pruning(encoded).unwrap(), s);
        }
    }

    #[test]
    fn pruning_decode_invalid() {
        assert!(matches!(
            decode_pruning("autumn"),
            Err(DbError::Malformed(_))
        ));
    }

    #[test]
    fn lifespan_roundtrip_annual() {
        let l = Lifespan::Annual;
        let c = encode_lifespan(l);
        assert_eq!(c.kind, "annual");
        assert!(c.lifespan_years.is_none());
        assert!(c.pattern.is_none());
        assert!(c.years_to_first_yield.is_none());
        let back =
            decode_lifespan(c.kind, c.lifespan_years, c.pattern, c.years_to_first_yield).unwrap();
        assert_eq!(back, l);
    }

    #[test]
    fn lifespan_roundtrip_biennial() {
        let l = Lifespan::biennial().unwrap();
        let c = encode_lifespan(l);
        assert_eq!(c.kind, "pluriannual");
        assert_eq!(c.lifespan_years, Some(2));
        assert_eq!(c.pattern, Some("single_cycle"));
        assert!(c.years_to_first_yield.is_none());
        let back =
            decode_lifespan(c.kind, c.lifespan_years, c.pattern, c.years_to_first_yield).unwrap();
        assert_eq!(back, l);
    }

    #[test]
    fn lifespan_roundtrip_perennial() {
        let l = Lifespan::perennial(40, 3).unwrap();
        let c = encode_lifespan(l);
        assert_eq!(c.kind, "pluriannual");
        assert_eq!(c.lifespan_years, Some(40));
        assert_eq!(c.pattern, Some("recurring"));
        assert_eq!(c.years_to_first_yield, Some(3));
        let back =
            decode_lifespan(c.kind, c.lifespan_years, c.pattern, c.years_to_first_yield).unwrap();
        assert_eq!(back, l);
    }

    #[test]
    fn lifespan_decode_malformed() {
        // pluriannual but no years
        assert!(matches!(
            decode_lifespan("pluriannual", None, Some("single_cycle"), None),
            Err(DbError::Malformed(_))
        ));
        // recurring without yfty
        assert!(matches!(
            decode_lifespan("pluriannual", Some(10), Some("recurring"), None),
            Err(DbError::Malformed(_))
        ));
        // bogus kind
        assert!(matches!(
            decode_lifespan("perennial", None, None, None),
            Err(DbError::Malformed(_))
        ));
    }

    #[test]
    fn variety_profile_roundtrip_annual() {
        let p = VarietyProfile::Annual(AnnualProfile::new(Some(35), 70, 60).unwrap());
        let c = encode_variety_profile(p);
        assert_eq!(c.kind, "annual");
        assert_eq!(c.days_to_transplant, Some(35));
        let back = decode_variety_profile(
            c.kind,
            c.days_to_transplant,
            c.days_to_maturity,
            c.harvest_window_days,
            c.bud_break_doy,
            c.flowering_doy,
            c.harvest_start_doy,
            c.harvest_end_doy,
            c.expected_yield_kg_per_plant,
        )
        .unwrap();
        assert_eq!(back, p);
    }

    #[test]
    fn variety_profile_roundtrip_pluriannual() {
        let p = VarietyProfile::Pluriannual(
            PluriannualProfile::new(Some(80), Some(110), 220, 280, Some(dec!(15.5))).unwrap(),
        );
        let c = encode_variety_profile(p);
        assert_eq!(c.kind, "pluriannual");
        assert_eq!(c.harvest_start_doy, Some(220));
        let back = decode_variety_profile(
            c.kind,
            c.days_to_transplant,
            c.days_to_maturity,
            c.harvest_window_days,
            c.bud_break_doy,
            c.flowering_doy,
            c.harvest_start_doy,
            c.harvest_end_doy,
            c.expected_yield_kg_per_plant,
        )
        .unwrap();
        assert_eq!(back, p);
    }

    #[test]
    fn planting_schedule_roundtrip_cycle() {
        let s = PlantingSchedule::cycle(
            Some(NaiveDate::from_ymd_opt(2026, 3, 1).unwrap()),
            Some(NaiveDate::from_ymd_opt(2026, 5, 1).unwrap()),
            NaiveDate::from_ymd_opt(2026, 7, 1).unwrap(),
            NaiveDate::from_ymd_opt(2026, 10, 1).unwrap(),
        )
        .unwrap();
        let c = encode_planting_schedule(s);
        assert_eq!(c.kind, "cycle");
        let back = decode_planting_schedule(
            c.kind,
            c.sown_on,
            c.transplanted_on,
            c.first_harvest_on,
            c.last_harvest_on,
            c.established_on,
            c.expected_removal_on,
        )
        .unwrap();
        assert_eq!(back, s);
    }

    #[test]
    fn planting_schedule_roundtrip_perennial() {
        let s = PlantingSchedule::perennial(
            NaiveDate::from_ymd_opt(2026, 3, 15).unwrap(),
            Some(NaiveDate::from_ymd_opt(2056, 12, 31).unwrap()),
        )
        .unwrap();
        let c = encode_planting_schedule(s);
        assert_eq!(c.kind, "perennial");
        let back = decode_planting_schedule(
            c.kind,
            c.sown_on,
            c.transplanted_on,
            c.first_harvest_on,
            c.last_harvest_on,
            c.established_on,
            c.expected_removal_on,
        )
        .unwrap();
        assert_eq!(back, s);
    }
}
