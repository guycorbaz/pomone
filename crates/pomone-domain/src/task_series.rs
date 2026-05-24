//! Recurring task series.
//!
//! A [`TaskSeries`] is the template behind a stream of `Task`s repeated on
//! a fixed interval (every N days / weeks / months). The series stores the
//! recurrence parameters and the shared metadata (planting, location, type,
//! method, implement, notes) ; each materialized occurrence is a regular
//! `Task` row whose `series_id` points back to it.
//!
//! Two operations live here:
//!
//! - `RecurrenceRule::next_after` — pure date arithmetic for advancing
//!   one step. Month arithmetic clamps to the last valid day (March 31 +
//!   1 month → April 30, not May 1) so monthly chores stay anchored to
//!   the start-of-month / end-of-month rhythm the user picked.
//! - `TaskSeries::occurrences_until` — enumerate all dates in `[first,
//!   horizon]` that the series should materialize. Pure, easily
//!   unit-tested.

use chrono::{Datelike, Duration, NaiveDate};
use serde::{Deserialize, Serialize};

use crate::ids::{LocationId, PlantingId, TaskImplementId, TaskMethodId, TaskSeriesId, TaskTypeId};
use crate::validation::normalize_optional;
use crate::{DomainError, DomainResult};

/// Step unit for [`RecurrenceRule`]. Mapped to a snake-case codec string
/// so the DB column stays human-readable and stable across schema dumps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecurrenceUnit {
    Days,
    Weeks,
    Months,
}

/// "Every N units" with an optional end date. The series stops generating
/// occurrences once it would land past `end_on` (inclusive — a series
/// ending on 2026-12-31 will produce that exact date if it falls on the
/// rhythm).
///
/// `interval` must be ≥ 1 ; zero would mean "every zero days", which would
/// infinite-loop a generator. The constructor enforces this.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecurrenceRule {
    pub unit: RecurrenceUnit,
    pub interval: u32,
    /// Inclusive end date. `None` = open-ended (caller decides the
    /// horizon — see [`TaskSeries::occurrences_until`]).
    pub end_on: Option<NaiveDate>,
}

impl RecurrenceRule {
    /// Build a rule, enforcing `interval ≥ 1`.
    pub fn new(
        unit: RecurrenceUnit,
        interval: u32,
        end_on: Option<NaiveDate>,
    ) -> DomainResult<Self> {
        if interval == 0 {
            // Same shape as the other "must be > 0" guards (e.g.
            // `Planting::plants_count`); the UI surfaces it via the
            // existing `error-positive-required` Fluent template.
            return Err(DomainError::NonPositiveCount(0));
        }
        Ok(Self {
            unit,
            interval,
            end_on,
        })
    }

    /// Advance one step from `date`. Months clamp to the last valid day
    /// of the target month (so March 31 + 1 month → April 30, not May 1).
    /// Returns `None` if the arithmetic overflows (chrono is generous on
    /// the date range, so practically never).
    #[must_use]
    pub fn next_after(&self, date: NaiveDate) -> Option<NaiveDate> {
        match self.unit {
            RecurrenceUnit::Days => {
                date.checked_add_signed(Duration::days(i64::from(self.interval)))
            }
            RecurrenceUnit::Weeks => {
                date.checked_add_signed(Duration::days(i64::from(self.interval) * 7))
            }
            RecurrenceUnit::Months => add_months_clamped(date, self.interval),
        }
    }
}

/// Add `months` calendar months to `date`, clamping the day to the last
/// valid day of the target month. Returns `None` only on year overflow.
fn add_months_clamped(date: NaiveDate, months: u32) -> Option<NaiveDate> {
    let total_month = i64::from(date.month()) + i64::from(months);
    let extra_years = (total_month - 1) / 12;
    let new_month = u32::try_from(((total_month - 1) % 12) + 1).ok()?;
    let new_year = i32::try_from(i64::from(date.year()) + extra_years).ok()?;
    // Try the same day; if the target month is shorter, walk back to the last valid day.
    for day in (1..=date.day()).rev() {
        if let Some(d) = NaiveDate::from_ymd_opt(new_year, new_month, day) {
            return Some(d);
        }
    }
    None
}

/// Template for a stream of `Task`s. Shape mirrors the relevant subset of
/// `Task` fields ; the per-occurrence `planned_on` / `completed_on` /
/// `duration_min` / `labor_hours` are owned by the materialized rows.
///
/// `planting_id` and `location_id` follow the same "permissive anchor"
/// rule as `Task` — either, both, or neither may be set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskSeries {
    pub id: TaskSeriesId,
    pub planting_id: Option<PlantingId>,
    pub location_id: Option<LocationId>,
    pub task_type_id: TaskTypeId,
    pub task_method_id: Option<TaskMethodId>,
    pub implement_id: Option<TaskImplementId>,
    pub rule: RecurrenceRule,
    /// Date of the first occurrence — same field semantics as `Task::planned_on`.
    pub first_planned_on: NaiveDate,
    pub notes: Option<String>,
}

impl TaskSeries {
    /// Build a new series with a fresh ID. Notes are trimmed and collapsed
    /// to `None` if blank (same convention as other entities).
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn new(
        planting_id: Option<PlantingId>,
        location_id: Option<LocationId>,
        task_type_id: TaskTypeId,
        task_method_id: Option<TaskMethodId>,
        implement_id: Option<TaskImplementId>,
        rule: RecurrenceRule,
        first_planned_on: NaiveDate,
        notes: Option<String>,
    ) -> Self {
        Self {
            id: TaskSeriesId::new(),
            planting_id,
            location_id,
            task_type_id,
            task_method_id,
            implement_id,
            rule,
            first_planned_on,
            notes: normalize_optional(notes),
        }
    }

