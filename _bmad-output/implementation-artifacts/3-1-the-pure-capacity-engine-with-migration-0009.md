# Story 3.1: The pure capacity engine (with migration 0012)

Status: review

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->
<!-- Filename keeps the sprint-status key suffix `-0009` (legacy) for automation; the ACTUAL migration number is 0012 — see Dev Notes. -->

## Story

As a contributor,
I want `capacity.rs` — a pure engine computing soil occupancy over `[start, end)` windows, with a covered/open split, recursive hierarchy aggregation, and peak composition — built on `date_calc.rs`, plus the additive `occupation_kind` location column and the documented bed-meters rule,
so that capacity is exact, testable, and UI-independent — the foundation the placement screen (3.2) and the occupancy-map document (7.2) render.

## Context & scope boundary

This story delivers **two things and nothing else**:

1. **Migration 0012** — one additive column `occupation_kind` on the `location` table (single value `bed-meters` in R1), wired through the full persisted-field blast radius (domain → codec → both backends → cross-backend → copy_all).
2. **`crates/pomone-domain/src/capacity.rs`** — a **pure-functions-only** engine (no `Repository`, no I/O, no `now()`): inputs are plain value structs, outputs are plain values. Under heavy algebraic property tests, module coverage **≥ 95%**.

**Explicitly OUT of scope** (do not build here):
- No placement screen, no Slint, no `capacity_view.rs`, no DTOs (that is 3.2).
- No task generation (3.3), no perennial-termination gestures (3.4).
- No reading of real `Planting`/`PlannedPlanting` rows through a repository. The engine takes **abstract inputs**; the caller that later builds those inputs from repository data is 3.2's job.

The one bridge to the rest of the system in *this* story is the `occupation_kind` column + a **documented geometry rule** (below) that 3.2 will apply when it constructs engine inputs.

## Acceptance Criteria

1. **Migration 0012 is additive and dual-backend.** `migrations/sqlite/0012_geometry.sql` and `migrations/mariadb/0012_geometry.sql` each add `occupation_kind` to `location` with a non-null default of `bed-meters`. No `CHECK` constraint (per the additive-only rule); no trigger; no date/geometry computation in SQL. Both migrations run clean on connect.
2. **Full persisted-field blast radius closed.** `Location` carries the new field; the domain constructor sets/validates it; `codec.rs` has a paired `encode`/`decode` using the **same string literal** on both backends; both `SqliteRepository` and `MariaDbRepository` location row-mappers + INSERT + UPDATE handle it; `cross_backend_tests.rs` asserts round-trip parity; `copy_all` preserves it across a backend swap. A default-seeded and a round-tripped location both read back `occupation_kind = bed-meters`.
3. **Bed-meters geometry rule is implemented and documented.** The R1 capacity footprint of a placement = **bed-meters** = `length_m` of the **leaf (bed)** location it sits on; the covered/open discriminant = `true` iff the leaf location's kind **or any ancestor location's kind** is `covered`; `width_m` stays informative only. This rule lives as a documented, tested helper (doc-comment states the rule verbatim).
4. **The engine computes occupancy over half-open intervals.** Given a set of placements — each an interval `[start, end)` (end **exclusive**), a `bed_meters` footprint, a covered/open flag, and a leaf-location identity with its ancestor path — `capacity.rs` returns, for any query instant or sampled timeline, the total occupied bed-meters, split **covered vs open**, and aggregated **recursively up the location hierarchy** so every hierarchy level is readable (FR12).
5. **Peaks are explainable.** The engine can return, for a peak (or any instant), the **composing placements** (the series that overlap there) — the data 3.2/7.1 will render as "which series compose this peak" (FR11).
6. **Open-ended perennials extend to a horizon.** A placement with no end date (perennial without `expected_removal_on`) occupies from its start to the engine's supplied **horizon** and the result is stable under horizon extension (moving the horizon further out does not change occupancy before the old horizon).
7. **Algebraic property tests pass, coverage ≥ 95%.** Proptests hold: **superposition/additivity** (occupancy of A∪B = occupancy A + occupancy B where disjoint footprints), **commutativity** (placement order irrelevant), **monotonicity** (adding a placement never lowers occupancy anywhere), **adjacent-non-overlap** (`[a,b)` and `[b,c)` never double-count instant `b`), **horizon-extension stability**, **translation invariance** (shifting all intervals + query by the same offset shifts the result identically), **hierarchy coherence at sampled `t`** (a parent's occupancy = sum of its children's at every sampled instant), and **±50-year retro-entry** (intervals starting decades in the past compute without panic or overflow). `cargo llvm-cov` shows `capacity.rs` **≥ 95%**.
8. **Existing locations audited against the rule.** The seeded/demo locations are checked to conform to the geometry rule (leaf beds have a usable `length_m`); any gap is reported, not silently accepted.
9. **Green bar.** `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all -- --check` all pass; workspace coverage stays ≥ 80%.

