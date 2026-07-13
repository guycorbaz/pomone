# Story 0.6: CHECK audit, glossary and CI coherence test

Status: done

## Story

As the product owner,
I want the SQLite CHECK constraints audited, `docs/glossaire.md` created (~14 founding terms: term_id, FR, EN, definition, Fluent prefix), and a CI test asserting glossary↔Fluent coherence,
So that the states epic can't hit the CHECK trap and terminology can't drift.

## Acceptance Criteria

1. **Given** migrations 0001–0006 **when** the audit runs **then** every CHECK constraint is documented and confirmed harmless to the convergence (or its mitigation named).
2. **And** the glossary exists (succession, skipped, bed, growing schedule… included).
3. **And** the CI test fails when a term_id lacks a Fluent key or translations omit the glossary term — **scoped to glossary-tagged keys so it is born green** (existing strings align in 0.8).

## Tasks / Subtasks

- [x] Task 1: Audit every CHECK constraint in migrations 0001–0006 (AC: 1)
  - [x] `docs/check-constraint-audit.md` — 12 CHECKs found (11 in `0001_initial.sql`, 1 in `0003_planting_status.sql`; 0002/0004/0005/0006 add none).
  - [x] Classified A (value-set enums = trap), B (fundamental enums, stable), C (sum-type structural invariants), D (value bounds). Each risk-rated with a named mitigation; SQLite↔MariaDB parity confirmed.
- [x] Task 2: Write the founding glossary (AC: 2)
  - [x] `docs/glossaire.md` — 17 founding terms, columns term_id / FR / EN / definition / Fluent prefix / CI scope. Includes the AC-named `succession`, `skipped`, `bed`, `growing-schedule`, plus `field-event`, `correction` and the existing catalog nouns.
- [x] Task 3: The coherence gate (AC: 3)
  - [x] `crates/pomone-app/tests/glossary_coherence.rs` — parses the glossary table (column-name driven) and both `.ftl`; for every `checked` term asserts its Fluent prefix matches ≥1 key **and** fr↔en key sets are identical under that prefix.
  - [x] Scoped to `checked` rows (11 tagged today); convergence terms not yet wired stay `deferred` → born green.
- [x] Task 4: Verify (AC: 1–3)
  - [x] `cargo test -p pomone-app --test glossary_coherence` green; **negative test** confirmed (flipping `succession` to `checked` with an unmatched prefix fails with the expected message).
  - [x] `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` → 390 passed, 0 failed.

## Dev Notes

### The CHECK trap and the one live risk

The only genuinely dangerous constraint is `task_type.category IN (…)` — SQLite can't add an allowed value without a table rebuild. Mitigation (already in `CLAUDE.md` + epics AR2): **never extend the CHECK**; use `category = 'other'` + the free `task_type.name` and `category-*` Fluent labels; fine-grained additions go through additive seed defaults. `planting.status` (0003) is watched but the states epic (E1) records new field states in the additive, CHECK-free `field_event` table rather than by extending the status enum. Everything else is a fundamental enum, a sum-type structural invariant, or a value bound — all harmless.

### Born-green by scoping, not by weakening

The catalogues are already parity-clean (536 keys each, zero fr/en diff), so any tagged prefix passes today. Terms whose EN wording still diverges (`série`→`succession`, `planche`→`bed`) or that a later epic introduces (`skipped`, `field-event`, `growing-schedule`, `correction`) are listed as founding vocabulary but carry `CI scope = deferred` and a `—` prefix, so the gate skips them until story 0.8 (renames) or the introducing epic flips them to `checked`. The test guards against a `deferred`-everything regression: it asserts at least one `checked` row exists, rejects unknown scope tokens, and flags duplicate term_ids.

### Files

- `docs/check-constraint-audit.md` (new)
- `docs/glossaire.md` (new)
- `crates/pomone-app/tests/glossary_coherence.rs` (new, CI-blocking via `cargo test --workspace`)

## Completion Notes

- No production code touched — audit + glossary are docs, the gate is an integration test. `cargo test --workspace` picks it up automatically, so it is CI-blocking from merge without any workflow-YAML change.
- The gate is column-name driven (locates term_id / Fluent prefix / CI-scope columns by header text), so reordering or adding glossary columns won't break it.
- Story 0.8 will widen scope: align the diverging EN strings, then flip `succession`/`bed` (and any epic-introduced terms once wired) to `checked`.
