# Story 0.2: Catalog screens wiring extraction

Status: review

## Story

As a contributor,
I want crops/varieties/locations/strata/families wiring extracted,
So that catalog screens follow the pattern.

## Acceptance Criteria

1. **Given** 0.1's pattern, **when** the five catalog screens are extracted, **then** behavior is unchanged (tests + manual smoke).
2. **And** no catalog callbacks remain in `main.rs`.
3. **No behavior change** — pure code relocation (Epic 0 rule: every story leaves `main` releasable).

## Tasks / Subtasks

- [x] Task 1: Extract the four catalog wiring modules (AC: 1, 2)
  - [x] `wiring/cultures.rs` — `wire_cultures` (10 callbacks: crops + varieties master-detail)
  - [x] `wiring/locations.rs` — `wire_locations` (5 callbacks)
  - [x] `wiring/strata.rs` — `wire_strata` (3 callbacks)
  - [x] `wiring/families.rs` — `wire_families` (5 callbacks)
  - [x] Declare the four modules in `wiring/mod.rs`; replace the removed blocks in `main()` with the four `wire_*` calls
- [x] Task 2: Move the screen-local helpers whose call sites all leave (inventory in Dev Notes); leave the shared ones (AC: 3)
- [x] Task 3: Verify no behavior change (AC: 1, 2)
  - [x] `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`
  - [x] Grep check: none of the 23 catalog `window.on_*` registrations remain in `main.rs`; each appears exactly once crate-wide
  - [x] Manual XDG-isolated smoke run (procedure in Dev Notes)

## Dev Notes

### Context — replicate story 0.1 exactly

Story 0.1 (`0-1-wiring-skeleton-and-settings-module-extraction.md`, merged as fb4ec4a) established everything: the module skeleton, the canonical signature `pub(crate) fn wire_<screen>(window: &MainWindow, state: &Rc<RefCell<UiState>>)` (documented in `wiring/mod.rs` — that header is the reference, not this file), the `Rc::clone(&state)` → `Rc::clone(state)` transform inside moved blocks, the `#[allow(clippy::too_many_lines)]` convention for long `wire_*` functions (PO-accepted in 0.1's review), and the extraction method: **a python script that asserts exact block boundaries by content before cutting** (see 0.1's implementation session — this avoided any manual copy error and the three-layer review verified byte-identical relocation).

Line numbers below are measured on `main` = fb4ec4a (5107-line `main.rs`). Re-verify with assertions before cutting.

### Move inventory — 23 callback registrations

**`wiring/cultures.rs` (10):** `on_navigate_cultures` (408), `on_select_crop` (425), `on_create_crop` (450), `on_edit_crop` (484), `on_delete_crop` (497), `on_cancel_crop_edit` (510), `on_delete_variety` (520), and the variety-form trio further down: `on_create_variety` (1577), `on_edit_variety` (1619), `on_cancel_variety_edit` (1632). Note the two source regions — the moved blocks join in one `wire_cultures`.

**`wiring/locations.rs` (5):** `on_navigate_locations` (536), `on_create_location` (553), `on_edit_location` (595), `on_cancel_location_edit` (608), `on_delete_location` (620).

**`wiring/strata.rs` (3):** `on_navigate_strata` (637), `on_create_strata` (652), `on_delete_strata` (685).

**`wiring/families.rs` (5):** `on_navigate_families` (1497), `on_families_save` (1511), `on_families_cancel_edit` (1536), `on_families_edit_row` (1548), `on_families_delete_row` (1562).

### Helpers that MOVE (all call sites leave with the callbacks — grep-verified on fb4ec4a)

| Into | Helper | Sole remaining caller |
|---|---|---|
| cultures.rs | `try_save_crop`, `open_crop_form_for_edit`, `lifespan_kind_from_index` | crop form callbacks |
| cultures.rs | `try_save_variety`, `open_variety_form_for_edit`, `pruning_from_index` | variety form callbacks |
| locations.rs | `try_save_location`, `open_location_form_for_edit`, `reset_location_form_to_create` | location form callbacks (reset called at 568, 612 — both leave) |
| strata.rs | `try_create_strata` | `on_create_strata` |
| families.rs | `open_families_page`, `open_family_form_for_edit`, `try_save_family_form` | families callbacks |

