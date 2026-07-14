# Story 1.4: The virtual PrintDoc contract and the rough weekly print

Status: in-review

## Story

As the product owner,
I want a frozen, versioned PrintDoc data contract rendered as a **plain-text printable** of my real week (no printpdf yet — E4 owns PDF),
So that dogfooding starts now (clause 1) and E4 only adds rendering (clause 2).

## Acceptance Criteria

1. **Given** my real database **when** I trigger «Imprimer ma semaine (brut)» **then** a dated plain-text file lists the week's tasks by day then bed (tour-de-plaine), states ☐/☒/⊘, bed+crop per line, saved via the export ritual.
2. **And** the PrintDoc DTO is documented + versioned in the glossary; the harness asserts its shape.
3. **And** the harness gains: facts → virtual PrintDoc → projection assertions (the E1→E4 oracle).

## Design decisions (owner-confirmed)

- Skipped tasks render as **⊘ + reason** (`(ignorée : météo)`); pending ☐, done ☒.
- The printed week is **Monday→Sunday of the current week** (the ISO week containing the reference date).
- The DTO (`WeekSheet` v1) is **locale-neutral** — enums (`EntryState`, `SkipReason`) + dates, never localized strings. Renderers (text now, PDF in E4) localize the chrome; the bed/crop/task labels are user data.

## Tasks / Subtasks

- [x] Task 1: The frozen, versioned contract (AC: 2)
  - [x] `crates/pomone-app/src/printdoc.rs` — `PRINTDOC_VERSION = 1`, `WeekSheet { version, week_start, week_end, days }`, `DaySheet`, `Entry`, `EntryState`. Serializable, locale-neutral.
- [x] Task 2: Build from the real DB (AC: 1)
  - [x] `build_week_sheet(repo, reference)` — Monday→Sunday, projects each task's state (completed→Done, skipped→Skipped, else Pending), groups by day then bed (tour-de-plaine, sorted).
- [x] Task 3: Plain-text renderer + export ritual (AC: 1)
  - [x] `render_text(sheet, i18n)` — ☐/☒/⊘, `bed · crop — task`, skip reason inline, localized chrome (new `print-*`, `weekday-*`, `skip-reason-*` Fluent keys, fr+en). `export_week_sheet(repo, i18n, reference, dir)` writes `pomone-semaine-<monday>.txt`.
- [x] Task 4: The triggers (AC: 1)
  - [x] CLI `pomone-cli print-week [--week YYYY-MM-DD]` (dogfooding-ready, scriptable). UI button "🖨 Imprimer ma semaine (brut)" on the Home page → export next to the DB + open with the system viewer (3-layer Slint wiring + status banner).
- [x] Task 5: The harness oracle (AC: 3)
  - [x] `paper_loop.rs` `step_e1_record_facts` filled: create a task, record a skip fact, build the `WeekSheet`, assert the entry projects to `Skipped` and the contract `version` holds — facts → projection → PrintDoc.
- [x] Task 6: Glossary (AC: 2)
  - [x] `docs/glossaire.md` — `printdoc` founding term (prefix `print`, CI-checked) + a "Le contrat PrintDoc (WeekSheet, v1)" section documenting the shape + versioning rule.
- [x] Task 7: Verify (AC: 1–3)
  - [x] `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` → 422 passed, 0 failed; coverage ~82% lines.
  - [x] CLI smoke on the demo DB: the sheet renders the real week (tour-de-plaine, grouped by bed, ☐ states). Glossary coherence green (12 founding terms).

## Dev Notes

- **The UI button and the CLI share the exact `export_week_sheet` path**, which the CLI smoke exercised on real demo data (verified output: `Semaine du Lundi 23 Février 2026` → `Dimanche 1 Mars 2026` with the sow tasks grouped by bed). The Slint button is compiled + wired; its smoke is the shared export path (no Xvfb click automation for a rough story).
- **`recorded_at`/clock at the UI only** (continues 1.3): the Home button reads `Local::now()`; `build_week_sheet` and the renderer read no clock.
- **Versioning**: any breaking shape change bumps `PRINTDOC_VERSION`; E4's PDF renderer branches on it and the harness asserts it.
- E4 owns the PDF; 1.4 deliberately ships only the plain-text renderer (no `printpdf`).

## Completion Notes

_(review pending — 3-layer adversarial review per retro AI-2.)_
