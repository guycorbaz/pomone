# Story 0.4: Tasks, calendar, agenda and harvests wiring extraction

Status: done

## Story

As a contributor,
I want the remaining screens extracted,
So that `main.rs` shrinks to bootstrap + wiring calls.

## Acceptance Criteria

1. **Given** 0.1–0.3 merged, **when** the remaining screens are extracted, **then** `main.rs` contains only startup, config, window lifecycle and `wire_*` calls (**< 500 lines**).
2. **And** the full suite is green.
3. **And** the `EXEMPT` entry for `main.rs` is removed from the size-gate test (#116) — every source file now honors the 2000/3000 rule.
4. **No behavior change** — pure code relocation.

## Tasks / Subtasks

- [x] Task 1: Extract the five remaining wiring modules (AC: 1)
  - [x] `wiring/home.rs` — `wire_home` (2 cb: navigate_home, open_manual) + `find_manual_path`
  - [x] `wiring/confirm.rs` — `wire_confirm` (2 cb: confirm_accepted/cancelled) + the 9 `do_delete_*` executors (their ONLY caller is the dispatch)
  - [x] `wiring/task_calendar.rs` — `wire_task_calendar` (9 cb: navigate_tasks, prev/next_month, go_today, toggle_category, select_all_categories, toggle_milestones, task_edit_requested, task_rescheduled, task_milestone_clicked) + calendar-local helpers (`prev_month`, `next_month`, `first_of_month`, `weekday_offset_mon`, `kind_to_int`, `kind_glyph_key`)
  - [x] `wiring/agenda.rs` — `wire_agenda` (2 cb: navigate_agenda, agenda_task_clicked)
  - [x] `wiring/task_form.rs` — `wire_task_form` (4 cb: task_new_requested, task_form_save/cancel/delete) + form helpers (`populate_task_form_options`, `open_task_form_for_create`, `open_task_form_for_edit`†, `try_save_task_form`, `populate_recurrence_units`, `default_end_date_iso`, `render_task_form_error`†)
  - [x] `wiring/task_types.rs` — `wire_task_types` (6 cb) + catalog helpers (`refresh_task_types`, `reset_task_types_form_to_create`, `open_task_types_for_create`, `open_task_type_form_for_edit`, `try_save_task_type_form`, `render_task_type_form_error`, `populate_task_type_categories`)
- [x] Task 2: Move the shared layer out of `main.rs` into dedicated modules, with `pub(crate) use` re-exports in `main.rs` so the nine EXISTING wiring modules keep compiling untouched (AC: 1, 4)
  - [x] `src/state.rs` — `UiState`, `PendingDelete`
  - [x] `src/translations.rs` — `apply_translations` (~600 lines), `apply_unit_labels`, `area_unit_index`, `mass_unit_index`, `holiday_region_index`
  - [x] `src/refresh.rs` — every shared `refresh_*` + snapshots + row converters + `open_planting_detail`, `status_to_index`, `reset_*_form_to_create`, `color_chooser_palette`, `category_*` helpers, `all_category_keys`, `refresh_after_task_form`
  - [x] `src/forms.rs` — `FormError`, all `validate_*`, `parse_*`, `optional_text`, `today_iso`, `localize_app_error`, `localize_domain_error`, `render_form_error`, `render_family_form_error`, `parse_hex_color`, `usize_to_i32`, `i32_to_usize`
- [x] Task 3: Remove the `main.rs` entry from `EXEMPT` in `crates/pomone-app/tests/source_file_size.rs` (AC: 3)
- [x] Task 4: Verify (AC: 1, 2, 4)
  - [x] `main.rs` < 500 lines; no `.rs` file > 2000 lines (aim) / 3000 (gate)
  - [x] fmt + clippy `-D warnings` + `cargo test --workspace` (389 tests, size gate now unexempted)
  - [x] Zero `window.on_*` registrations left in `main.rs`; each of the 26 moved callbacks exactly once crate-wide
  - [x] Manual XDG-isolated smoke run (procedure in Dev Notes)

† `open_task_form_for_edit` and `render_task_form_error` are ALSO called from other wiring modules (`planting_detail`, `task_calendar`, `agenda`) via `use crate::{...}` — the re-export layer (Task 2) covers them; they live in `wiring/task_form.rs` and are re-exported from `main.rs`.

### Review Findings

- [x] [Review][Patch] Gratuitous `#[allow(clippy::too_many_lines)]` on `main()` + stale comment ("four panes' worth of callbacks" — `main()` no longer registers callbacks inline). Removed both; clippy verified clean without it. Found by Acceptance Auditor. [crates/pomone-ui/src/main.rs]
- [x] [Review][Patch] Completion Notes said the shared layer moved into "four root modules" — corrected to note the five cross-wiring helpers that stay in `wiring/task_form.rs`/`task_types.rs` (re-exported); allow-list count corrected. Found by Blind Hunter (Low). [story file]
- [x] [Review][Dismissed] Three-layer review confirmed pure relocation (no High/Med): 26 callbacks 1:1, ~85 fn bodies byte-identical (only rustfmt signature reflow differs), `Rc::clone` transform consistent, all re-exports resolve, 33 `UiState` fields preserved, 389 tests. Low style notes accepted by design: root re-exports from child wiring modules is the deliberate "re-export trick" that avoids editing the nine existing modules; uniform `pub(crate)` from the mechanical extraction is harmless (no `unreachable_pub`/unused lint fires); the "unobservable risk" on unchanged modules is disproven by the green build.

## Dev Notes

### The re-export trick (the story's key decision)

The nine existing `wiring/*.rs` modules import shared helpers with `use crate::{refresh_plantings, FormError, UiState, …}`. Moving those helpers into `src/{state,translations,refresh,forms}.rs` would normally mean editing every module's imports. Instead, `main.rs` (crate root) keeps a single block:

```rust
mod forms;
mod refresh;
mod state;
mod translations;

pub(crate) use forms::*;
pub(crate) use refresh::*;
pub(crate) use state::*;
pub(crate) use translations::*;
```

`crate::X` keeps resolving for every existing module — zero edits outside `main.rs` and the new files. `pub(crate)` avoids the `unreachable_pub` lint. Items inside the new modules need `pub(crate)` visibility (they were private at the root; from a child module they must be exported to be re-exported). The compiler will list every item needing the bump — mechanical.

### Inventory measured on `main` = ddeb5ed (3510-line main.rs)

Callback registrations remaining (26): lines 317–831 (contiguous block: home 317/331, confirm 368/391, tasks 404–673, task_form 677–730, task_types 732–831). After extraction `main()` keeps only startup + the grouped `wire_*` calls (now 14 of them) + geometry + run.

Shared-layer functions with their lines: `apply_translations` 865, `refresh_bed_usage` 1477, `polyline_path` 1519, `PlantingsSnapshot` 1534, `refresh_plantings` 1543, `sort_planting_rows` 1633, `to_slint_row` 1656, `to_gantt_bar` 1683, parsers 1717–1740, `CulturesSnapshot`/`refresh_cultures`/`refresh_varieties_of_selected_crop`/converters 1741–1825, `reset_crop_form_to_create` 1826, `do_delete_crop/variety/location` 1840/1873/1907, `reset_variety_form_to_create` 1935, `parse_u8`/`parse_optional_decimal` 1950/1956, locations 1967–2032, `today_iso` 2033, `FormError`+validators 2040–2124, localizers 2125–2182, `render_form_error`/`render_task_form_error` 2183/2202, `do_delete_strata/task/task_type` 2220/2245/2264, `status_to_index` 2287, `do_delete_planting` 2300, `refresh_strata` 2330, `open_planting_detail` 2353, `do_delete_treatment_row` 2378, unit/holiday index helpers ~2400–2460, `parse_i32` 2461, `refresh_after_task_form` 2471, `refresh_agenda` 2486, `refresh_planting_detail` 2509, calendar helpers 2613–2682, `refresh_task_calendar` 2683, task-form helpers 2841–3128, `refresh_task_types`→`render_task_type_form_error` 3129–3277, `refresh_families` 3278, `reset_families_form_to_create` 3299, `color_chooser_palette` 3315, `render_family_form_error` 3331, `do_delete_family` 3351, category helpers 3375–3426, `refresh_task_filter_chips` 3427, `parse_hex_color` 3462, `usize_to_i32`/`i32_to_usize` 3489/3495, `_config_for_dev_fallback` 3500.

### Ventilation rules

- **Callbacks + screen-local helpers → their `wiring/<screen>.rs`** (0.1–0.3 pattern, `Rc::clone(&state)` → `Rc::clone(state)` in blocks).
- **`do_delete_*` (9) → `wiring/confirm.rs`**: their only caller is the confirm dispatch; they reference `refresh_*` via `crate::` (re-exported).
- **Shared helpers → `src/{state,translations,refresh,forms}.rs`** with `pub(crate)` items + root re-exports.
- **`refresh_agenda`**: called by `on_navigate_agenda` AND `refresh_after_task_form` (shared) → goes to `refresh.rs`, not agenda.rs.
- **`refresh_task_calendar`, `refresh_task_filter_chips`, `all_category_keys`, `category_*`**: called across settings/planting_detail/task modules and `main()` startup → `refresh.rs`.
- **Stays in `main.rs`**: module docs, `mod generated`, `mod wiring`, the four `mod` + `pub(crate) use` blocks, `main()`, `restore_window_geometry`, `save_window_geometry`, `_config_for_dev_fallback`.
- **`main()` startup calls** (`apply_translations`, `refresh_*`, `all_category_keys` in the UiState initializer) keep working through the re-exports.

### Traps

- **`UiState` field visibility**: moving the struct to `state.rs` makes its private fields inaccessible from other modules — but every access today is from wiring modules/helpers that were ALREADY sibling code at the root. Fields must become `pub(crate)`. Mechanical: add `pub(crate)` to every field (the compiler lists them).
- **`generated` module stays in `main.rs`**: `slint::include_modules!()` is fine at the root; new modules import `crate::generated::…` like the existing nine.
- **Doc-comment cross-references**: fix any `[`link`]` that crosses the new module boundaries (0.3 review lesson — rewrite as plain prose with module path).
- **Import prune in `main.rs` will be massive** — most of the `use pomone_app::{…}` list leaves. Let the compiler enumerate; expect 2–3 iterations (0.2/0.3 method). Never patch imports by global substring replace (0.2 incident).
- **Size check on the NEW files**: `refresh.rs` lands ~1400 lines and `translations.rs` ~700 — both under the 2000 target. If refresh.rs exceeds 2000, split `refresh_task_calendar`+calendar chips into `refresh_calendar.rs`.
- `#[allow(clippy::too_many_lines)]`: expect it on `wire_task_calendar` (9 blocks) and possibly `apply_translations` (already carries one — keep it) — add only where clippy demands.
- The story is big; commit in two steps if useful (Task 1 then Task 2), but the PR is one unit and CI must be green at the end (intermediate commits need not be).

### Testing & verification

- Gate: fmt + clippy `-D warnings` + `cargo test --workspace` (389; the size gate must pass WITHOUT the main.rs exemption — Task 3).
- AC2-style grep: zero `window.on_` in `main.rs`; 26 moved callbacks exactly once each.
- Manual smoke (Xvfb, `/tmp/pom`, fresh seed, `windowfocus` before typing):
  - Home: counts render; Help/manual button shows a status line (found or not-found — both fine).
  - Calendar: navigate months (prev/next/today), toggle a category chip, toggle milestones, click a milestone → detail (route "tasks"), back → calendar (exercises the go_back "tasks" branch missed in 0.3).
  - Agenda: open, click a task → task form opens; cancel.
  - Task form: create a task (+ Nouvelle tâche), save; edit it from the calendar; delete it via confirm dialog.
  - Task types: open catalog, create a type, edit row, delete via confirm.
  - Regression sweep of extracted screens: settings language toggle, cultures row select, plantings sort, crop-map render (one click each — the shared layer moved under them).

### Workflow

Branch `refactor/0-4-wiring-tasks` off fresh `main`; PR references **#110**. This story completes AR1's wiring refactor: after merge, announce Epic 0 stories 0.1–0.4 done in #110.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 0.4] — statement + ACs (< 500 lines)
- [Source: _bmad-output/implementation-artifacts/0-3-*.md] — method + review learnings
- [Source: crates/pomone-app/tests/source_file_size.rs] — EXEMPT removal (Task 3, #116)
- Line references measured on `main` = ddeb5ed

## Dev Agent Record

### Agent Model Used

Claude Fable 5 (claude-fable-5), then Opus 4.8 for the final smoke + finalization.

### Debug Log References

- Two-phase scripted extraction. Phase 1 (six `wiring/*.rs`): boundary assertions caught the `open_manual` block (its comment sits *inside* the block) — widened the brace-rewind window from 6 to 12 lines. Phase 2 (shared layer → `state/translations/refresh/forms.rs`): 61 items relocated, `pub(crate)` applied to items + struct fields.
- The re-export layer (`pub(crate) use forms::*` …) worked as designed — zero edits to the nine pre-existing wiring modules. Compiler then flagged 283 → 65 → 18 → 0 errors across ~5 import iterations (calendar helpers `first_of_month`/`weekday_offset_mon`/`kind_*` had to move to `refresh.rs` since `refresh_task_calendar` uses them; plot consts too; `services`, `Datelike`, `anyhow::{Context,Result}` per module).
- Automated unused-import prune removed 494 names in one pass but left `use ;` husks where a whole `use pomone_app::{…}` emptied — cleaned with a regex pass.
- clippy: wildcard-import lint forbids `pub(crate) use module::*` at the root — expanded all four to explicit lists. `too_many_lines` allows land on `apply_translations`, `try_save_task_form`, `refresh_task_calendar` and `wire_task_calendar` — each verified demanded by clippy (removal test). No allow on `main()` (review finding — removed).

### Completion Notes List

- **`main.rs`: 3510 → 206 lines** (AC1: < 500 ✓). Contains only module docs, `mod generated`, the `mod` + `pub(crate) use` re-export block, `main()`, `restore/save_window_geometry`, `_config_for_dev_fallback`. **Zero `window.on_*` in `main.rs`.**
- 26 callbacks + their screen-local helpers → six new `wiring/` modules (home, confirm, task_calendar, agenda, task_form, task_types). The 9 `do_delete_*` executors → `wiring/confirm.rs` (sole caller is the dispatch).
- Shared layer → four root modules (plus five helpers that stay in `wiring/task_form.rs` and `wiring/task_types.rs`, re-exported from the root because their callers span several wiring modules): `state.rs` (124: `UiState`+`PendingDelete`, fields `pub(crate)`), `translations.rs` (682: `apply_translations` et al.), `refresh.rs` (1014: all `refresh_*`, snapshots, converters, `open_planting_detail`, calendar helpers, category helpers, plot consts), `forms.rs` (302: `FormError`, validators, parsers, localizers). Re-exported from the root so every `crate::X` keeps resolving — the nine existing wiring modules are untouched.
- **AC3: size-gate `EXEMPT` list emptied** — every `.rs` now honors 2000/3000 (largest: `refresh.rs` 1014). Verified: no file > 1500 lines.
- Verification: fmt ✓, clippy `-D warnings` ✓, **389/389 tests** (size gate now unexempted) ✓; each of the 26 callbacks registered exactly once crate-wide.
- Manual smoke (Xvfb, `/tmp/pom`, fresh seed) — every extracted screen + the shared layer under them: Home startup counts; Calendar month nav (prev/next/today), category-chip toggle, milestone click → detail (route "tasks") → **back → calendar (the "tasks" branch missed in 0.3)**; Agenda list → task click → edit form → delete via confirm dialog (85→84 tasks); Task-types catalog create ("Paillage") + delete via confirm (back to 9); **+ Nouvelle tâche → create → save (11→12 tasks, calendar refreshed via `refresh_after_task_form`)**; regression sweep — settings language toggle (whole sidebar flips FR↔EN, proving relocated `apply_translations` works), cultures, plantings, crop-map all navigable.

### File List

- `crates/pomone-ui/src/main.rs` (modified — 3510 → 206 lines)
- `crates/pomone-ui/src/state.rs` (new)
- `crates/pomone-ui/src/translations.rs` (new)
- `crates/pomone-ui/src/refresh.rs` (new)
- `crates/pomone-ui/src/forms.rs` (new)
- `crates/pomone-ui/src/wiring/mod.rs` (modified — six new module declarations)
- `crates/pomone-ui/src/wiring/home.rs` (new)
- `crates/pomone-ui/src/wiring/confirm.rs` (new)
- `crates/pomone-ui/src/wiring/task_calendar.rs` (new)
- `crates/pomone-ui/src/wiring/agenda.rs` (new)
- `crates/pomone-ui/src/wiring/task_form.rs` (new)
- `crates/pomone-ui/src/wiring/task_types.rs` (new)
- `crates/pomone-app/tests/source_file_size.rs` (modified — EXEMPT emptied)
- `_bmad-output/implementation-artifacts/sprint-status.yaml` (status tracking)

## Change Log

- 2026-07-12: Story 0.4 implemented — the last six screens' wiring extracted and the shared helper layer split into state/translations/refresh/forms modules with root re-exports. `main.rs` 3510 → 206 lines; size-gate exemption removed. Completes Epic 0's wiring refactor (AR1). No behavior change (389 tests, full GUI smoke).