## Tasks / Subtasks

- [x] **Task 1 — Migration 0012 `occupation_kind` (AC: 1, 2, 8)**
  - [x] `migrations/sqlite/0012_geometry.sql`: `ALTER TABLE location ADD COLUMN occupation_kind TEXT NOT NULL DEFAULT 'bed-meters';` — additive, no CHECK, no trigger.
  - [x] `migrations/mariadb/0012_geometry.sql`: the MariaDB-dialect equivalent (`VARCHAR`, `NOT NULL DEFAULT 'bed-meters'`), matching column semantics.
  - [x] Domain: add `occupation_kind` to `Location` (see Task 2).
  - [x] `codec.rs`: `encode_occupation_kind` / `decode_occupation_kind` (exhaustive match, **identical literal `"bed-meters"` on both backends**; a missed decode arm fails at runtime with `DbError::Malformed`).
  - [x] `crates/pomone-db/src/sqlite/location.rs`: add the column to every `SELECT` list, the `INSERT`, and the `UPDATE`; extend the row → `Location` mapper.
  - [x] `crates/pomone-db/src/mariadb/location.rs`: the identical changes on the MariaDB impl.
  - [x] `crates/pomone-db/src/cross_backend_tests.rs`: extend location coverage to assert `occupation_kind` round-trips identically on both backends.
  - [x] `crates/pomone-app/src/migration.rs` (`copy_all`): confirm locations carry `occupation_kind` across a swap; add/extend a test asserting it.
  - [x] Audit seeded locations (seed API + demo) for the geometry rule; report any leaf with no usable `length_m`.

- [x] **Task 2 — Domain: `OccupationKind` + geometry rule (AC: 3)**
  - [x] Add an `OccupationKind` enum in the domain (single variant `BedMeters` in R1; `#[non_exhaustive]`-minded design so future variants — tree-rows, hectares — are additive). Default = `BedMeters`.
  - [x] Thread it through `Location::new` (default `BedMeters`; add a builder or parameter consistent with the existing `LocationKind::with_covered` style — **copy the nearest sibling, do not invent a new shape**).
  - [x] Implement the **bed-meters footprint** helper: footprint = `length_m` of the leaf location. Document the rule verbatim in the doc-comment.
  - [x] Implement the **covered resolution** helper: a leaf is covered iff its kind or any ancestor's kind is `covered`. (The hierarchy walk itself may live where the ancestor chain is available — keep the domain helper pure over an ancestor list passed in.)
  - [x] Unit-test both helpers, including the ancestor-covered case and the leaf-with-no-children definition.

