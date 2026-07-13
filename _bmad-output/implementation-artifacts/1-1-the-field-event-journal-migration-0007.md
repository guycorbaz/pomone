# Story 1.1: The field_event journal (migration 0007)

Status: in-review

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

## Completion Notes

_(review pending — 3-layer adversarial review scheduled per the Epic 0 retro action item AI-2, since this is the first schema/domain/dual-backend story.)_
