# Deferred work

Items surfaced during reviews that are real but not actionable in their originating story.

## Deferred from: code review of story-1.1 (2026-07-13)

- ✅ **RESOLVED in 1.3.** ~~Story 1.3 must tolerate pre-1.3 `occurred_at > recorded_at` rows.~~ The invariant is enforced only in `FieldEvent::new`; the DB decode path (`sqlite/mariadb::row_to_field_event`) builds the struct literally and never re-runs `new()`, so any historic inverted row loads unvalidated — the forward-only guard does not reject existing history. Tolerated by construction.

## Deferred from: code review of story-1.2 (2026-07-13)

- ✅ **RESOLVED in 1.3.** ~~reopen `corrects` linkage on same-day ties.~~ 1.3 makes `recorded_at` a caller-injected real `NaiveDateTime` (the UI passes `Local::now().naive_local()`), so settling events on the same day no longer tie at midnight; `ORDER BY recorded_at, id` resolves `latest_settling_event` correctly.

## Deferred from: Epic 0 retro AI-4 / story-1.7 (2026-07-15, decision by Guy)

- ⏸️ **Real `SIGKILL` mid-write crash injection** (a subprocess writing facts, killed mid-write, parent reopens → exact prefix). The paper-loop harness and `fact_invariants::prefix_replay_yields_prefix_state` model the crash as a clean drop+reopen. **Rationale for deferring:** SQLite runs `synchronous=FULL` + rollback journal here (no PRAGMA overrides), so every `record_fact` transaction is fsync-durable before it returns — drop+reopen faithfully models a crash *between* transactions. A true torn-write *inside* a transaction is SQLite's own atomicity domain, not our write path's. Revisit only if a backend without per-transaction durability (or a batched write path) is introduced.

## Deferred from: story-3.4 planning (2026-07-21, decision by Guy)

- ⏸️ **`planting.terminated` through the fact write path.** `FactKind::PlantingTerminated` already exists in `codec.rs` (both backends, identical literal) but has no emitter: `FactsRepo::record_fact` accepts only a `TaskProjection`, so recording a termination as a fact needs a new `PlantingProjection` variant — a db-trait change plus both backend impls plus the transaction, plus `cross_backend_tests`. **Decision: Epic 5**, where reversible corrections over the journal live (FR24/FR26); story 3.4 persists the termination as `planting.terminated_on` via `planting_update` only. **Rationale:** the alternative — appending a bare `field_event` next to a separate `planting_update` — is two non-atomic writes, exactly the split-brain the 1.2 single-write-path rule exists to forbid. Half-doing it would be worse than not doing it. When Epic 5 adds `PlantingProjection`, `terminated_on` becomes its projection target and the column stays as-is (no migration needed then).

## Deferred from: story-3.4 implementation (2026-07-21)

- ⏸️ **Pre-existing MariaDB failure: `cross_backend_tests::mariadb_backend::record_fact`.** The scenario asserts that re-recording the same event id returns `FactOutcome::AlreadyRecorded`; on MariaDB it returns `Recorded`, i.e. the **idempotency guard of `record_fact` does not hold on that backend** while it does on SQLite — a genuine behavioural divergence between the two impls (story 1.2's core promise: "re-applying is harmless"). **Not caused by story 3.4** — reproduced at `f554d1d`, the pre-story commit. It went unnoticed because the MariaDB legs are `#[ignore]`d and CI has no Docker, exactly the blind spot the 2.1 retro flagged. **Needs its own story/issue** (likely the `INSERT … ON DUPLICATE KEY UPDATE` vs `INSERT OR IGNORE` asymmetry in the MariaDB `record_fact`); it belongs with Epic 5's fact work, or sooner if the owner ever switches backend.