- [x] **Task 3 — `capacity.rs` pure engine (AC: 4, 5, 6)**
  - [x] New file `crates/pomone-domain/src/capacity.rs`; register in `lib.rs`. **No `Repository`, no I/O, no `now()`** — pure functions on value inputs, built on `date_calc.rs` helpers (`add_days`, `date_in_range`, `DateRange`…). **Never `.unwrap()` chrono arithmetic** — return `DomainError` on overflow.
  - [x] Define the input value type(s): a placement = `{ interval: [start, end) exclusive, bed_meters: Decimal, covered: bool, leaf location id + ancestor path }`. Open-ended (perennial) placements carry `end = None`, resolved against a supplied `horizon`.
  - [x] Occupancy at an instant / over a sampled timeline: total bed-meters, **covered vs open** split.
  - [x] Recursive **hierarchy aggregation**: occupancy readable at every location level (leaf → parcel → farm); a parent = sum of descendants at every instant (FR12).
  - [x] **Peak composition**: return the placements overlapping a given instant/peak (FR11 data only — no UI).
  - [x] **Read-path defensive posture (E2 lesson, project-context rule):** every `Decimal` sum/product and every count derived from unbounded persisted input must **saturate/cap, never panic**. A pathological-but-persisted `bed_meters` or a decades-wide interval must not crash aggregation. Prove saturation in a test.

