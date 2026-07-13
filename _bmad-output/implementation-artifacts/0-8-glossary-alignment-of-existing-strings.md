# Story 0.8: Glossary alignment of existing strings

Status: done

## Story

As the product owner,
I want existing `.ftl` entries and screen labels aligned to the glossary (notably EN «succession» for série, «skipped» wording, «bed»),
So that the 0.6 CI test can widen to all keys and the app speaks one language per locale.

## Acceptance Criteria

1. **Given** the founding glossary **when** existing keys are renamed/reworded (fr+en in lockstep, key-set parity kept) **then** the CI coherence test runs unscoped and green.
2. **And** no user-visible regression (manual smoke on all screens, both locales).

## Outcome — the alignment audit found nothing to rename

An audit of every candidate string (owner-confirmed) showed the existing catalogues are **already aligned**; the epic's «succession for série» framing conflated two distinct concepts. So 0.8 ships the **glossary restructure + test widening** with **zero user-visible string changes** — which makes AC-2 trivially satisfied (no strings touched → no regression, no smoke needed).

- **`série`/`series` stay as-is.** Every occurrence (fr+en) is a *recurring task series* (watering, mowing) — correct, and distinct from market-gardening *succession* (staggered replanting, arrives with E2). Added as founding term `task-series` (FR «Série (récurrente)» / EN «Series (recurring)»).
- **`planche`↔`bed` are already consistent** in the strings (EN «bed» everywhere, FR «planche»). No dedicated bed/planche label key exists to anchor a check (location kinds are user data), so `bed` stays **documented, not machine-checked**.
- **`skipped` / `field-event` / `correction` / `growing-schedule`** have no strings today — introduced (and promoted into the checked table) by their epics (E1/E2).

## Tasks / Subtasks

- [x] Task 1: Restructure `docs/glossaire.md` into two tables (AC: 1)
  - [x] Table 1 — founding terms **CI-checked** (12 rows, all with a Fluent prefix; dropped the `CI scope` column — the whole table is now covered).
  - [x] Table 2 — planned/documented vocabulary **outside the gate** (`bed`, `succession`, `growing-schedule`, `skipped`, `field-event`, `correction`) with per-term status; a "Terminology decisions" section records the série≠succession and planche↔bed findings.
- [x] Task 2: Widen `glossary_coherence.rs` to run **unscoped** (AC: 1)
  - [x] Removed the `checked`/`deferred` scope mechanism; the test now checks **every** Table-1 row.
  - [x] Parser bounded to Table 1 only (stops at the first non-table line) so Table 2's differently-shaped rows are never parsed.
- [x] Task 3: Verify (AC: 1, 2)
  - [x] `cargo test -p pomone-app --test glossary_coherence` green — **12 founding terms checked, unscoped**.
  - [x] Negative tests: breaking a Table-1 prefix fails (message confirms "12 founding term(s) checked"); tampering a Table-2 cell still passes (Table 2 correctly ignored).
  - [x] `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` → 391 passed, 0 failed.
  - [x] No `.ftl` changes → AC-2 (no user-visible regression) holds by construction; no screen smoke required.

## Dev Notes

- The founding table is now the single source of truth the gate enforces; promoting a planned term (Table 2 → Table 1) is a one-line move the introducing epic makes alongside its new fr+en keys.
- `task-series` is anchored on `task-form-series` (the recurring-series badge key), present and parity-clean in both locales.

## Completion Notes

- Closes epic 0. No production code touched; the change is documentation (glossary) + a widened integration test.
- The «succession for série» rename was deliberately **not** applied — it would have mislabelled recurring tasks. `succession` is reserved for E2's staggered plantings and documented as planned vocabulary.
