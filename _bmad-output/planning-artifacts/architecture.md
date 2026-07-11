---
stepsCompleted: [1, 2, 3, 4, 5, 6, 7, 8]
lastStep: 8
status: 'complete'
completedAt: '2026-07-11'
inputDocuments:
  - _bmad-output/planning-artifacts/prd.md
  - _bmad-output/planning-artifacts/implementation-readiness-report-2026-07-11.md
  - _bmad-output/project-context.md
  - docs/architecture.md
  - docs/analyse/qrop-vs-pomone.md
  - docs/roadmap.md
workflowType: 'architecture'
project_name: 'pomone'
user_name: 'Guy'
date: '2026-07-11'
---

# Architecture Decision Document

_This document builds collaboratively through step-by-step discovery. Sections are appended as we work through each architectural decision together._

## Project Context Analysis

### Requirements Overview

**Functional Requirements (49 FRs — architectural reading):**

R1 decomposes into **Slice 0 + seven work packages**. Two are genuinely new engines; the rest extend proven patterns of the existing 5-crate stack:

- **Slice 0 — Foundation (no behavior change):** refactor `pomone-ui/main.rs` (5500+ lines) into per-screen wiring modules; introduce request structs in `services.rs` (current constructors at arity limit); audit SQLite CHECK constraints in migrations 0001–0006; decide the dormant `Task.task_method_id`/`implement_id` FKs (revive for ITK or explicitly retire); stand up the **empty paper-loop test harness** (kill/replay runner, XDG-isolated DB, driver at `pomone-app` level — CI-blocking from day one).
- **P1 — Planning domain (FR1–FR8):** `CropPlanLine` + ITK activity-template aggregates, generation plan-line → plantings → tasks extending `task_autogen.rs`. Pure domain + persistence. ITK doses reuse the shipped `units.rs` patterns (display units merged 2026-07-10).
- **P2 — Location geometry (prerequisite of P3):** bed-meters and covered/open attributes on the location model (`covered` exists on `LocationKind`, `length_m` on `Location` — audit, then additive migration if gaps), `locations_view` + edit form updates. Extracted so the capacity engine stays pure math.
- **P3 — Capacity engine (FR9–FR17):** pure functions over placements and `[start,end)` windows — occupancy, covered/open split, recursive hierarchy aggregation, explainable peaks, perennial-to-horizon. Built on `date_calc.rs` (never duplicated). **New engine #1.**
- **P4 — Facts & states (FR18–FR28, merged):** one mutation model, not two. Every field gesture (done, skipped, terminate, treatment confirmation) is an **immutable, idempotent fact** — dual timestamps, explicit-correction semantics — and task/planting states are projections of facts. Batch reconciliation is *an entry surface* over this model; a future mobile client is just a second emitter. Contains: additive skipped columns, `occurred_at`/`recorded_at`, hardened autogen idempotence guard (skip-aware, replan-aware).
- **P5 — Document engine (FR29–FR36):** **new engine #2** and the only new external dependency (pure-Rust PDF crate). Pipeline: `*_view.rs` DTOs → **`PrintDoc`** (localized, formatted strings and *logical page breaks* inside — the insta-snapshot boundary, FR+EN goldens) → PDF bytes (contract tests only, never snapshotted) → saved file. Genericity: a new compliance register = a new view over the same pipeline.
- **P6 — Demo mode (FR44):** separate local SQLite file (`pomone-demo.sqlite`), in-memory demo state (never persisted in config), ignores the configured backend, **locks out swap_backend/backup/restore while active**; `seed` promoted from CLI debug tool to `pomone-app` API used by both CLI and UI.
- **P7 — Entry ergonomics (FR2–FR3, FR23, FR47):** draft persistence, session resume, no-Save line persistence, catch-up start screen.

**Non-Functional Requirements (27 — the architecture-shaping ones):**