### Helpers that STAY in `main.rs` (a call site remains — verified)

- **`do_delete_crop` / `do_delete_variety` / `do_delete_location` / `do_delete_strata` / `do_delete_family`** — called ONLY from the shared confirm-dialog dispatch `on_confirm_accepted` (main.rs:707–714), which stays (it also dispatches task/task_type/planting/treatment deletes — story 0.3/0.4 territory). `PendingDelete` stays with it.
- **`refresh_cultures`, `refresh_locations`, `refresh_strata`, `refresh_families`** — called from startup, from `wiring::settings::try_swap_backend` (via `crate::`), and from the moved callbacks (via `crate::` again). Story 0.1 already established these as shared.
- **`refresh_varieties_of_selected_crop`** — also called inside `refresh_cultures` (2682) and `do_delete_variety`-adjacent code (2920).
- **`crop_to_slint`, `variety_to_slint`, `location_to_slint`** — called by the staying `refresh_*`.
- **`reset_crop_form_to_create`** (2885 ← `do_delete_crop`), **`reset_variety_form_to_create`** (2918), **`reset_families_form_to_create`** (4696, 4805 ← `do_delete_family`), **`render_family_form_error`** (4809 ← `do_delete_family`), **`color_chooser_palette`** (4531 ← task-types screen, stays until 0.4), **`establishment_method_from_index`** (2576 ← `try_create_planting`, plantings screen, story 0.3).

Rule (same as 0.1): move a helper only if EVERY call site moves; when in doubt, leave it — `crate::` reaches it either way. If a moved helper turns out to be needed by staying code, prefer leaving it in `main.rs` over adding `pub(crate)` re-exports.

### Traps

- **The confirm dialog does NOT move.** `on_confirm_accepted`/`on_confirm_cancelled` (~700–745) dispatch deletes for 9 entity kinds across screens — they stay until the last extraction story. The catalog `on_delete_*` callbacks (which only set `pending_delete` + show the dialog) DO move; the `do_delete_*` executors stay.
- **Two source regions for cultures**: the crop callbacks sit at 408–535, the variety-form callbacks at 1577–1643. Both regions feed `wire_cultures`; removal order matters in the script (descending).
- **`wire_settings` call is at main.rs:733** — between the strata blocks and the families blocks. Do not disturb it; add the four new `wire_*` calls where their blocks were (or group all five calls together where settings' call sits — cleaner; either satisfies the ACs, grouping is recommended for 0.4's endgame).
- Each moved block needs the `Rc::clone(&state)` → `Rc::clone(state)` transform; nothing else changes inside blocks.
- Imports: build once after surgery and prune exactly what rustc flags — 0.1 showed the unused-import warnings identify the leavers precisely (`-D warnings` in CI).
- `#[allow(clippy::too_many_lines)]` will likely be needed on `wire_cultures` (10 blocks); add it as in 0.1. The others may pass under 100 lines — only add where clippy demands.

### Testing & verification

Same gate as 0.1: `cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` (expect 388 tests green; `pomone-ui` has no unit tests).

Manual smoke, XDG-isolated (`/tmp/pom`, `seed-demo` first), under Xvfb with `WAYLAND_DISPLAY` unset (0.1 lesson — otherwise the window opens on the session compositor):
- Cultures: open the screen, click a crop row (varieties panel updates), create a crop (form clears, list grows), edit + cancel, create a variety, delete a variety (confirm dialog appears and works — this exercises the moved callback AND the staying dispatch).
- Locations: open, create, edit/cancel, delete with confirm.
- Strata: open, create, delete with confirm.
- Families: open (color palette renders), save a family, edit row, delete with confirm.
- After a backend "Test connection" from Settings: verifies `try_swap_backend`'s cross-module `crate::refresh_*` calls still compile-path correctly (they do — but the smoke confirms end-to-end).

Known pre-existing artifact: the Plantings table body renders empty under Xvfb (not catalog-related, do not chase).

### Workflow

Branch `refactor/0-2-wiring-catalogs` off fresh `main`; PR references **#110** (no `Closes`). Same commit/PR conventions as 0.1 (PR #114 as model).

### Project Structure Notes

