# Story 0.3: Plantings, detail and treatments wiring extraction

Status: review

## Story

As a contributor,
I want plantings, planting-detail and treatments wiring (incl. split/move dialogs) extracted,
So that the planting family follows the pattern.

## Acceptance Criteria

1. **Given** 0.1's pattern, **when** the three planting screens are extracted, **then** behavior is unchanged.
2. **And** no planting callbacks remain in `main.rs`.
3. **No behavior change** — pure code relocation (Epic 0 rule: every story leaves `main` releasable).

## Tasks / Subtasks

- [x] Task 1: Extract the three planting wiring modules (AC: 1, 2)
  - [x] `wiring/plantings.rs` — `wire_plantings` (4 callbacks)
  - [x] `wiring/crop_map.rs` — `wire_crop_map` (6 callbacks, incl. split/move dialogs)
  - [x] `wiring/planting_detail.rs` — `wire_planting_detail` (7 callbacks, incl. harvests + treatments)
  - [x] Declare the modules in `wiring/mod.rs`; add the three `wire_*` calls to the grouped block in `main()`
- [x] Task 2: Move the screen-local helpers whose call sites all leave (inventory in Dev Notes) (AC: 3)
- [x] Task 3: Verify no behavior change (AC: 1, 2)
  - [x] `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (389 tests incl. the new size gate)
  - [x] Grep check: none of the 17 planting `window.on_*` registrations remain in `main.rs`; each exactly once crate-wide
  - [x] Manual XDG-isolated smoke run (procedure in Dev Notes)

## Dev Notes

### Context — third replication of the 0.1 pattern

Stories 0.1 (fb4ec4a) and 0.2 (a3171b8) are merged; the pattern, the scripted-extraction method (python with content assertions on every boundary, cut descending), the `Rc::clone(&state)` → `Rc::clone(state)` transform, and the compiler-guided import resolution are all established — see both story files' Dev Agent Records. 0.2's key lesson: **never patch imports with a global substring replace** (it corrupted `on_create_strata` once); edit the `use` line precisely. Line numbers below measured on `main` = 98b381d (4225-line `main.rs`; the #117 size-gate merge added only a test file).

### Move inventory — 17 callback registrations

**`wiring/plantings.rs` (4):** `on_navigate_plantings` (359), `on_create_planting` (376), `on_planting_row_clicked` (558), `on_plantings_sort` (571).

**`wiring/crop_map.rs` (6):** `on_navigate_crop_map` (445), `on_crop_map_bar_clicked` (462), `on_crop_map_move_to` (478), `on_crop_map_split_clicked` (505), `on_crop_map_split_confirm` (522), `on_crop_map_split_cancel` (545).

**`wiring/planting_detail.rs` (7):** `on_record_harvest` (593), `on_record_treatment` (627), `on_delete_treatment` (664), `on_detail_go_back` (681), `on_detail_change_status` (711), `on_detail_delete_planting` (748), `on_detail_task_clicked` (1024).

Note the source regions: 359–392 (plantings nav+create), 445–556 (crop map, contiguous), 558–764 (row click, sort, detail block), and the isolated `on_detail_task_clicked` at 1024–1038 (sits between task callbacks that stay for 0.4). Cut descending.

### Helpers that MOVE (all call sites leave — grep-verified on 98b381d)

| Into | Helper | Call sites (all leaving) |
|---|---|---|
| plantings.rs | `try_create_planting` (~2110) | `on_create_planting` only |
| plantings.rs | `establishment_method_from_index` | `try_create_planting` (2136); the other grep hit (1647) is a comment inside `apply_translations`, not code |
| crop_map.rs | `refresh_crop_map` (~4030) | callbacks at 450/496/531 only (NOT in `try_swap_backend`'s reload list — verified); hit at 1334 is a comment |
| crop_map.rs | `bar_to_slint` | `refresh_crop_map` only |
| crop_map.rs | `prefill_split_form`, `try_confirm_split` | split callbacks only (hit 4106 is a comment) |
| planting_detail.rs | `try_record_harvest`, `try_record_treatment` | their callbacks only |
| planting_detail.rs | `status_from_index` | `on_detail_change_status` (717); hit 1770 is a comment |

### Helpers that STAY in `main.rs` (verified — a call site remains)

- **`open_planting_detail`** — called by `on_planting_row_clicked` (562, leaves) AND `on_task_milestone_clicked` (1016, **stays for 0.4**). Leave it in `main.rs`; the moved callback reaches it via `crate::`.
- **`refresh_planting_detail`** — called from `wiring::settings::on_units_saved`, from `open_planting_detail`, and from detail callbacks. Shared.
- **`refresh_plantings`** — startup, swap, units, confirm dispatch (698), do_delete (2798). Shared.
- **`sort_planting_rows`, `to_slint_row`, `to_gantt_bar`** — called by `refresh_plantings`.
- **`status_to_index`** — called by `refresh_planting_detail` (3132).
- **`do_delete_planting`, `do_delete_treatment_row`** — confirm-dialog dispatch (420/425), which stays until 0.4.
- Parsers/validators (`parse_u16`, `optional_text`, `validate_*`, `parse_id` usage…) — multi-screen, stay.

Rule unchanged: move a helper only if EVERY call site moves; comments don't count as call sites but update them if they'd mislead.

### Traps

- **`on_detail_task_clicked` (1024) is marooned mid-task-block** — its neighbours (`on_task_milestone_clicked` 1012, `on_task_new_requested` 1040) stay for 0.4. Cut its exact block (1023?–1038); assert both neighbours' registrations survive in `main.rs` afterwards.
- **It calls `open_task_form_for_edit`** (a 0.4 helper staying in `main.rs`) — fine via `crate::`, no visibility change needed.
- The detail callbacks use `s.detail_planting_id` and `detail_previous_page` from `UiState` — no change, they travel by `crate::UiState` field access.
- `on_crop_map_move_to` uses `move_planting_to_location` + `localize_app_error` + `FluentArgs` — check module imports.
- After removal, add the three calls to the **existing grouped `wire_*` block** in `main()` (~line 400).
- Size gate: this story should bring `main.rs` to roughly ~3300 lines — still above the 3000 cap, so **the `EXEMPT` entry in `crates/pomone-app/tests/source_file_size.rs` stays until 0.4** (do not remove it in this story; 0.4 removes it when `main.rs` drops under 500).
- `#[allow(clippy::too_many_lines)]`: add ONLY where clippy demands (0.2 review lesson — the >90-line heuristic overfired; verify per module by running clippy without it first).

