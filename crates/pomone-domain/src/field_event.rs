//! The append-only field-event journal (story 1.1).
//!
//! A [`FieldEvent`] is a durable, idempotent record of something that happened
//! in the field — a task marked done, a task skipped with a reason, a planting
//! terminated. The journal is **append-only**: rows are never updated or
//! deleted. A *correction* is itself a new event whose [`FieldEvent::corrects`]
//! points at the event it amends, so history is preserved ("nothing is lost").
//!
//! The `id` is a client-generated UUIDv4 that doubles as an idempotency key:
//! re-recording the same event is a harmless no-op at the storage layer.
//!
//! The string form of [`FactKind`] and [`SkipReason`] lives in the `pomone-db`
//! codec (identical literals on both backends); the domain keeps the enums pure.

use crate::error::{DomainError, DomainResult};
use crate::ids::FieldEventId;
use crate::validation::require_name;
use chrono::{NaiveDate, NaiveDateTime, Timelike};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Max length of `target_kind`, matching the `VARCHAR(64)` MariaDB column so
/// both backends accept/reject the same values (SQLite `TEXT` is unbounded).
pub const MAX_TARGET_KIND_LEN: usize = 64;

/// What a field event asserts. Dot-namespaced by target (`task.done`,
/// `task.skipped`, `planting.terminated`). The set grows as later epics record
/// new gestures — there is no schema CHECK gating it (story 0.6 audit).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FactKind {
    /// A task was carried out.
    TaskDone,
    /// A task was deliberately not done (a [`SkipReason`] is recorded).
    TaskSkipped,
    /// A settled task (done or skipped) was explicitly reopened — a correction
    /// that clears its settled state. Carries `corrects` pointing at the event
    /// it reopens (story 1.2).
    TaskReopened,
    /// A planting's lifecycle was ended.
    PlantingTerminated,
}

/// Why a task was skipped — a closed set the grower picks from (story 1.5).
/// Extensible without a schema change (no CHECK); the UI wording is a separate
/// concern (Fluent), aligned to `docs/glossaire.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkipReason {
    /// Unworkable weather.
    Weather,
    /// Pest or disease pressure.
    PestDisease,
    /// The crop failed (poor emergence, loss…).
    CropFailure,
    /// No time this window.
    NoTime,
    /// Turned out unnecessary.
    NotNeeded,
    /// Replaced by another crop/task.
    Replaced,
    /// Anything else (a note usually accompanies it).
    Other,
}

impl SkipReason {
    /// The canonical string literal — the single source of truth shared by the
    /// DB codec (both backends) and the event payload.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            SkipReason::Weather => "weather",
            SkipReason::PestDisease => "pest-disease",
            SkipReason::CropFailure => "crop-failure",
            SkipReason::NoTime => "no-time",
            SkipReason::NotNeeded => "not-needed",
            SkipReason::Replaced => "replaced",
            SkipReason::Other => "other",
        }
    }

    /// Parse a canonical literal back into a `SkipReason`.
    #[must_use]
    pub fn from_literal(s: &str) -> Option<Self> {
        Some(match s {
            "weather" => SkipReason::Weather,
            "pest-disease" => SkipReason::PestDisease,
            "crop-failure" => SkipReason::CropFailure,
            "no-time" => SkipReason::NoTime,
            "not-needed" => SkipReason::NotNeeded,
            "replaced" => SkipReason::Replaced,
            "other" => SkipReason::Other,
            _ => return None,
        })
    }
}

/// Build the JSON payload for a `task.skipped` fact — the durable record of the
/// reason (+ optional note) that the projection also mirrors into task columns.
#[must_use]
pub fn skip_payload(reason: SkipReason, note: Option<&str>) -> String {
    match note {
        Some(note) => serde_json::json!({ "reason": reason.as_str(), "note": note }).to_string(),
        None => serde_json::json!({ "reason": reason.as_str() }).to_string(),
    }
}