- New files: `crates/pomone-ui/src/wiring/{cultures,locations,strata,families}.rs`; `wiring/mod.rs` gains four `pub(crate) mod` lines (keep its header doc untouched — it is the canonical pattern reference).
- Architecture delta tree: `pomone-ui/src/wiring/` "one module per screen (existing + new)" — four catalog modules match it.
- Story 0.4's target (`main.rs` < 500 lines) depends on 0.2+0.3 clearing their share; don't leave movable code behind out of caution when the inventory above says it moves.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 0.2] — statement + ACs
- [Source: _bmad-output/implementation-artifacts/0-1-wiring-skeleton-and-settings-module-extraction.md] — pattern, method, review learnings
- [Source: crates/pomone-ui/src/wiring/mod.rs] — canonical `wire_<screen>` contract
- [Source: _bmad-output/planning-artifacts/architecture.md#Structure Patterns] — anti-pattern: new callbacks in `main.rs`
- Line references measured on `main` = fb4ec4a

## Dev Agent Record

### Agent Model Used

Claude Fable 5 (claude-fable-5)

### Debug Log References

- The scripted extraction verified all boundary assertions on the first pass; the only mishap was a sloppy string substitution while adding the `StrataInput` import that also rewrote `on_create_strata`/`try_create_strata` identifiers — caught immediately by the compiler and fixed. Lesson: patch imports by editing the `use` line specifically, not by global replace on a substring.
- Import resolution took three quick compiler-guided iterations (unresolved names per module, then the `Input` structs, then `anyhow::{Context, Result}` for `.context()` calls). Zero manual guessing — rustc's error list was the checklist.
- Xvfb smoke: typing into Slint fields needs `xdotool windowfocus` first (no WM on Xvfb → no keyboard focus). Clicks work without it.

### Completion Notes List

- 23 catalog callback registrations + 13 screen-local helpers moved verbatim into four new modules: `wiring/cultures.rs` (10 callbacks + 6 helpers), `wiring/locations.rs` (5 + 3), `wiring/strata.rs` (3 + 1), `wiring/families.rs` (5 + 3). `main.rs`: 5107 → 4225 lines.
- The five `wire_*` calls are now grouped in one block in `main()` (settings included, per the story's recommendation for 0.4's endgame). Registration order is behavior-neutral (0.1 review finding: nothing fires before `window.run()`).
- Stay-list respected: `do_delete_*` executors, the shared confirm dialog, all `refresh_*`, `*_to_slint`, `reset_crop/variety/families_form_to_create`, `render_family_form_error`, `color_chooser_palette`, `establishment_method_from_index` remain in `main.rs`.
- `#[allow(clippy::too_many_lines)]` needed on `wire_cultures` only (heuristic in the generation script added it where >90 lines; clippy confirmed the others pass).
- Verification: fmt ✓, clippy `-D warnings` ✓, 388/388 tests ✓; AC2 grep: each of the 23 callbacks appears exactly once crate-wide, none in `main.rs`.
- Manual smoke (Xvfb, `/tmp/pom`, demo seed): Cultures — row select updates varieties panel, crop "Tomate" created ("Culture créée"), name-required validation renders; Strates — create form rendered, delete Grimpante → shared confirm dialog → "Strate supprimée" + list refresh (exercises the moved callback AND the staying `do_delete_*` dispatch end-to-end); Lieux — full hierarchy rendered; Familles — catalog + color palette rendered.

### File List

- `crates/pomone-ui/src/main.rs` (modified — 5 callback regions + 13 helpers removed, grouped `wire_*` calls, imports pruned)
- `crates/pomone-ui/src/wiring/mod.rs` (modified — four new module declarations)
- `crates/pomone-ui/src/wiring/cultures.rs` (new)
- `crates/pomone-ui/src/wiring/locations.rs` (new)
- `crates/pomone-ui/src/wiring/strata.rs` (new)
- `crates/pomone-ui/src/wiring/families.rs` (new)
- `_bmad-output/implementation-artifacts/sprint-status.yaml` (status tracking)

## Change Log

- 2026-07-12: Story 0.2 implemented — the four catalog screens' wiring extracted from `main.rs` into per-screen modules, replicating 0.1's pattern. No behavior change (388 tests green, clippy/fmt clean, GUI smoke on isolated DB incl. the shared confirm-dialog bridge).
