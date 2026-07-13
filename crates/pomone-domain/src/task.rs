//! Concrete tasks (operations) carried out on a planting or a location.
//!
//! A `Task` is one work item — a sowing, a transplant, a weeding pass, a
//! harvest, etc. — anchored on a date and optionally tied to a planting
//! and/or a location. Tasks form the operational backbone of the season:
//! the calendar and the worktime tracking are both projections over them.
//!
//! Tasks are deliberately permissive on what they target:
//! - `planting_id` set, `location_id` unset → "do something on this planting"
//! - `location_id` set, `planting_id` unset → "do something on this bed"
//!   regardless of which planting (e.g. preparing a bed before any planting)
//! - both set → useful when a planting spans a single bed and the task
//!   distinguishes the two anchors
//! - both unset → general farm task (admin, maintenance, training…)

use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::field_event::SkipReason;
use crate::ids::{
    LocationId, PlantingId, TaskId, TaskImplementId, TaskMethodId, TaskSeriesId, TaskTypeId,
};

/// A single scheduled or completed operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Task {
    pub id: TaskId,
    /// Target planting, if the task is tied to one.
    pub planting_id: Option<PlantingId>,
    /// Target location, if the task is tied to one independently of a planting.
    pub location_id: Option<LocationId>,
    /// What kind of operation this is. Always set; types live in the
    /// [`TaskType`](crate::TaskType) lookup table.
    pub task_type_id: TaskTypeId,
    /// Method (manual vs mechanized, drill vs broadcast…) — optional.
    pub task_method_id: Option<TaskMethodId>,
    /// Implement / tool used — optional.
    pub implement_id: Option<TaskImplementId>,
    /// Date the task is scheduled for.
    pub planned_on: NaiveDate,
    /// Date the task was actually completed. `None` while it's still pending
    /// (or skipped); setting this is how the UI marks a task done.
    pub completed_on: Option<NaiveDate>,
    /// Estimated duration in minutes (for planning).
    pub duration_min: Option<u32>,
    /// Actual labor time in hours (for costing). Decimal to allow 0.25h etc.
    pub labor_hours: Option<Decimal>,
    /// Optional link to the [`TaskSeries`](crate::TaskSeries) that
    /// materialized this occurrence. `None` for one-shot tasks
    /// (manual creates + auto-gen at planting creation); `Some` for
    /// each row produced by a recurring template.
    pub series_id: Option<TaskSeriesId>,
    /// Free-form notes (max 4 KiB, enforced by [`Task::new`]).
    pub notes: Option<String>,
    /// Date the task was skipped, if it was. A skip projection written only by
    /// `facts::record_fact` (story 1.2); `None` for pending/done tasks.
    pub skipped_on: Option<NaiveDate>,
    /// Why the task was skipped (closed set). Set together with `skipped_on`.
    pub skip_reason: Option<SkipReason>,
    /// Optional free-text note attached to a skip.
    pub skip_note: Option<String>,
}

impl Task {
    /// Build a new task with a fresh ID.
    ///
    /// `completed_on` may legitimately precede `planned_on` (you can record
    /// finishing earlier than planned), so the two dates aren't compared.
    /// Notes aren't length-checked at the domain level — the DB column
    /// applies the only hard cap.
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn new(
        planting_id: Option<PlantingId>,
        location_id: Option<LocationId>,
        task_type_id: TaskTypeId,
        task_method_id: Option<TaskMethodId>,
        implement_id: Option<TaskImplementId>,
        planned_on: NaiveDate,
        completed_on: Option<NaiveDate>,
        duration_min: Option<u32>,
        labor_hours: Option<Decimal>,
        notes: Option<String>,
    ) -> Self {
        let notes = crate::validation::normalize_optional(notes);
        Self {
            id: TaskId::new(),
            planting_id,
            location_id,
            task_type_id,
            task_method_id,
            implement_id,
            series_id: None,
            planned_on,
            completed_on,
            duration_min,
            labor_hours,
            notes,
            skipped_on: None,
            skip_reason: None,
            skip_note: None,
        }
    }

