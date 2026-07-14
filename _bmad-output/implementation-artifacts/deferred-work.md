# Deferred work

Items surfaced during reviews that are real but not actionable in their originating story.

## Deferred from: code review of story-1.1 (2026-07-13)

- ✅ **RESOLVED in 1.3.** ~~Story 1.3 must tolerate pre-1.3 `occurred_at > recorded_at` rows.~~ The invariant is enforced only in `FieldEvent::new`; the DB decode path (`sqlite/mariadb::row_to_field_event`) builds the struct literally and never re-runs `new()`, so any historic inverted row loads unvalidated — the forward-only guard does not reject existing history. Tolerated by construction.

## Deferred from: code review of story-1.2 (2026-07-13)

- ✅ **RESOLVED in 1.3.** ~~reopen `corrects` linkage on same-day ties.~~ 1.3 makes `recorded_at` a caller-injected real `NaiveDateTime` (the UI passes `Local::now().naive_local()`), so settling events on the same day no longer tie at midnight; `ORDER BY recorded_at, id` resolves `latest_settling_event` correctly.