- **NFR6/NFR7 (nothing lost on kill; invariants I1–I6):** line-level write-through, per-gesture transactions. SQLite: WAL. **MariaDB: this guarantee is not free** — server durability and network availability are outside our control; durability posture per backend is an explicit decision (local fact-journal/outbox vs documented degraded guarantee).
- **NFR20–NFR25 (quality gates):** pure-function engines keep 80/95% reachable; the PDF *render* layer needs an explicit coverage policy (precedent: MariaDB backend counted 0% by design).
- **NFR8 + additive-only migrations:** «additive at domain level» ≠ «additive at SQL level» where CHECK constraints exist.
- **NFR11 (zero outbound):** PDF generation fully local, fonts embedded.
- **NFR1/NFR5:** startup and document-generation budgets to test; no architectural change expected.

**Scale & Complexity:** medium-high, driven by *correctness density* (state machines, capacity math, deterministic documents, dual-backend parity), not volume. Estimated: 4–5 migration pairs (**× 8 touchpoints each**: SQL×2, codec, sub-trait, impl×2, cross-backend tests, seed), ~5 new Slint screens (3-layer plumbing + FR/EN keys + testable view-models), 2 pure engines, 1 facts model.

### Technical Constraints & Dependencies

- **Fixed stack:** Rust 2021/MSRV 1.80, 5-crate layering, sqlx dual-backend, Slint software renderer, Fluent, `Decimal`/`NaiveDate`, no SQL triggers, additive migrations.
- **New dependency decision:** PDF crate — `printpdf` (low-level, own layout), `genpdf` (layout, drowsy maintenance), `typst`-as-lib (great layout, heavy compiler + fonts). Deferred behind the frozen `PrintDoc` contract, which makes it reversible. **Must be pure-Rust and cross-platform** (Windows/macOS ports are post-v1 but committed — no fontconfig/cairo dependencies).
- **Brownfield seams (complete list):** `task_autogen.rs` idempotence guard (riskiest — the skip-resurrection mechanism); `codec.rs` (×4 multiplier per enum variant); 3-layer Slint plumbing; `cross_backend_tests.rs`; **`swap_backend`/backup coverage for every new table**; **i18n seam** — Fluent catalogs load in `pomone-ui` today, but PrintDoc needs localized strings in `pomone-app`: where I18n lives in the document pipeline is an explicit decision; SQLite CHECK constraints; `services.rs` arity; `date_calc.rs` as foundation; `config.rs` (documents directory, optional-with-default).

### Cross-Cutting Concerns Identified

1. **The «fact» schema is the mother decision** — immutable record shape, idempotency key, dual timestamps, correction mechanism. P1/P4/P5 and the printed registers are its consumers. **It must accommodate R2 fact types (observations, weather, treatment quantity confirmations) without new machinery** — an open fact-type + payload, or R2 re-migrates what R1 froze too narrow.
2. **Durability posture per backend** — may reshape the fact schema; decide second.
3. **Printability as a view-layer property** — one filter predicate (e.g. skipped-exclusion), one place; PrintDoc carries localized+formatted content.
4. **FR/EN parity** — every new label, document string, enum display; CI-gated.
5. **Test architecture as delivery driver:** walking-skeleton paper loop grows with every slice (DoD: each slice extends the loop by a step or a dataset); capacity engine under algebraic property tests (superposition, horizon-extension stability, hierarchy coherence at every t, ±50-year retro-entry); **severest review on the facts/states package** — its invariants are interleaving properties (autogen ∘ reconciliation ∘ edition), state-machine-tested, deceptively small-looking.

### Decision Queue for Step 4 (order matters)

