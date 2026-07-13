# Deferred work

Items surfaced during reviews that are real but not actionable in their originating story.

## Deferred from: code review of story-1.1 (2026-07-13)

- **Story 1.3 must tolerate pre-1.3 `occurred_at > recorded_at` rows.** The 1.1 journal is append-only, so any inverted (occurred_at, recorded_at) pair recorded before 1.3 introduces the `occurred_at ≤ recorded_at` constructor invariant becomes permanent history that the forward-only guard cannot retroactively reject. 1.3's guard should tolerate (or backfill-flag) pre-1.3 violators rather than assume journal-wide validity. [crates/pomone-domain/src/field_event.rs]
