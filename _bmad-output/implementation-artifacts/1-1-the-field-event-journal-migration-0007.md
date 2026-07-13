# Story 1.1: The field_event journal (migration 0007)

Status: done

## Story

As the product owner,
I want the append-only `field_event` table (client UUIDv4, dot-namespaced kind, target, occurred_at, recorded_at, JSON payload, `corrects`) plus additive task skip columns, on both backends,
So that every field gesture has a durable, idempotent record.

## Acceptance Criteria

1. **Given** `0007_field_event.sql` in both migration trees (additive, no CHECK) **when** cross-backend tests run **then** FactKind/SkipReason round-trip with identical literals on both backends.
2. **And** duplicate event id insert = conflict-no-op.
3. **And** `copy_all` covers the table; a decade-old fixture migrates cleanly.

## Design decisions (schema is additive-only → irreversible)

- **`field_event`**: `id` (client UUIDv4, idempotency key) · `kind` (FactKind, dot-namespaced) · `target_kind` + `target_id` (**non-FK** — the journal must outlive a deleted target: "nothing is lost") · `occurred_at` (`NaiveDate`, backdatable) · `recorded_at` (**`NaiveDateTime`** — audit precision + stable ordering; the 1.3 invariant `occurred_at ≤ recorded_at` is 1.3's) · `payload` (JSON `TEXT`) · `corrects` (nullable, non-FK). **No CHECK** (0.6 audit) — the `kind`/`target_kind` literal sets grow freely.
- **Idempotency**: `ON CONFLICT(id) DO NOTHING` (SQLite) / `INSERT IGNORE` (MariaDB) — a replayed insert is a silent no-op. No prior precedent; introduced here.
- **Task skip columns** (`skipped_on`, `skip_reason`, `skip_note`) are added and **read** now (row-mapper decodes them; `Task` gains the fields, defaulting `None`); they are **written only by `facts::record_fact`** (story 1.2), so `task_update`/`task_create` do not project them (aligns with the 1.2 lint "no `UPDATE task SET skipped_on` outside facts.rs").
- **FactKind** starter set: `task.done`, `task.skipped`, `planting.terminated`. A *correction* is not a kind — it is any event with `corrects` set.
- **SkipReason** starter set: `weather`, `pest-disease`, `crop-failure`, `no-time`, `not-needed`, `replaced`, `other`. Not schema-baked; the UI wording lands in 1.5.

## Tasks / Subtasks

- [x] Task 1: Migration `0007_field_event.sql` on both backends (AC: 1, 3)
  - [x] SQLite (BLOB/TEXT/STRICT) + MariaDB (BINARY(16)/DATE/DATETIME(6)/TEXT), additive, **no CHECK**; two indexes (`target`, `recorded`); `ALTER TABLE task ADD COLUMN` × 3.
- [x] Task 2: Domain (AC: 1)
  - [x] `FieldEvent` + `FactKind` + `SkipReason` (`field_event.rs`); `FieldEventId` via `define_id!`; `Task` gains `skipped_on`/`skip_reason`/`skip_note` (constructors default `None`); `lib.rs` re-exports.
- [x] Task 3: Codec (AC: 1)
  - [x] `encode/decode_fact_kind`, `encode/decode_skip_reason` + `opt_skip_reason_{to,from}_text`; round-trip + invalid-literal unit tests. Both used in non-test code (field_event impls; task row-mapper/insert) → no `dead_code`.
- [x] Task 4: Repository + both backends (AC: 1, 2)
  - [x] `FieldEventRepo` (`create`/`get`/`list_for_target`/`list_all`), aggregated into `Repository`, re-exported. SQLite + MariaDB impls with the conflict-no-op insert. `task.rs` (both) wire the skip columns into `TASK_COLUMNS`/INSERT/row-mapper.
- [x] Task 5: `copy_all` (AC: 3)
  - [x] `MigrationReport.field_events` + a copy loop after tasks (idempotent create → safe re-run). Covered by an extended migration test asserting the journal round-trips into the target.
- [x] Task 6: cross_backend_tests (AC: 1, 2)
  - [x] `scenario_field_events`: round-trip, **duplicate-id no-op** (row count stays 1), ordering, correction pointing back, target isolation, whole-journal view — on both backend entry points.
- [x] Task 7: Verify
  - [x] `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` → 402 passed, 0 failed.
  - [x] XDG-isolated `seed-demo`: migration 0007 applies on a real file DB; `field_event` table + the 3 `task` skip columns confirmed present (python sqlite3).

## Dev Notes

- **"Decade-old fixture migrates cleanly"** is structurally guaranteed here: 0007 is additive (`CREATE TABLE` + nullable `ALTER ADD COLUMN`), and every test DB runs the full 0001→0007 chain. The heavyweight *real-database* migrate+smoke runbook is explicitly **story 1.7's** deliverable; 1.1 proves the DDL composes and that `copy_all` carries a populated journal across backends.
- **MariaDB** impl (`mariadb/field_event.rs`) is untested locally by design (testcontainer `#[ignore]`d; counted at 0% coverage per `CLAUDE.md`) — it mirrors the SQLite impl 1:1 except `?` placeholders and `INSERT IGNORE`. Its parity is asserted by the shared `scenario_field_events` when run with `--ignored` under Docker.
- **`recorded_at` = `NaiveDateTime`**: sqlx stores it as ISO TEXT (SQLite) / `DATETIME(6)` (MariaDB); round-trip verified by `scenario_field_events` (SQLite leg).

### Review Findings

3-layer adversarial review (Blind Hunter / Edge Case Hunter / Acceptance Auditor) — per Epic 0 retro AI-2. 6 patch, 1 defer, 3 dismissed.

- [x] [Review][Patch] Deterministic journal order: added `, id` tiebreaker to every `ORDER BY recorded_at` (both backends). [crates/pomone-db/src/{sqlite,mariadb}/field_event.rs]
- [x] [Review][Patch] MariaDB conflict-no-op scope: replaced `INSERT IGNORE` with `INSERT … ON DUPLICATE KEY UPDATE id = id` — only a PK conflict no-ops now, matching SQLite's `ON CONFLICT(id) DO NOTHING`. [crates/pomone-db/src/mariadb/field_event.rs]
- [x] [Review][Patch] Bounded `target_kind` to `MAX_TARGET_KIND_LEN = 64` in `FieldEvent::new` (new `DomainError::TooLong`) and widened the MariaDB column to `VARCHAR(64)` (= `kind`); both backends now accept/reject the same values. Boundary + rejection tested. [crates/pomone-domain/src/{field_event.rs,error.rs}, migrations/mariadb/0007_field_event.sql]
- [x] [Review][Patch] Truncate `recorded_at` to microseconds in `FieldEvent::new` (nanos → µs) so SQLite (TEXT) and MariaDB `DATETIME(6)` round-trip identically. Tested. [crates/pomone-domain/src/field_event.rs]
- [x] [Review][Patch] `target_has_data` now probes `field_event_list_all` too — a journal-only target no longer slips the emptiness guard. [crates/pomone-app/src/migration.rs]
- [x] [Review][Patch] Added `scenario_task_skip_roundtrip` (both backend entry points): builds a task with `skip_reason`/`skipped_on`/`skip_note` set and asserts the DB round-trip — closes AC1's literal "SkipReason round-trip on both backends". [crates/pomone-db/src/cross_backend_tests.rs]
- [x] [Review][Defer] Story 1.3's `occurred_at ≤ recorded_at` guard must tolerate/flag rows written before 1.3 — 1.1 is append-only, so any inverted pair recorded pre-1.3 is permanent history the forward-only invariant can't retroactively reject. [crates/pomone-domain/src/field_event.rs] — deferred to story 1.3.
- [x] [Review][Dismissed] Three Lows dismissed by design: (a) `payload` not JSON-validated — an append-only "nothing is lost" journal must not *reject* a fact over payload shape; 1.2 owns payload construction. (b) `payload → "{}"` normalisation only in the constructor — matches the codebase's pub-fields-plus-constructor-invariant pattern (`Treatment`, `Task`). (c) idempotent replay silently drops divergent content for a reused id — by design; the client id IS the idempotency key.

## Completion Notes

- 3-layer adversarial review (retro AI-2) run on PR #125: **6 patch, 1 defer, 3 dismissed, 0 blocking**. All 6 patches applied — they hardened dual-backend parity (ordering tiebreaker, `ON DUPLICATE KEY UPDATE` scope, `target_kind` length + column alignment, µs truncation) and closed an emptiness-guard gap and the literal AC1 `SkipReason` DB round-trip.
- Post-fix: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` → **405 passed, 0 failed**; coverage **81.46% lines** (≥80).
- One deferral recorded for **story 1.3**: its `occurred_at ≤ recorded_at` guard must tolerate/flag rows written before 1.3 (append-only ⇒ pre-1.3 inverted pairs are permanent).