- [x] **Task 4 — Algebraic property tests, ≥ 95% coverage (AC: 7)**
  - [x] Extend `crates/pomone-app/tests/support/mod.rs` generators (AI-E2-5 — **reuse, don't reinvent**) with placement/interval strategies (bounded magnitudes, incl. ±50-year spans).
  - [x] Proptests: superposition/additivity, commutativity, monotonicity, adjacent-non-overlap (`[a,b)`+`[b,c)` no double-count at `b`), horizon-extension stability, translation invariance, hierarchy coherence at sampled `t`, ±50-year retro-entry (no panic/overflow).
  - [x] Confirm `cargo llvm-cov` reports `capacity.rs` ≥ 95%.

- [x] **Task 5 — Green bar & self-review (AC: 9)**
  - [x] `cargo test --workspace` (+ `-- --ignored` if Docker present for MariaDB parity), `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all -- --check`.
  - [x] Walk the "Adding a persisted field" blast-radius checklist (Dev Notes) and confirm every row is closed.

## Dev Notes

### ⚠️ Migration number is 0012, not 0009

The story **filename** keeps the legacy `-0009` suffix only because that is the `sprint-status.yaml` key string (fixed for the automation). The **actual migration is `0012_geometry.sql`** — Epic 2 landed migrations 0008–0011 (`crop_plan_line`, `itk`, `first_on`, `planned_planting`), so 0009 is taken. This was decided and recorded in the Epic 2 retro. The stale `architecture.md` reference to `0011_geometry.sql` (line ~221) is also superseded — use **0012**. [Source: _bmad-output/implementation-artifacts/epic-2-retro-2026-07-15.md#Decisions; _bmad-output/planning-artifacts/epics.md#Epic-3]

### The persisted-field blast radius (walk every row — this is the #1 failure mode)

Adding `occupation_kind` ripples through the whole stack. Miss a link → compile error (exhaustive codec) or, worse, a **silent backend divergence**. [Source: _bmad-output/project-context.md#Adding-a-persisted-field]

```
domain      → Location gets occupation_kind; constructor sets/validates (default BedMeters)
codec       → encode_occupation_kind + decode_occupation_kind (exhaustive, identical "bed-meters" on both backends)
migration   → 0012_geometry.sql in sqlite AND mariadb (ADD COLUMN, additive only, no CHECK, no trigger)
db backends → sqlite/location.rs AND mariadb/location.rs: SELECT lists + INSERT + UPDATE + row mapper (BOTH)
cross-tests → cross_backend_tests.rs: occupation_kind round-trips identically
copy_all    → migration.rs: locations carry occupation_kind across a swap (test it)
i18n        → NOT needed this story — no user-facing string yet (label surfaces in 3.2). Do not add dead .ftl keys.
tests       → cross_backend + copy_all + domain unit tests
```

Note there is **no Slint / UI touchpoint** this story — the engine is pure and the column has no editor yet. That row of the usual checklist is deliberately empty here.

### Files to touch (existing patterns to copy)

- **Domain location:** `crates/pomone-domain/src/location.rs` (add field, constructor), `crates/pomone-domain/src/location_kind.rs` (the `with_covered` builder is the style to mirror for an additive attribute). [Source: crates/pomone-domain/src/location.rs; location_kind.rs]
- **Codec:** `crates/pomone-db/src/codec.rs` — the paired `encode_*`/`decode_*` pattern for sum types. `encode`/`decode` are a **pair**; same literal both backends. [Source: _bmad-output/project-context.md#Dual-Backend]
- **Backends:** `crates/pomone-db/src/sqlite/location.rs` and `crates/pomone-db/src/mariadb/location.rs` — current SELECT list is `id, parent_id, kind_id, name, length_m, width_m, notes`; add `occupation_kind` to all four SQL sites + the mapper. Decimals go through `decimal_to_text`/text helpers — `occupation_kind` is a plain enum string, simpler.
- **Cross-backend:** `crates/pomone-db/src/cross_backend_tests.rs`. **copy_all:** `crates/pomone-app/src/migration.rs` (has `copy_all_*` tests to extend).
- **Engine (NEW):** `crates/pomone-domain/src/capacity.rs` + `lib.rs` registration.
- **Proptest support:** `crates/pomone-app/tests/support/mod.rs` (extend, per AI-E2-5).

### The engine is PURE — the non-negotiable constraint

`capacity.rs` is one aggregate file in `pomone-domain`, **pure functions only, no `Repository`, built on `date_calc.rs` (never duplicate date math)**. This is the 95%-coverage core. Inputs in → values out. The mapping from real plantings/beds to engine inputs is 3.2's `capacity_view.rs`, not this story. [Source: _bmad-output/planning-artifacts/architecture.md line 177, 265; AR13 line 115]

### Domain modelling notes

- **Occupancy window `[start, end)` is half-open, end exclusive** — this is what makes adjacent successions (`[a,b)` then `[b,c)`) not double-count instant `b` (AC 7). Design the interval type around this from the start.
- Where the engine needs to derive an interval from a `PlantingSchedule`: a `Cycle` occupies from `start_date()` to `last_harvest_on` (inclusive of the harvest day → exclusive end = `last_harvest_on + 1`); a `Perennial` occupies `established_on` to `expected_removal_on` (or `None` → horizon). But keep the *engine* interval-based and put any schedule→interval mapping in a clearly separate, tested helper — the engine must not depend on `PlantingSchedule`'s shape. [Source: crates/pomone-domain/src/planting.rs `PlantingSchedule`, `start_date`]
- **Footprint = bed_meters (length along the bed), NOT area_m2.** `Planting` carries `area_m2`/`plants_count`; `PlannedPlanting` carries `bed_meters`. R1 capacity counts **bed-meters**. For the pure engine, footprint is just an input `Decimal` — the engine does not care where it comes from. [Source: architecture.md#D3 line 130; crates/pomone-domain/src/planned_planting.rs]
- **Covered resolution walks ancestors:** a bed is sheltered iff its own `LocationKind.covered` or any **ancestor location's** kind is covered. `LocationKind.covered` already exists (migration 0002). [Source: crates/pomone-domain/src/location_kind.rs; architecture.md#D3]

### Read-path defensive posture (codified E2 rule — a review-checklist item)

Any `Decimal` sum/product or count derived from an **unbounded persisted input** (SQLite decimals are free TEXT; a stored `bed_meters` could be absurd) must **saturate/cap, never panic** in aggregation. Precedents: `succession_dates` caps at `MAX_SUCCESSIONS`; `needs_view`/`plan_view` use `saturating_mul`/`saturating_add`. On every read-path arithmetic ask "what if this row is absurd?" and prove it saturates. Add a test with a pathological `bed_meters` and a ±50-year interval. [Source: _bmad-output/project-context.md#Read-path-defensive-posture; epic-2-retro §Cross-story pattern]

### Reference for domain semantics

For occupancy/capacity-curve behaviour, consult the Qrop reference at `../qrop-main` rather than guessing — but note Pomone models **perennials** (which Qrop does not), so the horizon-extension and ±50-year retro-entry behaviour is Pomone-specific and has no Qrop analogue. [Source: _bmad-output/project-context.md#Project-Origin]

### Review process for this story (Guy's decision)

**3-layer adversarial review is default-ON for 3.1** (schema/domain/engine story) — same discipline as 2.1/2.6. The adversarial reviews in Epic 2 caught real, happy-path-invisible defects (OOM caps, decimal overflow, position collisions); expect the same class of issue in interval arithmetic and hierarchy aggregation. [Source: epic-2-retro §Decisions, AI-E2-1]

### Traps carried from Epic 2

- **Never `perl` on UTF-8 `.ftl`** — mojibake. Use the Edit tool. (Not expected this story — no `.ftl` changes — but noted.) [Source: epic-2-retro AI-E2-4]
- `encode`/`decode` asymmetry fails at **runtime**, not compile time — keep both matches exhaustive and the literal identical.

### Project Structure Notes

- Layer discipline: `capacity.rs` in `pomone-domain` depends only on `date_calc.rs`, `error.rs`, `ids.rs`, `rust_decimal`, `chrono`. No upward/sideways deps. [Source: project-context.md#Architecture-Layering]
- File-size rule: keep `capacity.rs` well under 2000 lines (target), 3000 hard max — split if it grows. [Source: project-context.md; user memory regle-taille-fichiers]
- MSRV 1.80, edition 2021, `unsafe_code = deny`, clippy `pedantic`; rely on the workspace allow-list, don't sprinkle local `#[allow]`.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Epic-3 / Story-3.1] — user story + BDD acceptance criteria, migration 0012.
- [Source: _bmad-output/planning-artifacts/architecture.md#D3] — geometry: bed-meters = leaf `length_m`, covered from `LocationKind.covered`, additive `occupation_kind`.
- [Source: _bmad-output/planning-artifacts/architecture.md AR13 (line 115), P3 (line 34), lines 177/265] — capacity pure engine, proptest list, 95% coverage core.
- [Source: _bmad-output/planning-artifacts/prd.md FR9–FR15] — placement & capacity FRs; FR11 (explainable peaks — data here, UI in E7), FR12 (aggregates up hierarchy).
- [Source: _bmad-output/project-context.md] — 34 rules; blast-radius checklist, dual-backend, date-in-Rust, read-path saturation.
- [Source: _bmad-output/implementation-artifacts/epic-2-retro-2026-07-15.md] — migration renumber to 0012, review decision, AI-E2-5 (reuse proptest support), read-path posture.

## Dev Agent Record

### Agent Model Used

claude-opus-4-8[1m] (Opus 4.8, 1M context) — dev-story workflow.

### Debug Log References

- End-to-end migration check: `XDG_DATA_HOME=/tmp/pom312 cargo run -p pomone-cli -- seed-demo` →
  fresh DB shows `location` columns include `occupation_kind`; all 12 demo locations read
  `occupation_kind = 'bed-meters'`, every `length_m > 0`.
- `cargo llvm-cov` → `capacity.rs` **100%** line coverage (295/295), above the 95% gate.
- Full suite: `cargo test --workspace` green (domain 194, app 227+8 integration, db 95 +15 MariaDB
  ignored); `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo fmt --check` clean.

### Completion Notes List

- **Migration 0012** (`0012_geometry.sql`, both backends) adds `occupation_kind` additively — no CHECK,
  no trigger, no SQL geometry. Full persisted-field blast radius closed: `Location.occupation_kind`
  (default `BedMeters`, builder `with_occupation_kind` mirroring `LocationKind::with_covered`),
  paired `encode/decode_occupation_kind` in `codec.rs` (identical `"bed-meters"` literal both backends),
  SQLite + MariaDB SELECT/INSERT/UPDATE/row-mapper, cross-backend round-trip assertion, and a `copy_all`
  test asserting the column survives a backend swap.
- **`capacity.rs`** is a pure-functions-only engine (no `Repository`, no I/O, no `now()`): half-open
  `[start, end)` intervals (exclusive end → adjacent successions don't double-count the boundary),
  `CoverSplit` covered/open totals with **saturating** Decimal maths (E2 read-path posture),
  `path`-based recursive hierarchy aggregation (a parent sums all descendant beds — FR12),
  `composition_at` for explainable peaks (FR11 data), `peak` over a window (earliest-date tie-break),
  and `occupancy_window` mapping a `PlantingSchedule` to its interval (kept as a *separate* helper so the
  engine core stays independent of the schedule's shape; open-ended perennials → `None` → resolved
  against a caller-supplied horizon).
- **Geometry rule** implemented + tested as domain helpers: `Location::bed_meters()` (= `length_m`, not
  area) and `is_sheltered(...)` (leaf or any ancestor covered). Doc-comments state the rule verbatim.
- **Algebraic proptests** (8): superposition, commutativity, monotonicity, translation invariance,
  adjacent-non-overlap, horizon-extension stability, hierarchy coherence, ±50-year retro-entry — all pass.
- **Deviation from Task 4's first subtask (honest note):** the proptests live **inline in
  `pomone-domain/src/capacity.rs`**, not in `pomone-app/tests/support/mod.rs`. Rationale: `capacity.rs`
  is pure `pomone-domain` code and `pomone-domain` already carries `proptest` as a dev-dep with the
  `date_calc.rs` precedent for inline algebraic proptests; putting them in the `pomone-app` interleaving
  harness (a fact-sequence state machine over `record_fact`) would have created a spurious upward
  dependency and mismatched that harness's purpose. The *intent* of AI-E2-5 (bounded, reusable strategy;
  no reinvention of the wheel) is met via `arb_placement`/`arb_placements` in-module.
- **AC 8 audit:** `length_m > 0` is a domain invariant (`require_positive_dimension` in `Location::new`),
  so no persisted location can have a non-usable footprint; confirmed against the 12 demo locations.
- **Scope honored:** no Slint/UI, no `capacity_view`, no task generation, no placement screen — those are
  stories 3.2–3.4. No `.ftl` keys added (no user-facing string yet).

### File List

**Added**
- `migrations/sqlite/0012_geometry.sql`
- `migrations/mariadb/0012_geometry.sql`
- `crates/pomone-domain/src/capacity.rs`

**Modified**
- `crates/pomone-domain/src/location.rs` — `OccupationKind` enum, `Location.occupation_kind` field +
  `with_occupation_kind` builder, `bed_meters()` footprint helper, `is_sheltered` free fn, tests.
- `crates/pomone-domain/src/lib.rs` — register `capacity` module; re-export `OccupationKind`,
  `is_sheltered`, and the capacity engine types/fns.
- `crates/pomone-db/src/codec.rs` — `encode/decode_occupation_kind` + tests.
- `crates/pomone-db/src/sqlite/location.rs` — column in SELECT×4/INSERT/UPDATE + row mapper.
- `crates/pomone-db/src/mariadb/location.rs` — column in COLUMNS/INSERT/UPDATE + row mapper.
- `crates/pomone-db/src/cross_backend_tests.rs` — assert `occupation_kind` round-trips.
- `crates/pomone-app/src/migration.rs` — assert `copy_all` preserves `occupation_kind`.

## Change Log

- 2026-07-16 — Story 3.1 implemented: migration 0012 (`occupation_kind`, both backends, full blast
  radius) + pure `capacity.rs` engine (occupancy/cover-split/hierarchy/peaks) with 8 algebraic proptests
  at 100% module coverage. All green (test/clippy/fmt); verified end-to-end via `seed-demo`.