### Testing & verification

Gate: fmt + clippy `-D warnings` + `cargo test --workspace` (389 tests now — the size gate joined in #117).

Manual smoke, XDG-isolated + Xvfb (`WAYLAND_DISPLAY` unset; `xdotool windowfocus` before typing; delete buttons are disabled for in-use entries — 0.2 lessons):
- Plantings: open list (rows + Gantt), sort by clicking a column header, create a planting (defaults prefilled), click a row → detail opens.
- Detail: back button returns to list; change status (status line confirms); record a yearly harvest on a perennial (demo has Pommier) or record a treatment on any planting; delete the treatment via confirm dialog.
- Crop map: open (lanes render), click a bar (selection toggles), move a planting to another location (bar moves lane), split a planting 50/50 (two bars), cancel path.
- Cross-check after mutations: Plantings list refreshes coherently.
- Known pre-existing artifact: Plantings table body can render empty under Xvfb while Gantt is fine — do not chase.

### Workflow

Branch `refactor/0-3-wiring-plantings` off fresh `main`; PR references **#110** (no `Closes`). Same conventions as PR #114/#115.

### Project Structure Notes

- New: `crates/pomone-ui/src/wiring/{plantings,crop_map,planting_detail}.rs`; `wiring/mod.rs` gains three `pub(crate) mod` lines (header doc untouched — canonical reference).
- After 0.3, `main.rs` keeps: bootstrap, home/manual, confirm dialog, tasks/calendar/agenda/task-types/task-form wiring (0.4), `apply_translations` + shared helpers.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 0.3] — statement + ACs
- [Source: _bmad-output/implementation-artifacts/0-2-catalog-screens-wiring-extraction.md] — method + review learnings
- [Source: crates/pomone-ui/src/wiring/mod.rs] — canonical `wire_<screen>` contract
- [Source: crates/pomone-app/tests/source_file_size.rs] — size gate + EXEMPT rule (#116)
- Line references measured on `main` = 98b381d

## Dev Agent Record

### Agent Model Used

Claude Fable 5 (claude-fable-5)

### Debug Log References

- Boundary assertions caught two mis-measured line anchors on the first script run (crop-map end at 552 not 553; detail region split at 587/588) — corrected before any cut was made; the assertion-first method worked as designed.
- Import resolution: three compiler-guided iterations (module names list, anyhow Context/Result + slint ModelRc/VecModel + std FromStr for crop_map, pomone_app::services for planting_detail). No manual guessing.
- clippy demanded `too_many_lines` allows on `wire_crop_map` (106) and `wire_planting_detail` (163) — added exactly there; `wire_plantings` passes without (0.2 review lesson applied).

### Completion Notes List

- 17 callback registrations + 9 screen-local helpers moved verbatim into three new modules: `wiring/plantings.rs` (4 cb + `try_create_planting`, `establishment_method_from_index`), `wiring/crop_map.rs` (6 cb + `refresh_crop_map`, `bar_to_slint`, `prefill_split_form`, `try_confirm_split`), `wiring/planting_detail.rs` (7 cb + `try_record_harvest`, `try_record_treatment`, `status_from_index`). Only textual change inside moved code: `Rc::clone(&state)` → `Rc::clone(state)`.
- `main.rs`: 4225 → 3510 lines. Still above the 3000 cap → the `EXEMPT` entry in the size-gate test stays (removed at 0.4 as planned); the gate passes (389/389 tests).
- Stay-list respected: `open_planting_detail` (also called by 0.4's `on_task_milestone_clicked`), `refresh_planting_detail`, `refresh_plantings`, `sort_planting_rows`/`to_slint_row`/`to_gantt_bar`, `status_to_index`, `do_delete_planting`/`do_delete_treatment_row`, confirm dialog. The marooned `on_detail_task_clicked` was cut cleanly; both neighbour registrations verified still in `main.rs`.
- Verification: fmt ✓, clippy `-D warnings` ✓, 389/389 ✓; AC2 grep: each of the 17 callbacks exactly once crate-wide, none in `main.rs`.
- Manual smoke (Xvfb, `/tmp/pom`, fresh seed): Plantings list + row click → detail opens (table populated this run); Crop map renders 11 plantings, bar select → "Diviser" dialog pre-filled 50/50 → split confirmed (bar appears on second lane); Detail: treatment form validations ("Le nom est requis", "Nombre invalide"), treatment recorded ("Traitement enregistré"), treatment deleted via shared confirm dialog ("Traitement supprimé"), status change ("Statut mis à jour"). Not exercised: harvest record (perennial-only path, same moved shape as treatment), move-to picker (same dispatch as split), go-back/delete-planting (same 3-line shape).

### File List

- `crates/pomone-ui/src/main.rs` (modified — 4 callback regions + 9 helpers removed, 3 wire calls added to the grouped block, imports pruned)
- `crates/pomone-ui/src/wiring/mod.rs` (modified — three new module declarations)
- `crates/pomone-ui/src/wiring/plantings.rs` (new)
- `crates/pomone-ui/src/wiring/crop_map.rs` (new)
- `crates/pomone-ui/src/wiring/planting_detail.rs` (new)
- `_bmad-output/implementation-artifacts/sprint-status.yaml` (status tracking)

## Change Log

- 2026-07-12: Story 0.3 implemented — plantings, crop-map (split/move) and planting-detail (harvests/treatments) wiring extracted from `main.rs`, third replication of the 0.1 pattern. No behavior change (389 tests green, GUI smoke incl. split dialog and treatment lifecycle).