    /// Like [`Task::new`] but tied to a recurring series. Used by the
    /// app-layer occurrence generator when materializing a
    /// [`TaskSeries`](crate::TaskSeries).
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn new_in_series(
        series_id: TaskSeriesId,
        planting_id: Option<PlantingId>,
        location_id: Option<LocationId>,
        task_type_id: TaskTypeId,
        task_method_id: Option<TaskMethodId>,
        implement_id: Option<TaskImplementId>,
        planned_on: NaiveDate,
        notes: Option<String>,
    ) -> Self {
        let notes = crate::validation::normalize_optional(notes);
        Self {
            id: TaskId::new(),
            planting_id,
            location_id,
            task_type_id,
            task_method_id,
            implement_id,
            series_id: Some(series_id),
            planned_on,
            completed_on: None,
            duration_min: None,
            labor_hours: None,
            notes,
            skipped_on: None,
            skip_reason: None,
            skip_note: None,
        }
    }

    /// Mark this task complete on `date`. Returns a new value (Task is immutable).
    #[must_use]
    pub fn mark_completed(self, date: NaiveDate) -> Self {
        Self {
            completed_on: Some(date),
            ..self
        }
    }

    /// Move this task to a new planned date. Returns a new value (Task is
    /// immutable). Used by the calendar's drag-and-drop reschedule. The
    /// `series_id` is preserved untouched: dragging moves a single
    /// materialized occurrence, it does not rewrite the recurring template.
    #[must_use]
    pub fn reschedule(self, planned_on: NaiveDate) -> Self {
        Self { planned_on, ..self }
    }

    /// True if the task has a `completed_on` date set.
    #[must_use]
    pub fn is_completed(&self) -> bool {
        self.completed_on.is_some()
    }

    /// True if the task is still pending and planned for a date in the past.
    pub fn is_overdue(&self, today: NaiveDate) -> bool {
        !self.is_completed() && self.planned_on < today
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    fn fresh_task() -> Task {
        Task::new(
            Some(PlantingId::new()),
            None,
            TaskTypeId::new(),
            None,
            None,
            d(2026, 5, 1),
            None,
            Some(30),
            None,
            None,
        )
    }

    #[test]
    fn task_new_assigns_fresh_id() {
        let a = fresh_task();
        let b = fresh_task();
        assert_ne!(a.id, b.id);
    }

    #[test]
    fn mark_completed_records_the_date() {
        let t = fresh_task();
        assert!(!t.is_completed());
        let done = t.mark_completed(d(2026, 5, 3));
        assert!(done.is_completed());
        assert_eq!(done.completed_on, Some(d(2026, 5, 3)));
    }

    #[test]
    fn reschedule_changes_only_the_planned_date() {
        let t = fresh_task();
        let id = t.id;
        let moved = t.reschedule(d(2026, 6, 20));
        assert_eq!(moved.planned_on, d(2026, 6, 20));
        // Identity and every other field carry over untouched.
        assert_eq!(moved.id, id);
        assert_eq!(moved.duration_min, Some(30));
        assert_eq!(moved.completed_on, None);
        assert_eq!(moved.series_id, None);
    }

    #[test]
    fn overdue_only_if_pending_and_planned_before_today() {
        let t = fresh_task();
        assert!(t.is_overdue(d(2026, 5, 15)));
        assert!(!t.is_overdue(d(2026, 4, 15))); // not yet
        assert!(!t.is_overdue(d(2026, 5, 1))); // due today, not overdue

        let done = t.mark_completed(d(2026, 5, 1));
        assert!(!done.is_overdue(d(2026, 6, 1)));
    }

    #[test]
    fn notes_are_normalized() {
        let t = Task::new(
            None,
            None,
            TaskTypeId::new(),
            None,
            None,
            d(2026, 5, 1),
            None,
            None,
            None,
            Some("   ".into()),
        );
        // Blank-only notes collapse to None — same convention as other entities.
        assert_eq!(t.notes, None);

        let t = Task::new(
            None,
            None,
            TaskTypeId::new(),
            None,
            None,
            d(2026, 5, 1),
            None,
            None,
            None,
            Some("  hello  ".into()),
        );
        assert_eq!(t.notes.as_deref(), Some("hello"));
    }
}