1. **Fact schema** (idempotency key, dual timestamps, correction mechanism, open to R2 fact types) — unblocks everything.
2. **Durability posture per backend** (WAL vs local outbox vs degraded-documented for MariaDB).
3. **Location geometry** for capacity (units, covered split, migration audit).
4. **`PrintDoc` contract** (pagination + localization inside the snapshot boundary), then PDF crate (pure-Rust, cross-platform), then render-coverage policy.
5. **Events persisted vs derived** — explicit, with rationale.
6. **ITK template fields** + revive/retire `task_method_id`/`implement_id` (readiness gap #2); bed-geometry formalization (gap #3).

## Starter Template Evaluation

### Primary Technology Domain

Native desktop application (Rust workspace) — **brownfield**: the foundation is the existing production codebase, not a starter.

### Starter Options Considered

None applicable. The «starter» decisions a template would normally make are already made, shipped, and CI-enforced by the existing workspace:

| Decision a starter would make | Already established by |
|---|---|
| Language & toolchain | Rust 2021, MSRV 1.80, cargo workspace, `resolver = "2"` |
| Project structure | 5-crate strict layering (`domain ← db ← app ← ui/cli`) |
| Persistence | sqlx 0.8 dual-backend behind `Repository` trait, embedded migrations |
| UI framework | Slint 1.8, software renderer, 3-layer plumbing convention |
| i18n | Fluent, `locales/{fr,en}/main.ftl`, key parity CI-checked |
| Testing | proptest, rstest, insta, testcontainers; 80% gate; cross-backend suite |
| Lint/format | clippy pedantic, `-D warnings`, rustfmt, `unsafe_code = deny` |
| CI/packaging | GitHub Actions Linux, `.deb` + AppImage via cargo-packager |

### Selected Starter: the existing Pomone workspace (`main`)

**Rationale:** phases 0–9 shipped a tested, documented foundation; the PRD explicitly scopes a *convergence on top of it*, preserving its additions. Any architecture decision that contradicts the established stack rules (project-context, 34 rules) is invalid by default.

**Initialization command:** none — first implementation story starts from `git switch -c <slice-0-branch>` on `main`.

**The single new-dependency decision** (PDF crate — the only «starter-like» choice left) is deliberately deferred to Step 4, behind the frozen `PrintDoc` contract, with pure-Rust + cross-platform as hard criteria.

**Dependency freshness note:** the workspace tracks three known transitive advisories (RUSTSEC-2026-0192/0194/0195 — Slint/wayland/resvg, no upstream fix available), explicitly ignored in `deny.toml` and re-checked by `cargo-deny` in CI. No dependency upgrade is required for the convergence work; the PDF crate will be the only addition, chosen at Step 4 with maintenance status as a criterion.

## Core Architectural Decisions

### Decision Priority Analysis

**Critical (block implementation):** D1 fact schema, D2 durability posture, D3 location geometry, D4 PrintDoc contract + PDF crate.
**Important (shape architecture):** D5 ITK template fields, D6 demo mode, D7 export conventions.
**Deferred (post-R1, consciously):** MariaDB outbox (D2 fallback), full Type→Method→Implement editing UI (R2), treatment-dose modeling in ITK (R2, reuses `units.rs`).

### D1 — Fact schema (the mother decision) — **Hybrid: readable states + append-only fact journal**

Entities keep their readable state columns (existing `completed_on`, plus additive `skipped_on`/`skip_reason`/`skip_note`) **and** every field gesture also writes one row into a new append-only `field_event` table — same transaction:

`field_event { id: UUID (client-generated = idempotency key), kind, target_kind, target_id, occurred_at: DATE, recorded_at: DATETIME, payload: TEXT/JSON, corrects: Option<event_id> }`

- States remain the simple read source (views, printing); the journal provides idempotence (re-apply = unique-constraint no-op), audit, crash-exact-prefix semantics, **open fact kinds for R2** (observation, weather, treatment-quantity) and the anchoring point for a future mobile emitter.
- Correction = a new event with `corrects` set — never a silent update; the projection updates the state columns accordingly.
- Explicitly answers «events persisted or derived?»: **persisted, alongside states** (rejected: pure event-sourcing — would force rewriting all existing views as projections; pure mutable states — cannot honor exact-prefix-on-crash and idempotent replay cleanly).

### D2 — Durability posture per backend — **Full on SQLite, documented-degraded on MariaDB**

- SQLite: WAL mode + one transaction per gesture → the kill/replay guarantee (NFR6) holds fully.
- MariaDB: same per-gesture transactions; durability depends on server config and network. **A gesture against an unreachable server fails visibly** — no silent local queueing in R1. NFR6 is annotated accordingly.
- The `field_event` table is already half of a future outbox if LAN reality ever demands the full guarantee (post-R1).

### D3 — Location geometry — **formalize, don't migrate**

Audit result: `Location.length_m × width_m` and `LocationKind.covered` already exist. R1 capacity footprint = **bed-meters = `length_m` of leaf (bed) locations**; covered/open split from `LocationKind.covered`; width stays informative. One additive column: `occupation_kind` (single value `bed-meters` in R1) as the discriminant for future polymorphism (tree rows, hectares) without debt. *(Closes readiness gap #3.)*

### D4 — PrintDoc contract first, then PDF crate — **`printpdf` 0.9.x under our own layout**

- **`PrintDoc`** (new `pomone-app/print/` module): the logical *and final* document model — strings already localized and formatted (`pomone-app`'s existing `I18n` serves the pipeline; the UI is not in the printing loop), **logical page breaks included** (unbreakable day-groups, injected A4 metrics), dated/versioned headers, serializable for insta goldens (one per locale FR/EN).
- **PDF crate: `printpdf` 0.9.x** — actively maintained (0.9 released 2026-01), pure Rust, cross-platform (no fontconfig/cairo), basic layout + auto page-breaking sufficient since *our* pagination is computed in PrintDoc. Rendered by a thin `print/render.rs`.
- **Coverage policy:** the PDF render layer is excluded from the coverage gate (MariaDB-backend precedent) — contract tests + smoke tests only, never byte goldens; everything up to PrintDoc carries the 95% target.
- Rejected: `genpdf` (unmaintained ~3 years — unacceptable on a 10-year horizon), `typst`-as-lib (full compiler, font embedding surface, moving API).
- **Default documents directory:** `XDG_DOCUMENTS_DIR/pomone/` (user-visible, habit-backed-up), configurable in Settings, created on first export; dated filenames.

### D5 — ITK template fields *(closes readiness gap #2)*

An ITK template belongs to a **crop** (variety-level override possible later):
- ordered list of **activities**: `{ task_type_id (existing FK), offset_days: i32 (signed, anchored on establishment), optional label, optional notes }`;
- the dormant **`task_method_id`/`implement_id` FKs are revived**: optional on template activities (no parallel columns — reuse schema 0001–0006); UI exposes them only when methods/implements exist (full taxonomy editing = R2);
- no doses in R1 ITKs (planned treatments are R2; will reuse `units.rs` + entered quantities per the PRD).

### D6 — Demo mode

Separate local `pomone-demo.sqlite`; demo state held in memory in `App` (never persisted to config); ignores the configured backend (demo is always local SQLite); **locks out swap_backend/backup/restore while active**; visible UI banner; `seed` promoted from CLI tool to a `pomone-app` API consumed by both CLI and UI.

### D7 — Export conventions (the stable contracts of NFR26)

CSV: **UTF-8 with BOM** (FR Excel/LibreOffice survival), **semicolon separator** (FR/CH locale convention where comma is decimal), **ISO-8601 dates**, **dot-decimal numbers** (machine contract, locale-free), stable documented header row + a `pomone_version` column. Contracts documented in `docs/export-contracts.md`, versioned with the code. Single-instance mechanism (NFR9): file lock (`flock` on `pomone.lock` next to the database) — releases on kill, friendly message when held.

### Decision Impact Analysis

**Implementation sequence:** Slice 0 (wiring refactor, request structs, CHECK audit, empty paper-loop harness) → D1+D2 (fact journal + states, inside the walking skeleton) → D5/P1 (ITK + plan lines) → D3/P2–P3 (geometry formalization + capacity engine) → D4/P5 (PrintDoc → PDF) → D6–D7/P6–P7.

**Cross-component dependencies:** D1 shapes P1 (plan-line generation events), P4 (all gestures), P5 (registers print facts) and the R2 ledger; D2 annotates NFR6; D3 unblocks the capacity engine's pure functions; D4's PrintDoc boundary defines the paper-loop's testable «print» step long before any PDF exists; D5 reuses existing FKs, avoiding one migration pair.

## Implementation Patterns & Consistency Rules

*(The 34 project-context rules govern everything below; these patterns fix the NEW divergence points introduced by decisions D1–D7.)*

### Naming Patterns

- **New tables** (existing singular snake_case convention): `crop_plan_line`, `itk_template`, `itk_activity`, `field_event`; columns snake_case, FKs `<entity>_id`, **no CHECK constraints** (invariants live in domain constructors). Next migration pair: `0007_*`.
- **Fact kinds (D1):** dot-namespaced snake strings, entity-first — `task.done`, `task.skipped`, `task.corrected`, `planting.activated`, `planting.terminated`, `treatment.confirmed`; R2: `observation.noted`, `weather.noted`. Encoded via `codec.rs`, identical literals on both backends.
- **Payload JSON** (`field_event.payload`): snake_case keys; **ids and values only, never denormalized labels** (labels resolve at read time); dates as ISO-8601 strings; decimals as strings, never JSON floats.
- **Fact identity & ordering:** `field_event.id` = client-generated **UUID v4** (workspace convention); chronological order comes from `recorded_at`, never from the id or insertion order (two future emitters must not fight over a sequence).
- **Fluent keys:** existing conventions hold; new print-document keys prefixed `print-*`; every key in both `fr` and `en`.

### Structure Patterns

| New code | Lives in | Rule |
|---|---|---|
| `CropPlanLine`, ITK aggregates, capacity engine, fact types | `pomone-domain` — one file per aggregate: `crop_plan.rs`, `itk.rs`, `field_event.rs`, `capacity.rs` | Capacity = **pure functions only** — no `Repository`, built on `date_calc.rs` |
| Fact recording, projections, reconciliation service | `pomone-app/services.rs` + new `facts.rs` | Single entry point: `record_fact(&dyn Repository, Fact) -> AppResult<...>` |
| PrintDoc + builders + render | `pomone-app/print/` — `mod.rs` (PrintDoc model), one builder per document (`weekly_sheet.rs`, `crop_list.rs`, `occupancy_map.rs`, `treatment_register.rs`), `render.rs` (printpdf, excluded from coverage gate) | Builders consume `*_view` DTOs + `I18n`; render consumes only PrintDoc |
| Screen wiring (post Slice-0 refactor) | `pomone-ui/src/wiring/<screen>.rs` — `fn wire_<screen>(…)` | New screens NEVER add callbacks to `main.rs` directly |
| Paper-loop harness | `crates/pomone-app/tests/paper_loop.rs` (integration, SQLite, XDG-isolated) | Drives services/view-models only — never Slint |
| Proptest suites | Next to their engine; interleaving state-machine tests in `pomone-app/tests/fact_invariants.rs` | Insta snapshots: `print/snapshots/<document>.<locale>.snap` |

### Process Patterns

- **The gesture pattern (D1) — every field mutation goes through it:** caller builds a `Fact` (client UUID, kind, target, `occurred_at`, payload) → `record_fact` opens ONE transaction: insert into `field_event` (UNIQUE(id) → conflict = idempotent no-op returning the existing result), apply the projection to state columns, commit. **No service ever mutates a state column outside this path.** Correction = new fact with `corrects`; replanning is a *plan* mutation, never a fact.
- **Time injection:** the existing injected-`today` pattern extends to `recorded_at` — passed by the caller (UI/CLI layer captures the clock), never `Utc::now()` inside `pomone-domain` or `services.rs`. This keeps the paper-loop and kill/replay harness deterministic and honors «every factual date entered explicitly».
- **The single-predicate rule:** «is this task visible on future lists/sheets?» is ONE shared `*_view` helper function, consumed by every view and every PrintDoc builder. Same for «counts as done» in aggregates.
- **Autogen guard (skip-aware):** idempotence keyed on (planting, task_type, campaign-window) treating done AND skipped as existing — a regenerated plan never inserts where a terminal-state task occupies the slot.
- **Error sentinels:** existing pattern holds; new sentinels documented alongside: `demo_mode_locked`, `instance_locked`, `fact_conflict`.
- **Refresh:** existing per-screen refresh helpers hold; the reconciliation screen updates line-locally after each gesture (never full-reload mid-session — it would lose the resume position).

### Enforcement Guidelines

**All AI agents MUST:** walk the 8-touchpoint checklist for any persisted change; `cargo fmt` + `clippy -D warnings` + `cargo test --workspace` before any PR; both `fr`+`en` keys for any string; extend the paper-loop by a step or dataset in every slice touching its path (DoD); never edit `field_event` rows (append-only).

**Verification:** cross_backend_tests for every new table incl. `swap_backend`/backup coverage; CI as pattern police (warnings, coverage, key parity).

**Anti-patterns:** state mutation outside `record_fact`; labels in payloads; a second «visible?» predicate; snapshotting PDF bytes; new callbacks in `main.rs`; CHECK constraints in new migrations; `now()` for any agronomic date or `recorded_at` below the UI/CLI layer.

## Project Structure & Boundaries

### Project Directory Structure (delta over existing workspace — NEW / MOD)

```
crates/
├── pomone-domain/src/
│   ├── crop_plan.rs                      NEW  P1 — CropPlanLine aggregate, staggering, draft state
│   ├── itk.rs                            NEW  P1 — ItkTemplate + ItkActivity (D5)
│   ├── field_event.rs                    NEW  P4 — Fact, FactKind, SkipReason, correction semantics (D1)
│   ├── capacity.rs                       NEW  P3 — pure occupancy/aggregation engine (D3)
│   ├── task.rs                           MOD  P4 — skipped projection, state() accessor, is_overdue fix
│   ├── planting.rs                       MOD  P4 — explicit-date transitions
│   └── date_calc.rs                      —    foundation, untouched (consumed by capacity.rs)
├── pomone-db/
│   ├── migrations/{sqlite,mariadb}/
│   │   ├── 0007_field_event.sql          NEW  P4 — field_event + task skip columns (numbered by merge order: E1 first)
│   │   ├── 0008_planning.sql             NEW  P1 — crop_plan_line, itk_template, itk_activity
│   │   ├── 0009_geometry.sql             NEW  P2 — occupation_kind discriminant
│   │   └── (per-slice additive pairs — never CHECK)
│   ├── src/codec.rs                      MOD  FactKind, SkipReason, plan-line states
│   ├── src/repository.rs                 MOD  + CropPlanRepo, ItkRepo, FieldEventRepo sub-traits
│   ├── src/{sqlite,mariadb}/             MOD  new repo impls ×2
│   └── src/cross_backend_tests.rs        MOD  every new table + swap/backup coverage
├── pomone-app/src/
│   ├── facts.rs                          NEW  P4 — record_fact single entry point, projections
│   ├── plan_view.rs                      NEW  P1 — plan-line DTOs, duplication, needs list
│   ├── itk_view.rs                       NEW  P1 — template editor DTOs
│   ├── capacity_view.rs                  NEW  P3 — curve DTOs, peak explanation
│   ├── reconcile_view.rs                 NEW  P4 — since-last-entry batch, 3 gestures, resume position
│   ├── print/                            NEW  P5 — mod.rs (PrintDoc), weekly_sheet.rs, crop_list.rs,
│   │                                            occupancy_map.rs, treatment_register.rs,
│   │                                            needs_list.rs (FR7 — 5th builder, desk document),
│   │                                            render.rs (printpdf, coverage-exempt)
│   ├── services.rs                       MOD  Slice 0 request structs; P1 generation
│   ├── task_autogen.rs                   MOD  P4 — skip-aware guard (campaign-window key)
│   ├── app.rs                            MOD  P6 — enter/exit_demo_mode, demo locks
│   ├── demo.rs                           MOD  P6 — demo farm gains plan lines + ITK templates
│   ├── migration.rs                      MOD  every new table joins copy_all (swap/backup completeness — lesson of bug #102)
│   ├── config.rs                         MOD  D4/D7 — documents_dir (serde default)
│   ├── locales/{fr,en}/main.ftl          MOD  every slice — print-*, plan-*, reconcile-* keys
│   └── tests/
│       ├── paper_loop.rs                 NEW  Slice 0 — walking skeleton, kill/replay harness
│       └── fact_invariants.rs            NEW  P4 — interleaving state-machine proptests
├── pomone-ui/
│   ├── src/main.rs                       MOD  Slice 0 — shrinks to bootstrap + wiring calls
│   ├── src/wiring/                       NEW  Slice 0 — one module per screen (existing + new)
│   └── ui/
│       ├── plan.slint                    NEW  P1 — crop-plan screen
│       ├── itk_editor.slint              NEW  P1
│       ├── placement.slint               NEW  P3 — placement + capacity curve
│       ├── reconcile.slint               NEW  P4 — batch reconciliation (start screen, FR47)
│       └── main.slint                    MOD  each screen (3-layer rule holds)
├── pomone-cli/src/main.rs                MOD  P6 — seed-demo delegates to pomone-app API
.github/workflows/ci.yml                  MOD  D4 — cargo-llvm-cov --ignore-filename-regex for print/render.rs
docs/export-contracts.md                  NEW  D7 — CSV/PDF contract documentation
```

### Architectural Boundaries

- **Write path (the only one):** UI/CLI gesture → `Fact` (client UUID, injected `recorded_at`) → `facts::record_fact` → one transaction: `field_event` insert + state projection → line-local refresh. No other state mutation exists.
- **Read paths:** screens ← `*_view.rs` DTOs (strings only, single visibility predicate) ← `Repository`; documents ← PrintDoc builders ← same `*_view` DTOs + `I18n` — **UI and print share the read path, never the render**.
- **Pure cores:** `capacity.rs` and `print/mod.rs` know no I/O — inputs in, values out; the 95% coverage target lives here.
- **Render frontier:** `print/render.rs` (printpdf) is the coverage-exempt leaf; nothing imports it except the export service.
- **Demo boundary:** demo mode swaps the `Repository` box in-memory; swap/backup/restore services check the demo lock sentinel.

### Requirements → Structure Mapping

| Work package | FRs | Primary locations |
|---|---|---|
| Slice 0 | — | `wiring/`, `services.rs` structs, `tests/paper_loop.rs` |
| P1 Planning | FR1–8 | `crop_plan.rs`, `itk.rs`, `0007`, `plan_view.rs`, `plan.slint` |
| P2/P3 Geometry+Capacity | FR9–17 | `capacity.rs`, `occupation_kind` col, `capacity_view.rs`, `placement.slint` |
| P4 Facts & states | FR18–28 | `field_event.rs`, `0008`, `facts.rs`, `reconcile_view.rs`, `reconcile.slint`, `task_autogen.rs` |
| P5 Documents | FR29–36 | `print/` (6 files), `config.rs`, `export-contracts.md`, `ci.yml` |
| P6 Demo | FR44 | `app.rs`, `demo.rs`, CLI |
| P7 Ergonomics | FR2–3, 23, 47 | `plan_view.rs` drafts, `reconcile_view.rs` resume, wiring |

### Data Flow

Plan: line → generated plantings (planned) → placed (tasks generated from ITK) → weekly PrintDoc → paper → facts → projections → re-print. Capacity: placements + windows → pure engine → curve DTO → screen & occupancy-map document. Register: treatment facts → view → PrintDoc → PDF + CSV. Needs list: unplaced plan lines → aggregation → PrintDoc → PDF.

## Architecture Validation Results

### Coherence Validation ✅

**Decision compatibility:** D1 (hybrid facts+states) is consumed consistently by D2 (per-gesture transactions), the gesture pattern, the autogen guard, and the future outbox option — no contradiction. D3 reuses existing columns, conflicting with nothing. D4's PrintDoc boundary is honored by the structure (render as coverage-exempt leaf) and the test architecture (goldens on PrintDoc only). D5 revives existing FKs instead of adding parallel columns — checked against migrations 0001–0006 conventions. D6/D7 touch disjoint seams. Stack versions: printpdf 0.9.x is the only addition; MSRV 1.80 unaffected.

**Pattern consistency:** naming follows shipped conventions (singular snake_case tables, codec literals, Fluent prefixes); the single write path (`record_fact`) and single visibility predicate directly enforce PRD invariants I1–I6; time-injection extends an existing pattern rather than inventing one.

**Structure alignment:** every pattern has a home in the delta tree; pure cores have no I/O imports by construction; the 3-layer Slint rule survives via the `wiring/` refactor.

### Requirements Coverage Validation ✅

**FRs:** all 30 R1 FRs map to a package and concrete files (Requirements → Structure table); the 14 R2 FRs are architecturally *anticipated* without being built (open fact kinds, generic document pipeline, `occupation_kind` discriminant, `units.rs` reuse for future doses). The 5 shipped FRs are preserved (autogen extended, views adapted — never replaced). The June-2027 R2 milestone (consumption ledger) rests on `treatment.confirmed` fact kinds + future additive quantity columns — anticipated.

**NFRs:** NFR6/7 → D1+D2+gesture pattern+kill/replay harness; NFR8 → additive-only + `migration.rs` copy_all coverage; NFR11 → printpdf offline, fonts embedded; NFR14–19 → PrintDoc carries localized/formatted content (localized numeric entry flagged for UX); NFR20–25 → pure cores + explicit render exemption in `ci.yml`; NFR26/27 → D7 contracts + `export-contracts.md`; NFR4 → `field_event` decade growth (~tens of thousands of rows) trivial under bounded queries.

### Implementation Readiness Validation ✅

Decisions carry versions and rationale; patterns cover the identified divergence points with anti-patterns listed; the delta tree is file-level specific; the walking-skeleton harness makes the release-blocking test executable from Slice 0.

### Gap Analysis Results

- **Resolved during validation — PDF font embedding.** FR/EN documents need an embedded font (no system-font dependency, offline, cross-platform). **Decision: DejaVu Sans (+ Bold) embedded** — permissive license (Bitstream Vera derivative), excellent Latin coverage, B&W-legible at small sizes. Lives in `pomone-app/print/fonts/`, loaded via `include_bytes!`.
- **Important (accepted by design):** UX design not yet run — the 4 new screens are architectural placeholders; internal layout awaits `create-ux-design`. No architectural decision depends on it.
- **Nice-to-have (deferred):** `docs/adr/` mirror of D1–D7 for contributors — this document serves the role meanwhile.

### Architecture Completeness Checklist

**Requirements Analysis:** [x] context analyzed · [x] scale assessed · [x] constraints identified · [x] cross-cutting mapped
**Architectural Decisions:** [x] critical decisions with versions · [x] stack specified · [x] integration patterns · [x] performance addressed
**Implementation Patterns:** [x] naming · [x] structure · [x] communication · [x] process
**Project Structure:** [x] directory structure · [x] boundaries · [x] integration points · [x] requirements mapping

### Architecture Readiness Assessment

**Overall Status: READY FOR IMPLEMENTATION** — 16/16 checklist items confirmed, zero critical gaps.
**Confidence: high** — the two novel engines are pure functions with defined test strategies; everything else extends shipped, tested patterns.
**Key strengths:** single write path enforcing PRD invariants structurally; test architecture designed *with* the delivery sequence (walking skeleton); brownfield seams exhaustively mapped with their costs.
**Future enhancement:** MariaDB outbox (half-built by `field_event`); ADR extraction; polymorphic occupancy behind `occupation_kind`.

### Implementation Handoff

**AI Agent Guidelines:** follow this document + the 34 project-context rules exactly; walk the 8-touchpoint checklist for persisted changes; every slice leaves `main` releasable and extends the paper loop.

**First Implementation Priority:** **Slice 0** — wiring refactor (`pomone-ui/src/wiring/`), request structs in `services.rs`, CHECK-constraint audit, empty paper-loop harness (`tests/paper_loop.rs`). No behavior change.
