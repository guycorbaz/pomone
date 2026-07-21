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

- ✅ **TRACKED as [#153](https://github.com/guycorbaz/pomone/issues/153).** `record_fact` is not idempotent on MariaDB: `cross_backend_tests::mariadb_backend::record_fact` returns `Recorded` where SQLite returns `AlreadyRecorded`. Pre-existing (reproduced at `f554d1d`), surfaced while running the ignored legs during the story-3.4 review. **Root cause corrected since first noted here:** the earlier guess ("`ON DUPLICATE KEY UPDATE` vs `INSERT OR IGNORE` asymmetry") was wrong — the SQL is fine. `sqlx-mysql` 0.8.6 unconditionally enables `Capabilities::FOUND_ROWS`, so MariaDB reports *matched* rather than *changed* rows and a replayed id yields `rows_affected() == 1`. Worse than a wrong return value: the code then **re-applies the projection**, which can silently re-settle a task that was corrected in between. Details, evidence and candidate fixes in the issue.

## Deferred from: code review of story-3.4 (2026-07-21)

- ⏸️ **AC 1's "no `Local::now()` in `pomone-app` or below" is literally false.** `crates/pomone-app/src/backup.rs:27` reads `Local::now()` to stamp backup filenames, and is called from `app.rs:126`/`app.rs:148`. Pre-existing and untouched by story 3.4; unrelated to the agronomic date-injection path that AR12 actually governs (every clock read on the task-generation path is now at the UI edge). Either the rule should be stated as "no clock on any path where a date flows into business logic", or `backup.rs` should take an injected timestamp like `record_fact` does. Cosmetic either way — a backup filename is not agronomic time.
- ⏸️ **No backfill for plantings terminated before migration 0014** (decision by Guy, 2026-07-21). Rows already carrying a terminal `status` keep `terminated_on = NULL`, so `occupancy_window` falls back to the scheduled end and they keep loading the capacity curve — the FR15 defect survives for them. **Accepted because the only existing database is disposable test data the owner will recreate, and Pomone is pre-release with a single user.** That reasoning is situational, not structural: before any database is shipped, shared, or dogfooded long-term, this needs either a backfill migration or a read-path fallback treating "terminal status + NULL date" as terminated. Cheap to add later — the column is already there.
