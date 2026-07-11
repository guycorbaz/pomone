---
stepsCompleted: ['step-01-validate-prerequisites', 'step-02-design-epics', 'step-03-create-stories', 'step-04-final-validation']
status: 'complete'
completedAt: '2026-07-12'
inputDocuments:
  - _bmad-output/planning-artifacts/prd.md
  - _bmad-output/planning-artifacts/architecture.md
  - _bmad-output/planning-artifacts/ux-design-specification.md
  - _bmad-output/project-context.md
---

# pomone - Epic Breakdown

## Overview

This document provides the complete epic and story breakdown for pomone's QRop-convergence (R1 «Usable in the field»), decomposing the PRD requirements, UX Design specification and Architecture decisions into implementable stories. Discipline: **every slice leaves `main` releasable and extends the weekly paper-loop harness.**

## Requirements Inventory

### Functional Requirements

- FR1 *(R1)*: The grower can define a crop-plan line — crop/variety, quantity as series × bed-geometry, staggering interval — **without assigning a location**.
- FR2 *(R1)*: The grower can duplicate an existing plan line and edit the copy.
- FR3 *(R1)*: Plan lines carry a draft/complete state; the grower can resume a fragmented planning session where they left off.
- FR4 *(R1)*: A plan line generates its staggered plantings (N successions at the defined interval).
- FR5 *(R1)*: The grower can define reusable itinéraires techniques (activity templates) anchored on establishment with signed day-offsets (before and after).
- FR6 *(R1)*: Tasks are generated from the ITK at placement, including pre-establishment activities.
- FR7 *(R1)*: The grower can produce a needs list — aggregated seed/plant quantities with buy-by deadlines — from plan lines, including unplaced ones.
- FR8 *(R2)*: The grower can carry a season's plan into the following year without re-entry.
- FR9 *(R1)*: The grower can place plantings into the location hierarchy (parcel → sector → bed).
- FR10 *(R1)*: The system shows a live soil-occupancy curve at placement, counting sheltered and open-field capacity separately.
- FR11 *(R1)*: The grower can see which series compose any capacity peak (explainable conflicts).
- FR12 *(R1)*: Capacity aggregates up the location hierarchy, readable at every level.
- FR13 *(R1)*: Annual and perennial plantings coexist on one parcel, each with its own time horizon (perennials occupy to end of horizon).
- FR14 *(R1)*: The grower can retro-enter pre-existing perennial plantings (historical establishment dates) without generating past tasks.
- FR15 *(R1)*: A terminated perennial releases its occupancy; a replacement can share the row (two ages coexist).
- FR16 *(R2)*: The grower can see a bed's crop history and family rotation interval at placement.
- FR17 *(R2)*: The system can propose bed-use optimizations (deterministic algorithm).
- FR18 *(R1)*: Tasks have three states — pending, done{date}, skipped{reason from closed set + optional note, never mandatory}; skipped tasks vanish from future lists and prints but remain in retrospective.
- FR19 *(R1)*: Task auto-generation never resurrects a skipped task; skipping an occurrence never affects its recurring series.
- FR20 *(R1)*: The grower reconciles in batch «since the last entry», in the printed sheet's order, three gestures per line (done / skipped / leave pending).
- FR21 *(R1)*: Every field event carries `occurred_at` (backdatable, proposed from the sheet's dated column) distinct from `recorded_at`.
- FR22 *(R1)*: Long gaps reconcile as a bounded batch (bulk accept), not line-by-line punishment.
- FR23 *(R1)*: A reconciliation session is interruptible at any line — each validated line is persisted immediately, the next session resumes exactly there; no Save button anywhere.
- FR24 *(R1)*: «Done», «skipped» and «terminated» are reversible as explicit data-entry corrections; nothing is ever un-done silently.
- FR25 *(R1)*: Replanning a task/planting and correcting an entry are two distinct actions, distinguishable in the UI at the moment of use.
- FR26 *(R1)*: Planting lifecycle transitions (placed → active → terminated{cause}) each carry an explicitly entered date.
- FR27 *(R2)*: The grower can declare a planned treatment (product, dose, target beds) — the system computes the needed product quantity as a preparation aid — then confirm it done with the actual quantity used (closed unit set, prefilled, correctable).
- FR28 *(shipped, kept)*: The grower can record treatments, recurring tasks, and split/move plantings.
- FR29 *(R1)*: The system generates four documents as PDFs saved to a configurable directory with dated filenames: multi-day journal sheet, planned/in-progress crop list, bed-occupancy map, treatment register.
- FR30 *(R1)*: The journal sheet is a multi-day form: one dated column per day, tasks in bed order, free-note zones, and a header stating coverage period + data freshness («last reconciliation N days ago», from `recorded_at`).
- FR31 *(R1)*: Every document is re-printable at any time and reflects current state: done stays done, skipped never reappears, replanned items sit on their new dates.
- FR32 *(R1)*: All documents exist in FR and EN, are legible in black-and-white, paginate on overflow, and carry the generating version + print date in the footer.
- FR33 *(R1)*: The grower can export raw treatments as CSV (inspection escape hatch).
- FR34 *(R2)*: The system produces the consumption ledger — per product and period, summed confirmed quantities, gaps explicitly listed — printable and CSV-exportable.
- FR35 *(R2)*: The system produces the Acorda census summary — cultures and areas per parcel at a reference date — printable/exportable.
- FR36 *(R2)*: The grower can export the season's structured history (planned vs actual, yields, dates, skip patterns, observations) for use with an external AI agent.
- FR37 *(shipped, kept)*: The grower records yearly harvests per perennial planting (expected/actual/variance), yields optional and fillable as reality arrives.
- FR38 *(R2)*: The grower can record harvest quantities for annual crops.
- FR39 *(R2)*: The grower can review a season — planned vs actual dates, yields, skip patterns and their reasons, weather events — as the deterministic retrospective.
- FR40 *(R2)*: The grower can keep an observation journal: dated, typed entries (incl. weather facts and unplanned work), attachable to a planting/bed, with photos by reference; a photo inbox lets field photos land in Pomone for later qualification.
- FR41 *(R2)*: The grower can review varieties across seasons (results, observations) and annotate each variety with a free «source/why» note at creation.
- FR42 *(shipped, kept)*: The grower manages crops, varieties, locations (hierarchy with dimensions), strata, families (colors), task types; display units m²/ha and kg/t; SQLite or MariaDB backend with live migration and backups.
- FR43 *(R2)*: The grower can reference a variety from an online supplier catalog without re-typing its data.
- FR44 *(R1)*: First-run experience: seeded botanical families only, a loadable demo farm strictly isolated from real data, an honest «no QRop import» notice with a fast re-entry path, contextual tooltips, and a getting-started manual (F1) — leading an unaided newcomer to a first printed plan.
- FR45 *(R1)*: The full UI and all documents are available in French and English, switchable at runtime.
- FR46 *(R1)*: The complete plan→place→print→reconcile cycle works with no network connection.
- FR47 *(R1)*: The application starts into the reconciliation catch-up screen, resuming any interrupted work.
- FR48 *(R2)*: The grower can opt into a discreet «new version available» check; a periodic backup reminder; a rotating local log file supports voluntary bug reports; a font-size/large-print setting serves aging eyes.
- FR49 *(shipped, kept — affected by new task states)*: The grower visualizes work through the existing views — monthly task calendar (drag-to-reschedule, milestones, holidays greyed), agenda, season Gantt, crop map, home occupancy curve. Skipped tasks render struck-through/greyed in past views and vanish from future ones, per FR18.
### NonFunctional Requirements

- NFR1: Startup to the reconciliation catch-up screen < 3 s on modest farm hardware.
- NFR2: A full week of paper notes reconciles in ≤ 15 min; reconciliation cost proportional to work done. Acceptance datasets: S-40 (40 lines, 70% as-planned) ≤ 10 min; S-april (60 lines + 8 unplanned) ≤ 15 min; in-app chrono, local-only alert.
- NFR3: Unaided newcomer reaches a first printed plan in ≤ 30 min.
- NFR4: Screens responsive with 10+ seasons of history (bounded queries, per-season pagination).
- NFR5: Document generation completes within a few seconds.
- NFR6: No validated line ever lost or duplicated across brutal interruption (kill/replay-verified); line-level persistence.
- NFR7: Product invariants I1–I6 property-tested (done absorbing except explicit correction; skipped never resurrects nor counts as done; series survive occurrence-skip; occurred_at ≤ recorded_at; crash = exact prefix).
- NFR8: Decade-old data round-trips all migrations on both backends; additive-only migrations.
- NFR9: Single-instance protection (flock, friendly message).
- NFR10: Backups — auto pre-migration, manual button (shipped); periodic reminder (R2).
- NFR11: Zero telemetry; no outbound network by default; full cycle runs offline (verifiable).
- NFR12: No account/cloud — GDPR/LPD by architecture.
- NFR13: Data leaves only by explicit user act.
- NFR14: Printed documents B&W-legible, outdoors, self-contained (per-page header, ≤6-symbol legend), photocopy-surviving.
- NFR15: Field-legibility grammar (filled=editable / no-fill=read-only) on all new screens + why-tooltips.
- NFR16: Font-size / large-print setting (R2, token-level ×1.15).
- NFR17: Re-learnable after two-month absence.
- NFR18: Entry ergonomics contract: never re-ask, one line at a time, no Save button.
- NFR19: Localized numeric/date entry (12,5 accepted); ISO-8601 in storage and exports.
- NFR20: Coverage ≥ 80% workspace, ≥ 95% on document-generation (up to PrintDoc) and capacity/autogen; PDF render layer coverage-exempt (ci.yml ignore).
- NFR21: Weekly paper-loop acceptance test release-blocking (brutal interruption + second loop included).
- NFR22: Dual-backend behavioral parity (cross-backend tests for every new table incl. swap/backup coverage).
- NFR23: Zero-warning CI; fr/en Fluent key parity.
- NFR24: PrintDoc golden snapshots in FR and EN (never PDF bytes).
- NFR25: Clone-to-green-tests via README alone.
- NFR26: Stable documented CSV contracts (UTF-8 BOM, semicolon, ISO dates, dot decimals, pomone_version column; docs/export-contracts.md).
- NFR27: PDFs archival-grade (dated filenames, embedded DejaVu fonts, re-openable years later).

### Additional Requirements (Architecture)

- AR1: **No starter template — brownfield.** First story starts from `main` (Slice 0); every slice leaves `main` releasable.
- AR2: **Slice 0 mandate:** wiring refactor (`pomone-ui/src/wiring/`, main.rs shrinks to bootstrap), request structs in `services.rs`, SQLite CHECK-constraint audit (migrations 0001–0006), decision on dormant `task_method_id`/`implement_id` executed (revived for ITK), **empty paper-loop harness** (`pomone-app/tests/paper_loop.rs`, kill/replay runner, XDG-isolated, drives services/view-models only) CI-blocking from day one.
- AR3: **Fact schema (D1):** append-only `field_event` table (client UUIDv4 = idempotency key, kind dot-namespaced, occurred_at/recorded_at, JSON payload ids-and-values-only, `corrects` link) + readable state columns; single write path `facts::record_fact` (one transaction: event insert + projection); no state mutation outside it.
- AR4: **Durability (D2):** SQLite WAL + per-gesture transactions (full guarantee); MariaDB documented-degraded (visible failure on unreachable server).
- AR5: **Location geometry (D3):** bed-meters = leaf `length_m`; covered split from `LocationKind.covered`; additive `occupation_kind` column (value `bed-meters`).
- AR6: **Document pipeline (D4):** `pomone-app/print/` — PrintDoc (localized strings, logical page breaks, injected clock/metrics) → printpdf 0.9 render (coverage-exempt) → dated PDF in configurable `documents_dir`; DejaVu Sans embedded; genericity: new register = new view over same pipeline.
- AR7: **ITK model (D5):** ItkTemplate per crop; ordered ItkActivity {task_type_id, offset_days signed, optional method/implement FKs (revived), label, notes}; no doses in R1.
- AR8: **Demo mode (D6):** separate pomone-demo.sqlite; in-memory state; ignores configured backend; locks swap/backup/restore; seed promoted to pomone-app API.
- AR9: **Export conventions (D7)** per NFR26 + flock single-instance.
- AR10: Migrations 0007 (planning) & 0008 (field_event + skip columns + occupation_kind), additive, ×2 backends, ×8 touchpoints each (codec, sub-traits, impls, cross-backend tests, copy_all, seed).
- AR11: Autogen guard upgraded: idempotence keyed (planting, task_type, campaign-window), done AND skipped count as existing.
- AR12: Time injection: recorded_at passed from UI/CLI layer; never now() below; single visibility predicate shared views/print.
- AR13: Capacity engine pure functions on `date_calc.rs`; algebraic property tests (superposition, horizon-extension stability, hierarchy coherence at every t, ±50y retro-entry).
- AR14: Interleaving state-machine proptests (`tests/fact_invariants.rs`) for autogen ∘ reconciliation ∘ edition.

### UX Design Requirements

- UX-DR1: **Design tokens formalized/extended** in theme.slint: `settled` grey (AA both themes), density variant (sprint ≈0.6 vertical), `font-size-row` ≈16px, motion tokens (150ms/0ms), every new token light+dark from day one.
- UX-DR2: **Field-state grammar** on all new screens: editable=filled+border, derived=no-fill muted, refused=amber+inline message+Échap restores; validation on exit.
- UX-DR3: **Screen components:** TriageRow (7 states), ReasonStrip (inline 1–4, never PopupWindow), DayGroupHeader (+ bulk variant), GridCell (editable/derived/forced), DateField (closed grammar + live echo), FreeLineInput, CounterFoot (+ completion swell), PeakPanel, FreshnessLine, DemoBanner, EmptyOutline; conditional SheetMiniMap (pending paper test).
- UX-DR4: **Reconciliation corridor** = Direction B (day-grouped sheet mirror, tour-de-plaine order): centralized FocusScope + current-index, physical keys (␣/X/R/N/↵/+/Échap/B), keyboard-complete, line-local model updates (kept VecModel handle), contextual landing (empty/pending/off-season), «Boucler» explicit always-available, in-app chrono (blur/2min pause).
- UX-DR5: **Plan grid** = «tableur fidèle»: arrows, type-to-edit, Enter advances, Ctrl+D duplicates, typed cells, derived columns visually distinct.
- UX-DR6: **Printed documents** per A4 mock: PageHeader every page (farm, period, print date, version, glossary version, freshness, pagination), DayColumn (tour-de-plaine), CheckLine (☐, bold bed, qté blank), Legend ≤6 symbols incl. handwriting convention (✓, qté+unit, → report, skip motif), RegisterTable, OccupancyBar (hatching, ≥40% grey).
- UX-DR7: **Glossary** `docs/glossaire.md` (term_id, FR, EN, definition, Fluent prefix) + CI coherence test glossary↔.ftl + version stamped on documents; founding terms incl. série→succession, abandonnée→skipped.
- UX-DR8: **UX patterns:** one primary per surface; destructive confirmations name the data; inline feedback never modal; Échap=cancel everywhere; empty states = EmptyOutline + proposition; export ritual (dated file, status line with path, «ouvrir»); tooltips = the why; no merit badges.
- UX-DR9: **Brand promise «Saisi une fois, produit partout»** as transversal acceptance criterion; every borrowed mechanic answers «how do you serve the paper loop?».
- UX-DR10: **Pre-code validation:** the Thursday-evening paper test (A4 mock + owner's real week, wizard-of-Oz, <10 min success) gates the corridor implementation.
- UX-DR11: Window adaptation (1366×768 reference, 100%+150% scale), keyboard focus always visible, sprint rows ≥34px.

### FR Coverage Map

### FR Coverage Map

**R1 epics:**
FR18, FR19, FR21, FR24, FR25, FR26 → Epic 1 (facts & states) · FR28/FR37/FR42/FR49 (shipped) → preserved, regression-guarded in Epic 1
FR1–FR7 → Epic 2 (planning: plan lines, ITK, generation, needs list on-screen)
FR9–FR15 → Epic 3 (placement & capacity; FR11 peak *explanation UI* completed in Epic 7)
FR29, FR30, FR31, FR32, FR46 → Epic 4 (documents: journal sheet; needs-list printing joins here)
FR20, FR22, FR23, FR47 → Epic 5 (reconciliation corridor + contextual landing)
FR33 → Epic 6 (registers: crop list, treatment register, raw CSV)
FR44, FR45 → Epic 8 (welcome; FR45 language parity is also a DoD from Epic 0 on)

**R2 backlog (not planned):**
FR8, FR16, FR17, FR27, FR34, FR35, FR36, FR38, FR39, FR40, FR41, FR43, FR48 → Backlog R2

**Total: 49/49 FRs mapped.**

## Epic List

**Sequencing law (owner-ordered):** facts hold → plan → place → print → reconcile. Strictly sequential epics; every story leaves `main` releasable; every epic extends the paper-loop harness (data or step — stated in its DoD).

**Transversal conventions (Definition of Done, every story from Epic 0):** fr+en Fluent keys for any user-facing string; 8-touchpoint checklist for persisted changes; migrations numbered by merge order (0007_field_event → E1, 0008_planning → E2, 0009_geometry → E3); field-state grammar + why-tooltips on new UI; «Saisi une fois, produit partout» as acceptance lens.

**Sign-off clauses (panel-negotiated, owner-ratified):**
1. *(John)* Epic 1 ships a **rough weekly print ritual**: the virtual PrintDoc renders a rudimentary printable used by the owner weekly on existing tasks — dogfooding starts at E1, not E4.
2. *(Amelia)* The **virtual-PrintDoc data contract is frozen at E1** (versioned in the glossary, asserted by the harness); E4 only adds rendering.
3. *(Amelia)* E1 view stories include the acceptance criterion: entry errors correctable from existing views (no corridor needed).
4. *(Murat)* E4 DoD: reserved slots in PrintDoc for E5's reconciliation mentions (goldens survive E5); E5 DoD: I4–I6 proptests reuse E1's fact generators enriched with backdating.
5. *(Owner)* Tasks are crop+place-bound in the vast majority; generic tasks possible but rare (model allows, UI doesn't foreground). ITK-less crops fall back to the shipped variety-profile autogen — the ITK enriches, never conditions.
6. *(Owner+John)* `FreeLineInput` gains an **optional one-gesture attachment** (crop/bed picker, skippable by Enter) — raw text alone always accepted; the fact carries an optional target. (Story in E5; UX spec amended.)

### Epic 0: Fondations

Enabling epic (AR1/AR2): wiring refactor, request structs, audits, glossary, harness skeleton, glossary alignment. Value: CI green with the harness blocking; `main.rs` maintainable. No behavior change. *(Epic DoD everywhere: manual updated, harness extended, fr/en parity.)*

### Story 0.1: Wiring skeleton and settings module extraction

As a contributor (human or AI agent),
I want `pomone-ui/src/main.rs` to delegate screen wiring to per-screen modules, starting with `wiring/` and the settings/backend/i18n screens,
So that new screens never add callbacks to a 5500-line file and the pattern is established.

**Acceptance Criteria:**

**Given** the monolithic `main.rs`
**When** `wiring/mod.rs` + `wiring/settings.rs` are extracted (settings, backend swap, backup, holidays, units, language)
**Then** tests, clippy `-D warnings` and a manual launch behave identically
**And** no settings `on_*` registration remains in `main.rs`
**And** the pattern (`fn wire_<screen>(…)`) is documented in the module header.

### Story 0.2: Catalog screens wiring extraction

As a contributor, I want crops/varieties/locations/strata/families wiring extracted, So that catalog screens follow the pattern.

**Acceptance Criteria:** **Given** 0.1's pattern **When** the five catalog screens are extracted **Then** behavior is unchanged (tests + manual smoke) **And** no catalog callbacks remain in `main.rs`.

### Story 0.3: Plantings, detail and treatments wiring extraction

As a contributor, I want plantings, planting-detail and treatments wiring (incl. split/move dialogs) extracted, So that the planting family follows the pattern.

**Acceptance Criteria:** **Given** 0.1's pattern **When** the three planting screens are extracted **Then** behavior is unchanged **And** no planting callbacks remain in `main.rs`.

### Story 0.4: Tasks, calendar, agenda and harvests wiring extraction

As a contributor, I want the remaining screens extracted, So that `main.rs` shrinks to bootstrap + wiring calls.

**Acceptance Criteria:** **Given** 0.1–0.3 merged **When** the remaining screens are extracted **Then** `main.rs` contains only startup, config, window lifecycle and `wire_*` calls (< 500 lines) **And** the full suite is green.

### Story 0.5: Request structs in services

As a contributor,
I want `services.rs` creation functions to take request structs instead of 8–10 positional parameters,
So that E1/E2 can add fields without breaking every call site twice.

**Acceptance Criteria:** **Given** the positional signatures **When** request structs are introduced and all call sites migrated (UI, CLI, demo, tests) **Then** behavior is unchanged **And** no creation function exceeds 3 parameters (repo, request, injected date/clock).

### Story 0.6: CHECK audit, glossary and CI coherence test

As the product owner,
I want the SQLite CHECK constraints audited, `docs/glossaire.md` created (~14 founding terms: term_id, FR, EN, definition, Fluent prefix), and a CI test asserting glossary↔Fluent coherence,
So that the states epic can't hit the CHECK trap and terminology can't drift.

**Acceptance Criteria:**

**Given** migrations 0001–0006
**When** the audit runs
**Then** every CHECK constraint is documented and confirmed harmless to the convergence (or its mitigation named)
**And** the glossary exists (succession, skipped, bed, growing schedule… included)
**And** the CI test fails when a term_id lacks a Fluent key or translations omit the glossary term — **scoped to glossary-tagged keys so it is born green** (existing strings align in 0.8).

### Story 0.7: Paper-loop harness skeleton

As the test architect,
I want `pomone-app/tests/paper_loop.rs` in CI from day one — XDG-isolated DB, kill/replay runner with `FailureMode = Kill | NetworkDrop`, injected clock, golden normalization helpers, explicit `// TODO(E_n)` no-op steps,
So that every epic extends one harness.

**Acceptance Criteria:**

**Given** an empty isolated database
**When** the harness seeds, kills mid-write, restarts, re-opens — on both failure modes
**Then** the database reopens cleanly and the assertions pass
**And** the harness is a required CI check
**And** normalization policy (fixed clock, stable ordering, locale-stable formatting) ships as shared helpers.

### Story 0.8: Glossary alignment of existing strings

As the product owner,
I want existing `.ftl` entries and screen labels aligned to the glossary (notably EN «succession» for série, «skipped» wording, «bed»),
So that the 0.6 CI test can widen to all keys and the app speaks one language per locale.

**Acceptance Criteria:**

**Given** the founding glossary
**When** existing keys are renamed/reworded (fr+en in lockstep, key-set parity kept)
**Then** the CI coherence test runs unscoped and green
**And** no user-visible regression (manual smoke on all screens, both locales).

## Epic 1: Rien ne se perd — faits, états et corrections

Skip with a reason, backdate, correct explicitly — on existing screens; nothing ever lost (D1). **Dogfooding starts at story 1.4.**

### Story 1.1: The field_event journal (migration 0007)

As the product owner,
I want the append-only `field_event` table (client UUIDv4, dot-namespaced kind, target, occurred_at, recorded_at, JSON payload, `corrects`) plus additive task skip columns, on both backends,
So that every field gesture has a durable, idempotent record.

**Acceptance Criteria:**

**Given** `0007_field_event.sql` in both migration trees (additive, no CHECK)
**When** cross-backend tests run
**Then** FactKind/SkipReason round-trip with identical literals on both backends
**And** duplicate event id insert = conflict-no-op
**And** `copy_all` covers the table; a decade-old fixture migrates cleanly.

### Story 1.2: record_fact — the single write path

As the grower,
I want every gesture (done, skipped, terminate, correction) recorded through one `facts::record_fact` (event insert + state projection, one transaction),
So that marked means persisted and re-applying is harmless.

**Acceptance Criteria:**

**Given** a pending task
**When** a `task.done` fact is recorded
**Then** event + projection commit atomically; same-id re-record returns the existing result
**And** `task.skipped` projects the skip columns (FR18 semantics); corrections re-project without touching the original event
**And** a lint test asserts no `UPDATE task SET (completed_on|skipped_on…)` statement exists outside `facts.rs` (exact pattern: SQL-level grep on repo impls + review rule).

### Story 1.3: Dual timestamps and the skip-aware autogen guard

As the grower,
I want occurred_at (backdatable) distinct from caller-injected recorded_at, and autogen never resurrecting a done/skipped task,
So that backdated entry is safe and regeneration never undoes decisions.

**Acceptance Criteria:**

**Given** a skipped task for (planting, task_type, campaign window)
**When** autogen re-runs after planting edit/replan
**Then** no new task inserts for that slot (done AND skipped count as existing)
**And** occurred_at ≤ recorded_at enforced in the domain constructor
**And** no `now()` below the UI/CLI layer (API takes the timestamp).

### Story 1.4: The virtual PrintDoc contract and the rough weekly print

As the product owner,
I want a frozen, versioned PrintDoc data contract rendered as a **plain-text printable** of my real week (no printpdf yet — E4 owns PDF),
So that dogfooding starts now (clause 1) and E4 only adds rendering (clause 2).

**Acceptance Criteria:**

**Given** my real database
**When** I trigger «Imprimer ma semaine (brut)»
**Then** a dated plain-text file lists the week's tasks by day then bed (tour-de-plaine), states ☐/☒/barré, bed+crop per line, saved via the export ritual
**And** the PrintDoc DTO is documented + versioned in the glossary; the harness asserts its shape
**And** the harness gains: facts → virtual PrintDoc → projection assertions (the E1→E4 oracle).

### Story 1.5: Skip and correct from the task views

As the grower,
I want to skip (closed-set reason, optional note) and explicitly correct done/skipped states from the existing tasks screen,
So that deliberate non-work is honest today.

**Acceptance Criteria:**

**Given** the tasks screen
**When** I «Abandonner» with a reason
**Then** the row strikes through with its reason badge and leaves future-facing lists; `is_overdue` never fires on skipped
**And** «Corriger» reopens any settled state via a correction fact — never silently
**And** entry errors are fully correctable from this view alone (clause 3); fr+en strings.

### Story 1.6: Calendar, agenda and detail honor the new states (FR49)

As the grower,
I want the calendar (struck/greyed, holiday-pattern reuse), agenda (skipped absent from upcoming) and planting detail coherent on done/skipped/corrected,
So that every surface tells the same story.

**Acceptance Criteria:**

**Given** mixed task states
**When** each view renders
**Then** the single shared visibility predicate decides everywhere (usage-searched)
**And** skipped never counts as done in aggregates; bounded agenda (#69) never resurrects past skipped.

### Story 1.7: I1–I3 property tests and dogfooding database compatibility

As the test architect,
I want proptest fact-sequence generators (shared module, reused later) with I1–I3 under kill-injection, plus a migrate+smoke test against a sanitized copy of the owner's real database,
So that invariants are machine-checked and the dogfooding base survives schema changes.

**Acceptance Criteria:** **Given** generated sequences **When** proptests run with kill-injection **Then** I1–I3 hold; prefix-replay yields prefix state **And** the real-database runbook (local, CI-ignored) is documented and executed once per schema change.

## Epic 2: Planifier les cultures — lignes de plan et ITK

Winter plan enters spreadsheet-style; ITKs on crops; staggered plantings; needs on screen. (Migration 0008.)

### Story 2.1: Plan-line persistence (migration 0008)

As the product owner,
I want `CropPlanLine` (crop/variety, series × bed-meters, staggering, draft state, notes) persisted on both backends,
So that the plan exists as durable data the grid and generation can build on.

**Acceptance Criteria:** **Given** `0008_planning.sql` (plan + itk tables, additive) **When** cross-backend tests run **Then** CRUD round-trips both backends; `copy_all` covers the tables **And** constructors enforce positive series/meters, staggering ≥ 0, draft orthogonal to validity.

### Story 2.2: ITK templates on crops

As the grower,
I want an itinéraire technique per crop — ordered activities {task_type, signed offset_days, optional method/implement (revived FKs), label, notes},
So that generation reflects how I actually grow each crop.

**Acceptance Criteria:**

**Given** a crop
**When** I define J-10 «préparation planche» and J+20 «désherbage»
**Then** the ordered template persists and round-trips both backends, on the dormant FKs (no parallel columns)
**And** an ITK-less crop keeps the shipped variety-profile autogen (clause 5 — fallback tested).

### Story 2.3: The plan grid — entry

As the grower,
I want a spreadsheet-like grid (arrows, type-to-edit, Enter advances, typed cells validated on exit, **derived date columns** grey/read-only),
So that entry feels like Calc with safety.

**Acceptance Criteria:**

**Given** `plan.slint` wired via `wiring/plan.rs`
**When** I edit keyboard-only
**Then** editable cells accept locale decimals, refuse invalid inline (cell stays open, Échap restores)
**And** derived columns are limited to computed dates in this story (needs figures arrive with 2.7), rendered distinct and non-focusable
**And** field-state grammar + fr/en throughout.

### Story 2.4: Duplication, drafts and session resume

As the grower,
I want Ctrl+D duplication, visible draft state, and reopening on the last edited line,
So that 60 lines cost evenings, not weeks.

**Acceptance Criteria:** **Given** a complete line **When** Ctrl+D **Then** a copy opens in edit on the first editable cell **And** cells persist on exit (no Save; kill loses at most the open cell) **And** reopening focuses the last edited line **And** drafts are excluded from generation/needs but visibly marked.

### Story 2.5: The ITK editor screen

As the grower,
I want the ITK editor on the crop's page (Cultures master-detail): ordered activity list, add/reorder/offset editing,
So that ITK configuration lives where crops live.

**Acceptance Criteria:** **Given** a crop detail **When** I open «Itinéraire technique» **Then** activities render as «J-10 / J+20», editable via existing form patterns **And** method/implement pickers appear only when entries exist **And** deleting an activity referenced by pending generated tasks warns naming the count (data-danger red).

### Story 2.6: Generation — plan line to staggered plantings

As the grower,
I want a complete line to generate its N staggered plantings (planned, unplaced),
So that my plan materializes without placement.

**Acceptance Criteria:**

**Given** «Laitue Batavia — 6 × 15 m, 14 j»
**When** I generate
**Then** 6 plantings exist, stagger-dated, line-linked, unplaced, listed
**And** regeneration after edit is non-destructive for active plantings
**And** the harness gains the planning dataset.

### Story 2.7: The needs list — on screen

As the grower,
I want aggregated needs (variety quantities + buy-by deadlines, backward-computed) from all non-draft lines, placed or not,
So that I can order in January before placing.

**Acceptance Criteria:** **Given** dated lines **When** I open «Besoins» **Then** per-variety aggregation with earliest buy-by, exact Decimal; unplaced included **And** the grid's derived needs figures activate here **And** printing deferred to E4 (disabled-with-tooltip, honest).

## Epic 3: Placer — géométrie et capacité

Live cover-aware capacity at placement; ITK tasks at placement; retro-entry safe; perennial death frees ground. (Migration 0009.)

### Story 3.1: The pure capacity engine (with migration 0009)

As a contributor,
I want `capacity.rs` — occupancy over `[start,end)`, covered/open split, recursive hierarchy aggregation, peak composition — as pure functions on `date_calc.rs`, with the `occupation_kind` additive column and the documented bed-meters rule,
So that capacity is exact, testable, UI-independent.

**Acceptance Criteria:**

**Given** `0009_geometry.sql` (additive; codec + cross-backend + copy_all covered) and placement inputs
**When** the algebraic proptests run
**Then** superposition, commutativity, monotonicity, adjacent-non-overlap, horizon-extension stability, translation invariance, hierarchy coherence at sampled t, ±50-year retro-entry all hold
**And** module coverage ≥ 95%; existing locations audited against the geometry rule.

### Story 3.2: The placement screen with the live curve

As the grower,
I want to assign unplaced plantings to beds (tree) and watch the curve react, covered/open apart,
So that I feel the constraint at placement.

**Acceptance Criteria:**

**Given** unplaced plantings
**When** I place one
**Then** the curve updates **within one frame at farm scale (≤100 ms for ≤500 placements — measured in a perf test)**; overflow shows amber with the peak value
**And** clicking a peak lists composing series (basic; full PeakPanel in E7)
**And** placement freely undoable while not active.

### Story 3.3: Tasks generate at placement

As the grower,
I want ITK activities (incl. J-negative preparation) becoming dated tasks when a planting is placed,
So that the plan becomes dated work.

**Acceptance Criteria:**

**Given** a placed planting whose crop has an ITK
**When** generation runs
**Then** each activity yields a task (anchor+offset); pre-establishment tasks land before the establishment date; ITK-less crops fall back to profile autogen
**And** the skip-aware guard holds across re-placement (no resurrections, no duplicates).

### Story 3.4: Retro-entry and perennial lifecycle

As the grower,
I want decades-old perennials to enter `active` with zero past tasks (explicit reassurance line), and termination to free occupancy at its date,
So that my 1996 orchard is safe and a dead bush stops haunting the curve.

**Acceptance Criteria:**

**Given** an establishment date decades past
**When** the planting is created
**Then** zero past tasks; the confirmation states «aucune tâche passée ne sera créée ; prochaines tâches : …»
**And** terminating a perennial ends its occupancy (engine proptest extended)
**And** the harness gains the perennial + capacity datasets.

## Epic 4: La feuille dans la poche — les documents

PrintDoc rendering real; the journal sheet prints; goldens born; paper dogfooding.

### Story 4.1: PrintDoc render infrastructure

As a contributor,
I want `print/render.rs` (printpdf 0.9, embedded DejaVu, A4 metrics) rendering any PrintDoc to PDF, plus `documents_dir` config (XDG Documents default) and dated filenames,
So that every document shares one pipeline the owner's exports depend on.

**Acceptance Criteria:** **Given** a PrintDoc fixture **When** rendered **Then** the PDF parses, page count matches logical breaks, extracted text has sentinels, fonts embedded **And** render coverage-exempt in `ci.yml` (contract tests only) **And** `documents_dir` serde-defaulted, created on first export.

### Story 4.2: The journal sheet — the hero document

As the grower,
I want the multi-day journal sheet per the A4 mock: per-page header (farm, period, print date, version, glossary version, freshness, pagination), dated day columns tour-de-plaine ordered, checkbox lines with bold bed+crop, **«qté: ...» blanks on harvest lines (pencil capture; on-screen ingestion: perennials only in R1 — FR38 decision)**, notes zones, ≤6-symbol legend incl. handwriting convention,
So that my pocket carries a trustworthy form.

**Acceptance Criteria:**

**Given** a placed, task-bearing week
**When** the sheet builds
**Then** the PrintDoc equals the frozen E1 contract + rendering enrichments; insta goldens exist FR and EN
**And** done stays done and skipped absent on re-print (harness re-print step live — paper oracle)
**And** reserved slots for E5 reconciliation mentions exist (clause 4)
**And** the photocopy legibility protocol is executed, **dated and archived** in the repo (docs/design/photocopy-test-YYYY-MM-DD.md).

### Story 4.3: The export ritual and the needs list on paper

As the grower,
I want every export following the ritual (dated filename, persistent status line with full path, «ouvrir») and the needs list printing through the pipeline,
So that January orders leave the screen.

**Acceptance Criteria:** **Given** the E2 needs view **When** I export **Then** the PDF lands per ritual; needs PrintDoc has FR/EN goldens; no blocking dialogs.

### Story 4.4: Weekly ritual switchover

As the product owner,
I want the E1 rough print replaced by the real sheet (one code path), contract continuity harness-asserted,
So that dogfooding upgrades without a gap.

**Acceptance Criteria:** **Given** both renderers **When** switchover completes **Then** the E1 entry point emits the real document; virtual-contract tests pass unchanged **And** the harness runs print → re-print on goldens.

## Epic 5: Boucler la semaine — le corridor

≤15 min a week, keyboard-only, interruptible, free lines; the full loop becomes the blocking gate.

### Story 5.0: GATE — the Thursday-evening paper test

As the product owner,
I want the wizard-of-Oz paper test executed before any corridor code: one real week from my notebook transcribed onto a printed E4 sheet, reconciled by me with a pencil on a real Thursday evening, timed discreetly,
So that the biggest bet (will I record reality at all, tired?) is validated for the price of a print.

**Acceptance Criteria:**

**Given** a real E4-printed sheet with my real week
**When** the test runs after invoicing on a Thursday
**Then** the outcome is recorded in `docs/design/thursday-test-YYYY-MM-DD.md`: duration, skipped lines, verbal reactions
**And** success (<10 min and «c'est tout ?») green-lights 5.2+; invalidation triggers a corridor design review (incl. the SheetMiniMap option) before any Slint code
**And** the decision (proceed/revise) is explicit in the document.

### Story 5.1: Reconciliation view-model and contextual landing

As the grower,
I want the since-last-entry batch view-model (sheet order, day groups, resume position) and contextual landing (empty→onboarding stub, pending→corridor, off-season→**planning stub until E7**),
So that opening the app is resuming work.

**Acceptance Criteria:**

**Given** unreconciled tasks
**When** the app opens
**Then** the corridor state lists them day-grouped, bed-ordered, cursor on first unreconciled (position persisted)
**And** view-model logic is pure `pomone-app` functions, unit-tested
**And** landing rules data-driven, tested for the three states (off-season target explicitly a stub)
**And** **NFR1 measured: cold start to interactive corridor < 3 s on the reference machine (perf test, documented protocol)**.

### Story 5.2: The corridor screen — navigation and focus

As the grower,
I want the `reconcile.slint` corridor (Direction B): one centralized FocusScope, current-line highlight, arrows, manual scroll-into-view, CounterFoot legend,
So that the screen is fully keyboard-driven.

**Acceptance Criteria:**

**Given** a populated corridor
**When** I navigate arrows-only
**Then** the current line stays visible and highlighted
**And** focus restitution is verified by a **documented manual test protocol** (checklist: after each edit/strip/field, keys still respond — executed and archived per release)
**And** line-local `set_row_data` only (handle kept by `wiring/reconcile.rs`); sprint density (rows 34–36px); shortcuts at foot.

### Story 5.3: The gestures — done, skip, carry over, correct

As the grower,
I want ␣ (done, day-group date; quantity extension on **perennial** harvest lines), X + inline 1–4 reasons, R + DateField (closed grammar, live echo), ↵ reopen/correct, Échap cancel,
So that a line costs one decision.

**Acceptance Criteria:**

**Given** the corridor
**When** each gesture fires
**Then** a fact records synchronously per line; kill mid-list loses nothing (harness corridor kill step live)
**And** **NetworkDrop is re-exercised here**: on MariaDB, an unreachable server fails the gesture visibly, the line stays pending, no local corruption (harness NetworkDrop step in corridor)
**And** reason strip inline (no PopupWindow); Échap cancels any in-progress gesture; aging lines wear the amber edge.

### Story 5.4: Free lines and the prodigal return

As the grower,
I want «+ ligne libre» (text + date ≤10 s, optional one-gesture crop/bed attachment) and week-grouped bulk acceptance with stale-series propositions,
So that unplanned work is first-class and long absences reconcile bounded.

**Acceptance Criteria:** **Given** the corridor **When** I press + **Then** text+date commit in one Enter; optional attach picker skippable (clause 6) **And** multi-week backlogs group with one-gesture bulk accept, individually overrideable **And** stale propositions explain, never auto-act.

### Story 5.5: I4–I6 interleaving proptests

As the test architect,
I want I4–I6 state-machine proptests (backdating never corrupts settled periods; reconciliation convergent and idempotent; order-insensitive within the batch) reusing E1's generators enriched with backdating,
So that the corridor's correctness is machine-checked (clause 4).

**Acceptance Criteria:** **Given** E1's shared generators + backdating strategies **When** interleaved sequences (autogen ∘ reconciliation ∘ edition) run **Then** I4–I6 hold; failures shrink to minimal counterexamples; generators live in the shared module (no duplicates).

### Story 5.6: Chrono, week close and the printed reward

As the grower,
I want the in-app chrono (first gesture → close; pauses on blur/2-min idle), the always-available «Boucler», and the closing invitation printing next week's sheet,
So that the loop closes and measures itself — locally.

**Acceptance Criteria:**

**Given** a session
**When** I close the week
**Then** ↵ renders next week's sheet (reconciliation mentions filling E4's reserved slots); FreshnessLine resets
**And** the chrono persists locally; 2 consecutive weeks >15 min → local neutral notice, zero telemetry
**And** **NFR2 verified as budgets, not human minutes**: per-gesture latency ≤ 50 ms (perf test) and S-40/S-april scripted at ≤ 15/22 machine-gestures-per-line-average respectively, with the conversion budget (gesture cost × count ≤ 10/15 min) documented — **the owner's weekly dogfooding journal (docs/design/dogfooding-journal.md) is the named human oracle**.

### Story 5.7: The complete paper loop — release-blocking

As the test architect,
I want the full weekly loop (print → simulated week incl. skipped reasons + free lines → batch backdated reconciliation + kill + resume → re-print → second loop) green and marked release-blocking in CI,
So that «the paper is always right» is enforced by the pipeline, forever.

**Acceptance Criteria:** **Given** all prior harness steps **When** the loop runs both loops **Then** done stays done, skipped never reappears, no line lost/duplicated, loop-2 goldens match **And** the job is required in branch protection **And** the harness `// TODO` markers are all resolved or explicitly moved to E6/E7 extensions.

## Epic 6: Les registres

*(Epic DoD: goldens of both documents join the harness re-print cycle — same rule as 7.2; manual updated.)*

### Story 6.1: The crop list document

As the grower,
I want the planned/in-progress crop list printable (cultures, varieties, series, placement, states),
So that the second field document exists.

**Acceptance Criteria:** **Given** a populated plan **When** the document builds **Then** FR/EN goldens exist and **join the harness re-print cycle**; export ritual; shared visibility predicate governs states; manual section updated.

### Story 6.2: The treatment register and its CSV

As the grower,
I want the phytosanitary register printable (one line per treatment: crop, bed, date, product, dose, quantity-when-known; gaps listed, never omitted) and the raw CSV export (D7 contract),
So that the June-2027 inspection is a print button with a machine-readable twin.

**Acceptance Criteria:**

**Given** recorded treatments
**When** the register prints for a period
**Then** all legally required fields appear; missing quantities explicitly listed; FR/EN goldens join the re-print cycle; decade fixture prints (retention)
**And** the CSV follows `docs/export-contracts.md` (UTF-8 BOM, semicolon, ISO, dot decimals, pomone_version) with a golden fixture; both follow the export ritual.

## Epic 7: Comprendre et arbitrer

### Story 7.1: The PeakPanel and the armchair landing

As the grower,
I want clicking a peak to open the explainer (amber header, composing series with candidate badges, «la décision t'appartient» — shift/move/cut acting as plan edits), and the off-season landing to surface the year plan + any standing overflow,
So that the January arbitrage happens on screen, with facts.

**Acceptance Criteria:**

**Given** an over-capacity April
**When** I click the peak
**Then** the panel lists the exact composing series (engine composition query) with shiftable/open-field/cut affordances
**And** each action performs a plan edit and refreshes the curve — never auto-acts; keyboard-complete; Échap closes
**And** the off-season landing activates (replacing 5.1's stub) with the overflow line linking to the panel.

### Story 7.2: The occupancy map document

As the grower,
I want the bed-occupancy map printable (hatched/solid per period, ≥40% greys, legend),
So that the fourth field document completes the set.

**Acceptance Criteria:** **Given** placements **When** the map builds **Then** FR/EN goldens **added to the harness re-print cycle** (DoD clause); B&W photocopy protocol executed, dated, archived; export ritual.

## Epic 8: Accueillir

### Story 8.1: Locked demo mode and single instance

As a newcomer,
I want «Explorer la démo» opening a separate local demo database (seeded farm incl. plan lines + ITKs), permanent banner, swap/backup/restore locked — and the app refusing a second instance politely,
So that I can play fearlessly and never corrupt anything.

**Acceptance Criteria:**

**Given** any configured backend
**When** demo enters
**Then** `pomone-demo.sqlite` is used (always local), state in memory, banner visible, locks enforced (localized sentinels); leaving restores the real repository; real data untouched (test)
**And** seed is a `pomone-app` API used by CLI and UI
**And** **NFR9: a second instance on the same database is refused with a friendly message (flock on pomone.lock; test)**.

### Story 8.2: First-run onboarding

As a newcomer,
I want the empty-base landing offering demo or «Créer ma ferme», with the honest no-import notice and the fast re-entry path (first crop → variety → line),
So that the blank-page wall never appears.

**Acceptance Criteria:** **Given** an empty base **When** the app opens **Then** the onboarding path per pattern (never an empty corridor); no-import wording per PRD; families pre-seeded; path reaches a printable plan skeleton.

### Story 8.3: The 30-minute path, measured

As the product owner,
I want the Marie scenario scripted end-to-end (first launch → first printed plan) with a **documented step budget** (modeled action costs summing ≤ 30 min), exercised in FR **and EN — including a runtime language switch mid-scenario re-rendering screens and the printed document in the new language (FR45)**,
So that NFR3 and FR45 are measured, not hoped.

**Acceptance Criteria:** **Given** a fresh install **When** the scripted scenario runs **Then** the step budget holds; frictions fixed or ticketed **And** the mid-scenario language switch produces a correctly localized screen set and PDF.

### Story 8.4: The getting-started manual

As a newcomer,
I want the LaTeX manual's getting-started chapter («de la graine à la récolte») covering the R1 loop (plan → place → print → reconcile), plus per-epic manual sections verified,
So that F1 answers a lost newcomer (and the per-epic manual DoD is settled).

**Acceptance Criteria:** **Given** the R1 feature set **When** the manual compiles **Then** the getting-started chapter walks the full loop with the glossary's terms; every R1 epic's sections exist (audit checklist); PDF embedded as shipped.