    /// Enumerate every occurrence date in `[first_planned_on, horizon]`.
    ///
    /// `horizon` is the caller-supplied upper bound — typically
    /// `min(rule.end_on, today + 1 year)` so an open-ended series doesn't
    /// materialize a thousand rows up front. When `rule.end_on` is set,
    /// it caps `horizon` automatically (whichever is earlier wins).
    ///
    /// The function is pure (no DB) and idempotent — callers re-running it
    /// with a later horizon can dedupe against existing rows by date.
    #[must_use]
    pub fn occurrences_until(&self, horizon: NaiveDate) -> Vec<NaiveDate> {
        let stop = match self.rule.end_on {
            Some(end) if end < horizon => end,
            _ => horizon,
        };
        if stop < self.first_planned_on {
            return Vec::new();
        }
        let mut out = Vec::new();
        let mut cur = self.first_planned_on;
        while cur <= stop {
            out.push(cur);
            match self.rule.next_after(cur) {
                Some(next) if next > cur => cur = next, // strictly forward
                _ => break,
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    #[test]
    fn rule_rejects_zero_interval() {
        assert!(RecurrenceRule::new(RecurrenceUnit::Days, 0, None).is_err());
    }

    #[test]
    fn days_advances_by_n_days() {
        let r = RecurrenceRule::new(RecurrenceUnit::Days, 10, None).unwrap();
        assert_eq!(r.next_after(d(2026, 5, 1)), Some(d(2026, 5, 11)));
    }

    #[test]
    fn weeks_advances_by_n_weeks() {
        let r = RecurrenceRule::new(RecurrenceUnit::Weeks, 2, None).unwrap();
        assert_eq!(r.next_after(d(2026, 5, 1)), Some(d(2026, 5, 15)));
    }

    #[test]
    fn months_clamp_to_last_valid_day() {
        let r = RecurrenceRule::new(RecurrenceUnit::Months, 1, None).unwrap();
        // March 31 + 1 month → April 30 (not May 1).
        assert_eq!(r.next_after(d(2026, 3, 31)), Some(d(2026, 4, 30)));
        // April 30 + 1 month → May 30 (day clamped back to 30, not 31).
        assert_eq!(r.next_after(d(2026, 4, 30)), Some(d(2026, 5, 30)));
        // Jan 31 + 1 month → Feb 28 (non-leap).
        assert_eq!(r.next_after(d(2027, 1, 31)), Some(d(2027, 2, 28)));
        // Jan 31 + 1 month → Feb 29 (leap).
        assert_eq!(r.next_after(d(2028, 1, 31)), Some(d(2028, 2, 29)));
    }

    #[test]
    fn months_can_span_year_boundary() {
        let r = RecurrenceRule::new(RecurrenceUnit::Months, 6, None).unwrap();
        assert_eq!(r.next_after(d(2026, 10, 15)), Some(d(2027, 4, 15)));
        let r2 = RecurrenceRule::new(RecurrenceUnit::Months, 24, None).unwrap();
        assert_eq!(r2.next_after(d(2026, 5, 1)), Some(d(2028, 5, 1)));
    }

    #[test]
    fn occurrences_until_horizon_inclusive() {
        let rule = RecurrenceRule::new(RecurrenceUnit::Weeks, 1, None).unwrap();
        let s = TaskSeries::new(
            None,
            None,
            TaskTypeId::new(),
            None,
            None,
            rule,
            d(2026, 5, 1),
            None,
        );
        let dates = s.occurrences_until(d(2026, 5, 22));
        assert_eq!(
            dates,
            vec![d(2026, 5, 1), d(2026, 5, 8), d(2026, 5, 15), d(2026, 5, 22)]
        );
    }

    #[test]
    fn occurrences_respect_end_on() {
        let rule = RecurrenceRule::new(RecurrenceUnit::Weeks, 1, Some(d(2026, 5, 14))).unwrap();
        let s = TaskSeries::new(
            None,
            None,
            TaskTypeId::new(),
            None,
            None,
            rule,
            d(2026, 5, 1),
            None,
        );
        // Horizon further than end_on → end_on wins.
        let dates = s.occurrences_until(d(2026, 12, 31));
        assert_eq!(dates, vec![d(2026, 5, 1), d(2026, 5, 8)]);
        // Note: 2026-05-15 > 2026-05-14, so it's excluded.
    }

    #[test]
    fn occurrences_empty_when_horizon_before_start() {
        let rule = RecurrenceRule::new(RecurrenceUnit::Days, 7, None).unwrap();
        let s = TaskSeries::new(
            None,
            None,
            TaskTypeId::new(),
            None,
            None,
            rule,
            d(2026, 5, 1),
            None,
        );
        assert!(s.occurrences_until(d(2026, 4, 30)).is_empty());
    }

    #[test]
    fn notes_are_normalized() {
        let rule = RecurrenceRule::new(RecurrenceUnit::Weeks, 1, None).unwrap();
        let s = TaskSeries::new(
            None,
            None,
            TaskTypeId::new(),
            None,
            None,
            rule,
            d(2026, 5, 1),
            Some("   ".into()),
        );
        assert!(s.notes.is_none());
    }
}
