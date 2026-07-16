# Story 3.2: The placement screen with the live curve

Status: review

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As the grower,
I want to assign my unplaced planned successions to beds in a location tree and watch the soil-occupancy curve react instantly — sheltered and open-field apart — with overflow flagged and its composing series listed,
so that I feel the capacity constraint at the moment I place, and can arbitrate before committing.

## Context, dependencies & scope boundary

**This story is the first screen built on the story-3.1 capacity engine.** It converts the *unplaced* `planned_planting` rows (materialized in story 2.6) into real placed `Planting`s on beds, and renders the live occupancy curve from the pure `capacity.rs` engine.

**Hard dependency — story 3.1 (PR #146, MERGED to `main` 2026-07-16).** This story `use`s `pomone_domain::capacity` (`Placement`, `occupancy_at`, `peak`, `composition_at`, `occupancy_window`, `CoverSplit`) and the `occupation_kind` column + `is_sheltered`/`Location::bed_meters()` helpers — all now on `main`. **Branch this work off `main`** — do not duplicate any capacity maths; the engine is the single source of truth.

**Placement model (decided by Guy): convert to a full `Planting`.** Placing a planned succession **creates a real `Planting`** on the chosen bed (reusing the existing `create_annual_planting` / `create_perennial_planting` services) and marks the planned row as placed (non-destructively — see migration 0013). See Dev Notes for how `area_m2` / `plants_count` / `strata` are sourced.

**In scope:**
1. Migration **0013** — additive `placed_planting_id` link on `planned_planting` (nullable FK → `planting`), so "unplaced" is a query and placement is reversible & traceable.
2. `capacity_view.rs` (NEW) — read-path builder: turns placed `Planting`s + location geometry/hierarchy into `capacity::Placement` inputs, calls the pure engine, returns **string-only** curve/peak/composition DTOs.
3. Placement service — `place_planned_planting` (convert + link) and `unplace_planned_planting` (undo while not acted upon), in `services.rs`.
4. `placement.slint` (NEW) + full 3-layer wiring (`placement.slint`, `main.slint`, `wiring/placement.rs`) — unplaced list (left), bed tree (right), live covered/open curve, overflow amber + peak value, click-peak → composing series (basic).
5. A **measured perf test**: curve recompute **≤ 100 ms for ≤ 500 placements**.
6. fr + en Fluent keys for every new user-facing string.

**Out of scope (later stories):**
- **Task generation at placement → story 3.3.** Placing here creates the `Planting` and updates the curve; it does **not** yet derive ITK tasks.
- **The full PeakPanel with candidate badges + shift/move/cut arbitrage → Epic 7 (7.1).** Here the peak only *lists* its composing series (FR11 "basic").
- Retro-entry perennial reassurance line → story 3.4.
- Printed occupancy map → story 7.2.

## Acceptance Criteria

1. **Migration 0013 is additive and dual-backend.** `planned_planting` gains `placed_planting_id` (nullable, FK → `planting(id)` `ON DELETE SET NULL`) in `migrations/{sqlite,mariadb}/0013_*.sql`. No CHECK, no trigger. Full blast radius: domain field on `PlannedPlanting`, codec (nullable uuid — reuse existing optional-uuid handling), both backend row-mappers + INSERT/UPDATE, `cross_backend_tests`, and `copy_all`.
2. **Unplaced list is correct.** The placement screen lists exactly the `planned_planting` rows with `placed_planting_id IS NULL`, newest-plan-first or plan-order (grouped legibly by plan line/variety), each showing variety, date, and bed-meters as localized strings.
3. **Placing converts and links.** Assigning an unplaced succession to a bed (tree) + strata creates a real `Planting` via the existing planting-creation service (annual → `create_annual_planting`, perennial → `create_perennial_planting`), sets the planned row's `placed_planting_id`, and removes it from the unplaced list — all in one logical operation. `area_m2` and `plants_count` are sourced per Dev Notes.
4. **The curve reacts live, covered/open apart.** After a placement the occupancy curve updates **within one frame at farm scale — ≤ 100 ms for ≤ 500 placements, proven by a perf test** — showing two distinct series (sheltered vs open-field) computed from the story-3.1 engine (`occupancy_at` over the season, bed-meters footprint, `is_sheltered` cover split).
5. **Overflow shows amber with the peak value.** When occupancy exceeds a bed/level's capacity, the curve/peak indicator turns amber and shows the peak occupancy value (per `theme.slint` amber token; no `PopupWindow` — inline/side-panel per UX-DR / Slint-1.8 constraint).
6. **Peaks list their composing series (basic).** Clicking a peak shows the placements composing it (`composition_at` from the engine) — variety + date + bed-meters — as a read-only list. No candidate badges, no shift/move/cut (that is Epic 7).
7. **Placement is freely undoable while not acted upon.** `unplace_planned_planting` deletes the created `Planting` and clears `placed_planting_id`, restoring the row to the unplaced list — allowed as long as no facts have been recorded against the planting (in this story's scope none exist yet; the guard must still be written so 3.3+ stay safe). No "unsaved changes" friction; Échap/undo never loses data.
8. **Read-path defensive posture holds.** Every `Decimal`/count in `capacity_view` aggregation **saturates/caps, never panics** on a pathological persisted `bed_meters`/`width_m` or an absurd interval (project-context rule; the engine already saturates — the view must not reintroduce a panic, e.g. in `area_m2 = bed_meters × width_m` or an `f64` cast for the curve).
9. **i18n parity.** Every new string has both `fr` and `en` keys, alphabetical within section, mirrored; DTOs expose strings only (no `Uuid`/`Decimal`/enums to the UI).
10. **Green bar.** `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all -- --check` pass; workspace coverage ≥ 80%; `capacity_view.rs` (a read/aggregation path) ≥ 95% per NFR20.

## Tasks / Subtasks

- [x] **Task 1 — Migration 0013: `placed_planting_id` on `planned_planting` (AC: 1, 7)**
  - [x] `migrations/sqlite/0013_planned_planting_placed.sql`: `ALTER TABLE planned_planting ADD COLUMN placed_planting_id BLOB REFERENCES planting(id) ON DELETE SET NULL;` (nullable, additive, no CHECK).
  - [x] `migrations/mariadb/0013_planned_planting_placed.sql`: MariaDB equivalent (`BINARY(16) NULL`, FK `ON DELETE SET NULL`).
  - [x] Domain: add `placed_planting_id: Option<PlantingId>` to `PlannedPlanting`; keep the constructor's `bed_meters > 0` guard; default `None` on `new`.
  - [x] Both backends (`{sqlite,mariadb}/planned_planting.rs`): column in SELECT/INSERT/UPDATE + row mapper (reuse the existing optional-uuid bind/read pattern used for `parent_id` on `location`).
  - [x] `cross_backend_tests.rs`: round-trip a placed and an unplaced planned_planting.
  - [x] `migration.rs` (`copy_all`): confirm the link survives a swap; assert it. **Order matters** — plantings must be copied before planned_plantings that reference them, or the FK will reject; verify/adjust `copy_all` ordering.

- [x] **Task 2 — `capacity_view.rs` (NEW): curve/peak/composition DTOs (AC: 2, 4, 6, 8)**
  - [x] New `crates/pomone-app/src/capacity_view.rs`; register in `lib.rs`. Takes `&dyn Repository`, returns **string-only** DTOs (never `Uuid`/`Decimal`/enums to UI).
  - [x] Build `capacity::Placement` inputs from placed `Planting`s: `path` = leaf bed + ancestor chain (walk `location.parent_id`); `bed_meters` = the planting's footprint on its bed; `covered` = `is_sheltered` over the leaf+ancestor kinds; interval via `capacity::occupancy_window(&schedule)`.
  - [x] `unplaced_list(repo)` → DTOs of `planned_planting WHERE placed_planting_id IS NULL` (variety name, localized date, bed-meters string).
  - [x] Curve DTO: a sampled season series (weekly buckets, like `bed_usage_view`) of covered vs open occupancy from `occupancy_at`; plus per-level `peak` (value + amber flag when over capacity). **Reuse `bed_usage_view`'s ancestor-covered walk and weekly bucketing shape — don't reinvent; but compute from the 3.1 engine, in bed-meters, perennials included.**
  - [x] Composition DTO for a clicked instant: `composition_at` → list of {variety, date, bed-meters} strings.
  - [x] Saturation: any `Decimal` product/sum (`area`, curve totals) uses `saturating_*`; any `Decimal`→`f64` for the curve clamps, never `unwrap`s.

- [x] **Task 3 — Placement service: place + unplace (AC: 3, 7)**
  - [x] `PlacementRequest` struct + `place_planned_planting(repo, req)` in `services.rs`: load the planned_planting, build the planting-creation request (strata + `area_m2`/`plants_count` per Dev Notes), call `create_annual_planting`/`create_perennial_planting` (annual vs perennial decided by the variety's profile — reuse the existing branch), then set `placed_planting_id` on the planned row. One consistent operation.
  - [x] `unplace_planned_planting(repo, planned_planting_id)`: guard that no facts are recorded against the placed planting (reuse `planting_has_activity` / the skip-aware predicate), then delete the `Planting` and clear `placed_planting_id`. Return a structured `AppError` if the guard blocks (localized).
  - [x] Unit tests: place → planned row hidden + Planting exists; unplace → Planting gone + row back; unplace blocked when activity present.

- [x] **Task 4 — `placement.slint` + 3-layer wiring (AC: 2, 4, 5, 6, 9)**
  - [x] `crates/pomone-ui/ui/placement.slint` (NEW): unplaced list (left), bed **tree** (right, reuse `crop_map.slint`/`locations.slint` tree pattern), live curve (covered/open), overflow amber + peak value, click-peak → composing series inline/side panel (**no `PopupWindow`**).
  - [x] 3-layer plumbing: page properties/callbacks on `PlacementPage` → re-declared+forwarded on `MainWindow` in `main.slint` → `get_*`/`set_*`/`on_*` in `crates/pomone-ui/src/wiring/placement.rs` (new module; register in `wiring/mod.rs`). **Touch all three files per property or the generated method won't exist.** Avoid the reserved `row` property name (rename to `data`) — recurring Slint gotcha.
  - [x] Wire callbacks to `capacity_view` + placement service; refresh the curve line-locally after each placement (keep the VecModel handle; don't rebuild the whole model).
  - [x] Field-state grammar (UX-DR2): derived curve/peak values non-editable/muted; amber = over capacity.

- [x] **Task 5 — Perf test: ≤ 100 ms for ≤ 500 placements (AC: 4)**
  - [x] New test (e.g. `crates/pomone-app/tests/capacity_perf.rs`): seed ≤ 500 placed plantings across a bed hierarchy, measure a full curve recompute with `std::time::Instant`, assert `< 100 ms`. **Wall-clock `Instant` in a test is fine** — the `now()` ban is about *business/agronomic* time below the UI (AR12), not test instrumentation. Keep the bound generous enough to not be flaky in CI (assert the p50 or a single warm run under a margin).

- [x] **Task 6 — i18n + green bar (AC: 9, 10)**
  - [x] Add `placement-*` / `capacity-*` keys to **both** `locales/fr/main.ftl` and `locales/en/main.ftl` (alphabetical within section; `{ $name }` interpolation). Update the glossary + coherence test if a new domain term appears (e.g. «placement», «pic»/«peak»).
  - [x] `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all -- --check`; confirm `capacity_view.rs` ≥ 95% via `cargo llvm-cov`.
  - [x] Walk the "Adding a persisted field" blast-radius checklist for `placed_planting_id`.

## Dev Notes

### Dependency on story 3.1 — branch accordingly

This story consumes the pure engine and geometry from 3.1 (PR #146, merged to `main`). **Do not re-derive occupancy, cover split, or the bed-meters rule** — import from `pomone_domain::capacity` and use `Location::bed_meters()` / `is_sheltered`. Branch off `main`. [Source: story 3-1; _bmad-output/planning-artifacts/architecture.md line 211/265]

### How `area_m2` / `plants_count` / `strata` are sourced (the placement conversion)

`PlannedPlanting`/`CropPlanLine` carry only `bed_meters` — `Planting::new` needs `area_m2 > 0`, `plants_count > 0`, and a `strata_id`. Decided approach (Guy): **convert to a full `Planting`**, sourcing the missing pieces at placement time:
- **`strata_id`** — chosen in the placement panel (required). The bed tree gives the location; the grower picks the vegetation stratum (reuse the strata options list — see `plantings_view`/`list_variety_options` sibling pattern).
- **`area_m2`** — derived: `bed_meters × location.width_m` (the bed's running metres × its width). Use `saturating_mul` (defensive: persisted `width_m`/`bed_meters` are unbounded TEXT). Must be `> 0` (both factors are domain-positive, so OK).
- **`plants_count`** — entered in the placement panel (a small numeric field, `> 0`). R1 has no density model on the plan line, so the grower supplies it; a future story may derive it from ITK/variety spacing. Keep the field minimal (one input, validated on exit per the field-state grammar).
- Reuse `create_annual_planting` (`AnnualPlantingRequest`) for annual varieties and `create_perennial_planting` for pluriannual — branch on `variety.profile` exactly as those services already do. The planted date = the planned succession's `planned_on`.

Flag any friction to Guy if the extra `plants_count`/`strata` inputs feel heavy at placement — but the domain requires them, so R1 asks once per placement.

### Placement is non-destructive + reversible (migration 0013 rationale)

Rather than deleting the `planned_planting` on placement (which would make undo lossy and history opaque), **add a nullable `placed_planting_id` link**. "Unplaced" = `placed_planting_id IS NULL`. This mirrors the 2.6 non-destructive-regeneration discipline and the 1.3 "don't resurrect settled work" guard. Undo = delete the `Planting` (FK `ON DELETE SET NULL` clears the link automatically) and the row reappears unplaced. [Source: epic-2-retro §"non-destructive regeneration"; project-context.md#Read-path]

### The persisted-field blast radius (walk every row)

```
domain      → PlannedPlanting.placed_planting_id: Option<PlantingId> (default None in new())
codec       → optional-uuid (reuse existing Option<Uuid> handling — NOT a new enum/codec fn)
migration   → 0013_*.sql sqlite AND mariadb (ADD COLUMN nullable FK, ON DELETE SET NULL, additive)
db backends → {sqlite,mariadb}/planned_planting.rs: SELECT + INSERT + UPDATE + row mapper (BOTH)
cross-tests → cross_backend_tests.rs: placed + unplaced round-trip
copy_all    → migration.rs: plantings BEFORE planned_plantings (FK order); assert link preserved
app/view    → capacity_view unplaced filter; placement service sets/clears it
ui (Slint)  → placement.slint + main.slint + wiring/placement.rs (all three)
i18n        → placement-*/capacity-* keys in fr AND en
tests       → cross_backend + copy_all + service unit tests + perf test
```
[Source: _bmad-output/project-context.md#Adding-a-persisted-field]

### Files to create / touch (copy the nearest sibling)

- **NEW** `crates/pomone-app/src/capacity_view.rs` — model on `bed_usage_view.rs` (weekly buckets, ancestor-covered walk) but compute from `capacity.rs` in bed-meters, perennials included, string-only DTOs. [Source: crates/pomone-app/src/bed_usage_view.rs]
- **NEW** `crates/pomone-ui/ui/placement.slint` + `crates/pomone-ui/src/wiring/placement.rs` — mirror the tree screen (`crop_map.slint` / `locations.slint`) for the bed tree and the list-screen pattern for the unplaced list. Register `mod placement;` in `wiring/mod.rs` and add the page to `main.slint`.
- **MOD** `services.rs` — `PlacementRequest` + `place_planned_planting` + `unplace_planned_planting`, next to `create_annual_planting`/`create_perennial_planting`. Reuse `plantings_view::parse_id`.
- **MOD** `crates/pomone-domain/src/planned_planting.rs` — new field + constructor default; extend tests.
- **MOD** both `planned_planting.rs` backends, `cross_backend_tests.rs`, `migration.rs`.
- **MOD** `locales/{fr,en}/main.ftl`.

### Slint traps carried from Epic 2

- **3-layer plumbing tax:** every page property touches three files (page.slint, main.slint re-declare+forward, wiring get/set/on). [Source: project-context.md#Slint-UI-Plumbing; epic-2-retro]
- **Reserved `row` property** → rename to `data` ("Cannot override property 'row'").
- **`PopupWindow` is banned (Slint 1.8):** overlays = inline expansion or side panel. The peak listing is a side panel / inline block, not a popup. [Source: ux-design-specification.md line 439]
- **Conditional-element property access** across an `if` → refactor to in-out props + `<=>` two-way binding.
- **Never `perl` on UTF-8 `.ftl`** (mojibake) — use the Edit tool. [Source: epic-2-retro AI-E2-4]

### UX intent (the "capacity moment")

The Sunday-armchair scene: place a crop, watch the curve answer; the April overflow forces a *named* sacrifice — but in this story the peak only **explains** (lists composing series), it does not yet offer shift/move/cut (Epic 7). "Explain, never prescribe." Covered and open are **two distinct curves**, each normalized to its own group (a farm reads "greenhouses 100% full, open field 40%"), not nested. Overflow = amber + the peak value; ≥40%-grey / hatching conventions belong to the printed map (7.2), not this screen. [Source: ux-design-specification.md lines 44/62/282/288/308–315/361; bed_usage_view.rs definitions]

### Testing & perf

- `capacity_view.rs` is a read/aggregation path → **≥ 95% coverage** (NFR20). The pure engine is already tested in 3.1; here test the *DTO construction*, the unplaced filter, cover-split from the hierarchy walk, and saturation on absurd rows.
- Perf: `Instant`-measured, ≤ 500 placements, < 100 ms full recompute. Wall-clock timing in a test is allowed (the `now()` ban is agronomic time, AR12). Give CI margin to avoid flakiness. [Source: epics.md story 3.2; epic-2-retro "First in-story perf budget"]
- MariaDB parity tests stay `#[ignore]`d; run locally with Docker via `-- --ignored` if changing repo code.

### Review process (Guy's decision)

**Focused 2-reviewer for 3.2** (screen + curve), not the full 3-layer — per AI-E2-1. The correctness-critical maths already landed adversarially-reviewed in 3.1; here the risk is UI plumbing + the read-path saturation, which a focused review covers. [Source: epic-2-retro §Decisions, AI-E2-1]

### Project Structure Notes

- Layer discipline: `capacity_view` depends on `capacity.rs` (domain) + `Repository`; the UI never sees domain types. `capacity.rs` stays pure — no repo leaks into it.
- File-size rule: keep `capacity_view.rs` and `placement.slint` under 2000 lines (target). [Source: project-context.md; user memory regle-taille-fichiers]
- MSRV 1.80, `unsafe_code = deny`, clippy `pedantic`; rely on the workspace allow-list.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Epic-3 / Story-3.2] — user story + BDD: curve within one frame (≤100ms/500), amber overflow + peak value, click-peak lists series, freely undoable while not active.
- [Source: _bmad-output/planning-artifacts/architecture.md lines 231/253/264–266] — `capacity_view.rs`, `placement.slint`, read-path (UI ← view DTOs), pure-core boundary.
- [Source: _bmad-output/planning-artifacts/prd.md FR9–FR15] — FR10 (live curve, covered/open counted separately), FR11 (explainable peaks — data here, full UI E7), FR12 (aggregates up hierarchy).
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md] — placement/capacity screen intent, PeakPanel deferred to E7, PopupWindow ban, two disjoint curves.
- [Source: crates/pomone-app/src/bed_usage_view.rs] — existing occupancy curve (naive predecessor): ancestor-covered walk + weekly buckets to reuse.
- [Source: crates/pomone-app/src/services.rs] — `create_annual_planting`/`create_perennial_planting`, `planting_has_activity` (undo guard).
- [Source: _bmad-output/implementation-artifacts/3-1-*.md] — the engine API this story consumes.
- [Source: _bmad-output/implementation-artifacts/epic-2-retro-2026-07-15.md] — review decision, non-destructive discipline, Slint traps, perf-budget flag.

## Dev Agent Record

### Agent Model Used

claude-opus-4-8[1m] (Opus 4.8, 1M context) — dev-story workflow.

### Debug Log References

- `capacity_view.rs` coverage: **98.3%** (400/407) — above the 95% read-path gate (NFR20).
- Perf: `tests/capacity_perf.rs` — 500 placements across a bed hierarchy, full `occupancy_curve`
  recompute well under the 100 ms budget (whole test incl. setup ≈ 0.15 s).
- Full suite green: `cargo test --workspace` (app lib 241, db 95 +15 MariaDB ignored, domain 194,
  + integration: capacity_perf, glossary_coherence, paper_loop, fact_invariants…);
  `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo fmt --check` clean.
- End-to-end: `seed-demo` on a fresh DB then launched the real app — migrations 0012 **and 0013**
  apply on connect, the window (with the placement screen wired) builds and runs without panic.

### Completion Notes List

- **Migration 0013** (`0013_planned_planting_placed.sql`, both backends) adds a nullable
  `placed_planting_id` FK on `planned_planting` (`ON DELETE SET NULL`) — placement is
  **non-destructive & reversible**: "unplaced" = `placed_planting_id IS NULL`, un-placing deletes the
  planting and the FK auto-returns the row to the unplaced list. Full blast radius closed (domain field
  + `is_placed()`, both backends via the existing optional-uuid pattern, cross-backend round-trip incl.
  `ON DELETE SET NULL`, copy_all preservation with the plantings-before-planned FK ordering asserted).
- **`capacity_view.rs`** (read-path, string-only DTOs): `unplaced_list`, `occupancy_curve` (weekly
  covered/open series from the 3.1 engine, per-group capacity + overflow flag), `peak_composition`
  (FR11 basic). Footprint = `area_m2 / bed.width_m` (recovers the placed bed-metres); cover split via
  the ancestor-covered walk (reused shape from `bed_usage_view`) + domain `is_sheltered`; interval via
  `capacity::occupancy_window`. **Read-path saturation** throughout (`saturating_*`, clamped `to_f32`);
  proven by `absurd_area_saturates_not_panics`.
- **Placement service** (`services.rs`): `place_planned_planting` converts a planned succession to a
  real `Planting` (annual → `create_annual_planting`, perennial → `create_perennial_planting`), setting
  `area_m2 = bed_meters × bed.width_m` (saturating) and taking `strata`/`plants_count` from the request;
  `unplace_planned_planting` guards via the existing `delete_planting` (surfaces `PlantingHasActivity`)
  and relies on `ON DELETE SET NULL`. 6 service tests incl. the activity-blocked undo path.
- **Placement screen** (`placement.slint` + full 3-layer wiring): unplaced list (left, selectable),
  bed tree (middle, depth-indented, reuses `list_locations_tree`) + strata dropdown + plant-count field
  + Place / Undo, live curve (two `Path` series like `home.slint`) + amber peak (`Palette.warning`) that
  is clickable to list its composing series (inline, **no `PopupWindow`**). Nav entry + `Ctrl+0`
  shortcut. Undo tracks the last-placed succession in `UiState.placement_last_placed`.
- **A small additive helper on the merged 3.1 engine** — `Placement::covers(t, horizon)` (public wrapper
  over the private `active_at`) — lets `capacity_view` compose peaks without re-deriving interval logic.
  Additive, covered by the existing engine tests + the new composition test.
- **i18n**: `nav-placement`, `title-placement`, and all `placement-*` keys added to **both** `fr` and
  `en` catalogues (mirrored); glossary-coherence test stays green.
- **Refactor**: extracted the per-screen `wire_*` calls in `main.rs` into `wire_all_screens` (kept
  `main()` under the 100-line clippy cap after adding the placement wiring).
- **Scope honored**: no task generation at placement (3.3), no full PeakPanel/arbitrage (E7), no
  perennial reassurance line (3.4), no printed map (7.2).
- **Not driven interactively**: the Slint GUI itself wasn't clicked headlessly (no scripted UI harness);
  all view/service/curve logic is unit/integration-tested and the app boots the wired screen without
  panic on the demo DB.

### File List

**Added**
- `migrations/sqlite/0013_planned_planting_placed.sql`
- `migrations/mariadb/0013_planned_planting_placed.sql`
- `crates/pomone-app/src/capacity_view.rs`
- `crates/pomone-app/tests/capacity_perf.rs`
- `crates/pomone-ui/ui/placement.slint`
- `crates/pomone-ui/src/wiring/placement.rs`

**Modified**
- `crates/pomone-domain/src/planned_planting.rs` — `placed_planting_id` field + `is_placed()` + test.
- `crates/pomone-domain/src/capacity.rs` — public `Placement::covers` helper.
- `crates/pomone-db/src/{sqlite,mariadb}/planned_planting.rs` — column in SELECT/INSERT/UPDATE + mapper.
- `crates/pomone-db/src/cross_backend_tests.rs` — placed/unplaced round-trip + `ON DELETE SET NULL`.
- `crates/pomone-app/src/migration.rs` — copy_all preserves the placement link (FK ordering asserted).
- `crates/pomone-app/src/services.rs` — `PlacementRequest` + place/unplace + 6 tests.
- `crates/pomone-app/src/lib.rs` — register `capacity_view`; re-export its DTOs.
- `crates/pomone-app/locales/{fr,en}/main.ftl` — `placement-*` / `nav-placement` keys.
- `crates/pomone-ui/ui/main.slint` — import/re-export, nav props+callbacks, NavButton, `Ctrl+0`, content block.
- `crates/pomone-ui/src/state.rs` — `placement_last_placed` on `UiState`.
- `crates/pomone-ui/src/translations.rs` — set placement labels.
- `crates/pomone-ui/src/wiring/mod.rs` — register `placement`.
- `crates/pomone-ui/src/main.rs` — `wire_all_screens` (incl. `wire_placement`) + `UiState` init field.

### Review fixes (focused 2-reviewer, AI-E2-1)

Two independent adversarial reviews (persistence/read-path + wiring/spec lenses) — no High, one Medium, several Low. Applied:
- **[Medium] Peak explanation named the wrong group/week on mixed covered+open farms.** `show_peak_composition` picked the cover group by mere *presence* (`has_covered`) while the amber peak line picked it by *ratio* — so clicking an open-field overflow could list the greenhouse's February series. Fixed: a shared `binding_group(curve)` (ratio-based) drives both; `peak_composition` gained a `cover_group: Option<bool>` filter so the panel composes exactly the peak's group. (Tested.)
- **[Low] `occupancy_curve` / `peak_composition` could panic on `season_year == i32::MAX`** (`+ 1` overflow before `from_ymd_opt` rejects) — contradicted the module's no-panic contract. Fixed with `checked_add(1)`.
- **[Low] `PlannedPlanting::is_placed` doc comment was inverted** (code correct). Corrected.
- **[Low] Dead `col-*` wiring** (declared/forwarded/translated, never rendered) — now used: the unplaced list shows a column-header row.
- *Accepted as-is:* non-transactional place two-step (consistent with the codebase's service style; guarded by `is_placed`) and ISO/raw unplaced-list strings (consistent with sibling views).

## Change Log

- 2026-07-16 — Review fixes: binding-group consistency for the peak explanation (Medium), `checked_add` year guard, `is_placed` doc, column headers.
- 2026-07-16 — Story 3.2 implemented: migration 0013 (`placed_planting_id`, reversible placement) +
  `capacity_view.rs` (live covered/open occupancy curve, peak, composition) + placement service
  (convert planned→Planting, undo) + the `placement.slint` screen with full 3-layer wiring + a measured
  perf test (≤100 ms / 500 placements). All green (test/clippy/fmt); `capacity_view` at 98.3% coverage;
  app boots the wired screen on the demo DB. Built on the merged story-3.1 engine.