/// One append-only journal entry. Built through [`FieldEvent::new`], which
/// mints the id and normalises the payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldEvent {
    pub id: FieldEventId,
    pub kind: FactKind,
    /// The kind of entity the event is about (`"task"`, `"planting"`…). Free
    /// but validated non-empty; not an enum so targets can grow across epics.
    pub target_kind: String,
    /// The referenced entity's id. Deliberately untyped and non-FK: the journal
    /// must outlive a deleted target.
    pub target_id: Uuid,
    /// Agronomic date the event is *about* (backdatable).
    pub occurred_at: NaiveDate,
    /// Instant the event was recorded (caller-injected — no clock below the UI).
    /// Distinct from `occurred_at`; orders the journal.
    pub recorded_at: NaiveDateTime,
    /// Event-specific data as a JSON string (`"{}"` when empty).
    pub payload: String,
    /// The event this one corrects, if any. A correction never mutates its
    /// target — it is a new entry pointing back at it.
    pub corrects: Option<FieldEventId>,
}

impl FieldEvent {
    /// Build a new event with a fresh id. `target_kind` must be non-empty; an
    /// empty/blank `payload` is normalised to `"{}"`.
    ///
    /// Note: the `occurred_at ≤ recorded_at` temporal invariant is introduced
    /// in story 1.3; this constructor does not yet enforce it.
    pub fn new(
        kind: FactKind,
        target_kind: impl Into<String>,
        target_id: Uuid,
        occurred_at: NaiveDate,
        recorded_at: NaiveDateTime,
        payload: impl Into<String>,
        corrects: Option<FieldEventId>,
    ) -> DomainResult<Self> {
        let target_kind = require_name(target_kind)?;
        if target_kind.len() > MAX_TARGET_KIND_LEN {
            return Err(DomainError::TooLong {
                field: "target_kind",
                max: MAX_TARGET_KIND_LEN,
                len: target_kind.len(),
            });
        }
        // Truncate to microseconds so the value round-trips identically on both
        // backends (SQLite stores ISO TEXT with nanoseconds; MariaDB
        // `DATETIME(6)` truncates to µs — otherwise the two would disagree).
        let recorded_at = recorded_at
            .with_nanosecond(recorded_at.nanosecond() / 1_000 * 1_000)
            .unwrap_or(recorded_at);
        let payload: String = payload.into();
        let payload = payload.trim();
        let payload = if payload.is_empty() {
            "{}".to_owned()
        } else {
            payload.to_owned()
        };
        Ok(Self {
            id: FieldEventId::new(),
            kind,
            target_kind,
            target_id,
            occurred_at,
            recorded_at,
            payload,
            corrects,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::DomainError;

    fn dt() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 3, 2)
            .unwrap()
            .and_hms_opt(9, 30, 0)
            .unwrap()
    }

    #[test]
    fn new_builds_a_valid_event() {
        let target = Uuid::new_v4();
        let e = FieldEvent::new(
            FactKind::TaskDone,
            "  task ",
            target,
            NaiveDate::from_ymd_opt(2026, 3, 2).unwrap(),
            dt(),
            "  {\"labor_h\":1.5}  ",
            None,
        )
        .unwrap();
        assert_eq!(e.kind, FactKind::TaskDone);
        assert_eq!(e.target_kind, "task"); // trimmed
        assert_eq!(e.target_id, target);
        assert_eq!(e.payload, "{\"labor_h\":1.5}");
        assert!(e.corrects.is_none());
    }

    #[test]
    fn blank_payload_becomes_empty_json_object() {
        let e = FieldEvent::new(
            FactKind::TaskSkipped,
            "task",
            Uuid::new_v4(),
            NaiveDate::from_ymd_opt(2026, 3, 2).unwrap(),
            dt(),
            "   ",
            None,
        )
        .unwrap();
        assert_eq!(e.payload, "{}");
    }

    #[test]
    fn overlong_target_kind_is_rejected() {
        let long = "x".repeat(MAX_TARGET_KIND_LEN + 1);
        let res = FieldEvent::new(
            FactKind::TaskDone,
            long,
            Uuid::new_v4(),
            NaiveDate::from_ymd_opt(2026, 3, 2).unwrap(),
            dt(),
            "{}",
            None,
        );
        assert!(matches!(
            res,
            Err(DomainError::TooLong {
                field: "target_kind",
                ..
            })
        ));
        // The boundary length itself is accepted.
        assert!(FieldEvent::new(
            FactKind::TaskDone,
            "y".repeat(MAX_TARGET_KIND_LEN),
            Uuid::new_v4(),
            NaiveDate::from_ymd_opt(2026, 3, 2).unwrap(),
            dt(),
            "{}",
            None,
        )
        .is_ok());
    }

    #[test]
    fn recorded_at_is_truncated_to_microseconds() {
        let with_nanos = NaiveDate::from_ymd_opt(2026, 3, 2)
            .unwrap()
            .and_hms_nano_opt(9, 30, 0, 123_456_789)
            .unwrap();
        let e = FieldEvent::new(
            FactKind::TaskDone,
            "task",
            Uuid::new_v4(),
            NaiveDate::from_ymd_opt(2026, 3, 2).unwrap(),
            with_nanos,
            "{}",
            None,
        )
        .unwrap();
        // 123_456_789 ns → 123_456_000 ns (microsecond precision).
        assert_eq!(e.recorded_at.nanosecond(), 123_456_000);
    }

    #[test]
    fn empty_target_kind_is_rejected() {
        let res = FieldEvent::new(
            FactKind::TaskDone,
            "  ",
            Uuid::new_v4(),
            NaiveDate::from_ymd_opt(2026, 3, 2).unwrap(),
            dt(),
            "{}",
            None,
        );
        assert_eq!(res, Err(DomainError::EmptyName));
    }

    #[test]
    fn each_event_gets_its_own_id() {
        let mk = || {
            FieldEvent::new(
                FactKind::TaskDone,
                "task",
                Uuid::new_v4(),
                NaiveDate::from_ymd_opt(2026, 3, 2).unwrap(),
                dt(),
                "{}",
                None,
            )
            .unwrap()
        };
        assert_ne!(mk().id, mk().id);
    }

    #[test]
    fn skip_reason_literals_round_trip() {
        for r in [
            SkipReason::Weather,
            SkipReason::PestDisease,
            SkipReason::CropFailure,
            SkipReason::NoTime,
            SkipReason::NotNeeded,
            SkipReason::Replaced,
            SkipReason::Other,
        ] {
            assert_eq!(SkipReason::from_literal(r.as_str()), Some(r));
        }
        assert_eq!(SkipReason::from_literal("nope"), None);
    }

    #[test]
    fn skip_payload_carries_reason_and_optional_note() {
        assert_eq!(
            skip_payload(SkipReason::Weather, None),
            r#"{"reason":"weather"}"#
        );
        let with_note = skip_payload(SkipReason::PestDisease, Some("puceron"));
        // Order of keys is stable (serde_json preserves insertion order here).
        assert!(with_note.contains(r#""reason":"pest-disease""#));
        assert!(with_note.contains(r#""note":"puceron""#));
        // A note with a quote is properly JSON-escaped.
        let tricky = skip_payload(SkipReason::Other, Some("a \"b\""));
        assert!(tricky.contains(r#""note":"a \"b\"""#));
    }

    #[test]
    fn a_correction_points_at_the_corrected_event() {
        let original = FieldEventId::new();
        let e = FieldEvent::new(
            FactKind::TaskDone,
            "task",
            Uuid::new_v4(),
            NaiveDate::from_ymd_opt(2026, 3, 2).unwrap(),
            dt(),
            "{}",
            Some(original),
        )
        .unwrap();
        assert_eq!(e.corrects, Some(original));
    }
}
