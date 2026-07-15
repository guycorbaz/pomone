# Deferred work

Items surfaced during reviews that are real but not actionable in their originating story.

## Deferred from: code review of story-1.1 (2026-07-13)

- ✅ **RESOLVED in 1.3.** ~~Story 1.3 must tolerate pre-1.3 `occurred_at > recorded_at` rows.~~ The invariant is enforced only in `FieldEvent::new`; the DB decode path (`sqlite/mariadb::row_to_field_event`) builds the struct literally and never re-runs `new()`, so any historic inverted row loads unvalidated — the forward-only guard does not reject existing history. Tolerated by construction.

## Deferred from: code review of story-1.2 (2026-07-13)

- ✅ **RESOLVED in 1.3.** ~~reopen `corrects` linkage on same-day ties.~~ 1.3 makes `recorded_at` a caller-injected real `NaiveDateTime` (the UI passes `Local::now().naive_local()`), so settling events on the same day no longer tie at midnight; `ORDER BY recorded_at, id` resolves `latest_settling_event` correctly.

## Deferred from: Epic 0 retro AI-4 / story-1.7 (2026-07-15, decision by Guy)

- ⏸️ **Real `SIGKILL` mid-write crash injection** (a subprocess writing facts, killed mid-write, parent reopens → exact prefix). The paper-loop harness and `fact_invariants::prefix_replay_yields_prefix_state` model the crash as a clean drop+reopen. **Rationale for deferring:** SQLite runs `synchronous=FULL` + rollback journal here (no PRAGMA overrides), so every `record_fact` transaction is fsync-durable before it returns — drop+reopen faithfully models a crash *between* transactions. A true torn-write *inside* a transaction is SQLite's own atomicity domain, not our write path's. Revisit only if a backend without per-transaction durability (or a batched write path) is introduced.
