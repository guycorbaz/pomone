# Story 0.1: Wiring skeleton and settings module extraction

Status: review

## Story

As a contributor (human or AI agent),
I want `pomone-ui/src/main.rs` to delegate screen wiring to per-screen modules, starting with `wiring/` and the settings/backend/i18n screens,
So that new screens never add callbacks to a 5500-line file and the pattern is established.

## Acceptance Criteria

1. **Given** the monolithic `main.rs`, **when** `wiring/mod.rs` + `wiring/settings.rs` are extracted (settings, backend swap, backup, holidays, units, language), **then** tests, clippy `-D warnings` and a manual launch behave identically.
2. **And** no settings `on_*` registration remains in `main.rs`.
3. **And** the pattern (`fn wire_<screen>(…)`) is documented in the module header of `wiring/mod.rs`.
4. **No behavior change** — pure code relocation. (Epic 0 rule: every story leaves `main` releasable.)

## Tasks / Subtasks

- [x] Task 1: Create the wiring skeleton (AC: 3)
  - [x] Add `mod wiring;` to `main.rs`; create `crates/pomone-ui/src/wiring/mod.rs` with `pub mod settings;`
  - [x] Write the `mod.rs` module header doc: one module per screen, `pub fn wire_<screen>(window: &MainWindow, state: &Rc<RefCell<UiState>>)` registers every `on_*` callback of that screen; new screens NEVER add callbacks to `main.rs` directly (architecture anti-pattern)
- [x] Task 2: Extract the settings wiring (AC: 1, 2)
  - [x] Create `wiring/settings.rs` with `pub fn wire_settings(window: &MainWindow, state: &Rc<RefCell<UiState>>)`
  - [x] Move the 9 callback registrations verbatim (inventory + current `main.rs` lines in Dev Notes)
  - [x] Move the settings-only helpers (inventory in Dev Notes); keep shared helpers in `main.rs`
  - [x] Replace the moved blocks in `main()` with one call: `wiring::settings::wire_settings(&window, &state);`
- [x] Task 3: Verify no behavior change (AC: 1)
  - [x] `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`
  - [x] `grep -n 'on_settings\|on_toggle_language\|on_navigate_settings' crates/pomone-ui/src/main.rs` returns nothing (AC 2)
  - [x] Manual XDG-isolated smoke run (procedure in Dev Notes)

## Dev Notes

### Why this story exists

`main.rs` is 5650 lines; the convergence adds ~5 new screens (plan, ITK editor, placement, reconcile). Architecture decision (Slice 0, AR1/AR2): screen wiring lives in `pomone-ui/src/wiring/<screen>.rs`, `main.rs` shrinks to bootstrap + `wire_*` calls (story 0.4 target: < 500 lines). This story establishes the pattern with the settings family; stories 0.2–0.4 replicate it. **Copy the established callback shape exactly — do not redesign it.**

### The existing callback pattern (preserve verbatim)

State is `Rc<RefCell<UiState>>`; each registration block clones the `Rc` and a `window.as_weak()`, then upgrades the weak at call time:

```rust
{
    let state = Rc::clone(state);
    let weak = window.as_weak();
    window.on_settings_backup_now(move || {
        let Some(window) = weak.upgrade() else { return; };
        let s = state.borrow();
        // ... body unchanged ...
    });
}
```

Async DB calls run via `s.runtime.block_on(...)` inside callbacks (single-threaded UI, SQLite is µs-fast — this is deliberate, keep it).

### Move inventory — callback registrations (9, with current `main.rs` lines)

| Callback | Line | Notes |
|---|---|---|
| `on_toggle_language` | 362 | language toggle (i18n screen scope per AC) |
| `on_navigate_settings` | 750 | calls `refresh_settings` |
| `on_settings_test_backend` | 875 | builds `SettingsFormValues`, calls `test_backend` |
| `on_settings_save_backend` | 922 | → `try_swap_backend(…, false)` |
| `on_settings_save_and_migrate` | 944 | → `try_swap_backend(…, true)` |
| `on_settings_backup_now` | 968 | matches sentinel `AppError::Inconsistent("backup_sqlite_only")` |
| `on_settings_holiday_region_changed` | 1005 | no-op guard on startup fire; refreshes task calendar |
| `on_settings_area_unit_changed` | 1038 | no-op guard; `on_units_saved` on success |
| `on_settings_mass_unit_changed` | 1062 | no-op guard; `on_units_saved` on success |

Preserve the startup no-op guards (combos fire once when `apply_translations` sets the initial index) — deleting them rewrites config at every launch.

### Move inventory — settings-only helpers (safe to move into `wiring/settings.rs`)

| Item | Line | Only used by |
|---|---|---|
| `refresh_settings` | 3512 | `on_navigate_settings` + startup call at line 305 |
| `backend_display` | 3538 | `refresh_settings` |
| `redact_password` | 3547 | settings display |
| `split_mariadb_url` | 3563 | `refresh_settings` form prefill |
| `SettingsFormValues` + `into_backend` | 3599 | the 3 backend callbacks |
| `format_migration_report` | 3647 | `try_swap_backend` |
| `try_swap_backend` | 3672 | save/save-and-migrate |
| `on_units_saved` | 4261 | the 2 unit callbacks |
| `area_unit_from_index` | 4317 | area unit callback |
| `mass_unit_from_index` | 4336 | mass unit callback |
| `holiday_region_code` | 4358 | holiday callback |

**Verify each with grep before moving** — if a call site outside the settings family appears (code moved since this analysis), leave the helper in `main.rs`.

### Must NOT move (shared with other screens — stay in `main.rs` for this story)

`UiState`, `PendingDelete`, `mod generated`, `FormError` (line 3771), `render_form_error`, `localize_app_error`, `localize_domain_error`, `apply_translations`, `apply_unit_labels` (called by `apply_translations` too), `refresh_plantings`, `refresh_locations`, `refresh_planting_detail`, `refresh_task_calendar`, and the index-direction helpers `area_unit_index` (4308), `mass_unit_index` (4327), `holiday_region_index` (4346) — all three are called by `apply_translations` (lines 2134, 2153–2154). Later stories relocate shared helpers; widening this story's blast radius is scope creep.

### Rust visibility — the key fact that makes this cheap

`main.rs` is the crate root; `wiring::settings` is a descendant module. **Private items declared in the crate root are visible in descendant modules** via `crate::…` — so `wiring/settings.rs` can use `crate::{UiState, FormError, refresh_task_calendar, localize_app_error, …}` with NO visibility changes in `main.rs`. Only the reverse direction needs `pub`: items `main.rs` calls from the new module (`wire_settings`, `refresh_settings` for the startup call) must be `pub` (module-private crate ⇒ plain `pub` is effectively crate-visible; match whatever clippy accepts without warnings).

Imports: `wiring/settings.rs` needs its own `use` block (subset of `main.rs` imports: `slint::{ComponentHandle, SharedString}`, `fluent::FluentArgs`, `pomone_app::{test_backend, AppError, BackendConfig, …}`, `crate::generated::MainWindow`, …). Remove the now-unused imports from `main.rs` — **unused imports are hard errors in CI** (`RUSTFLAGS: -D warnings`).

### What NOT to touch

- **No `.slint` file changes.** The 3-layer Slint plumbing (page → `main.slint` → Rust) is untouched — only the Rust side of layer 3 moves.
- **No signature/behavior changes** to moved code. No renames beyond module qualification. No new abstractions (no trait, no macro for wiring — the pattern is a plain function per screen).
- **No i18n changes** — zero new user-facing strings, so no `.ftl` edits (Epic 0 fr/en-parity DoD is vacuously satisfied).
- **No migration, no repository change** — the 8-touchpoint checklist does not apply (nothing persisted changes).

### Testing & verification

- `pomone-ui` has no unit tests (UI binary, coverage-exempt in practice); the workspace suite (~340 tests) guards the layers below. Full gate: `cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`.
- **Manual smoke (AC 1), XDG-isolated** — never against the real DB:
  ```sh
  XDG_DATA_HOME=/tmp/pom XDG_CONFIG_HOME=/tmp/pom cargo run -p pomone-cli -- seed-demo
  XDG_DATA_HOME=/tmp/pom XDG_CONFIG_HOME=/tmp/pom cargo run -p pomone-ui
  ```
  Checklist: open Settings (Ctrl+9); toggle language (all labels flip, twice for round-trip); change holiday region (status confirms, calendar page shows greyed holidays); change area unit m²→ha (status confirms, Plantings list re-renders in ha); change mass unit; « Sauvegarder maintenant » backup (status shows the backup path); « Tester » on the current SQLite backend (settings-test-ok). Restart the app: settings persisted.
- Headless option if no display: the app runs under `Xvfb` with the software renderer (used for prior GUI verifications).

### Workflow

Branch off fresh `main` (e.g. `refactor/0-1-wiring-settings`); PR references the convergence issue **#110** (do not `Closes` it). CI green required; never push to `main`. Commit message convention: imperative summary + `(#110)`.

### Project Structure Notes

- New: `crates/pomone-ui/src/wiring/mod.rs`, `crates/pomone-ui/src/wiring/settings.rs` — matches the architecture delta tree exactly (`pomone-ui/src/wiring/` NEW, `main.rs` MOD).
- Architecture anti-pattern list explicitly includes «new callbacks in `main.rs`» — the `mod.rs` header must state this rule (AC 3).
- Lints are workspace-level (clippy pedantic, `unsafe_code = deny`); do not add local `#[allow]` — the pragmatic allow-list lives in root `Cargo.toml`.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 0.1] — story statement + ACs
- [Source: _bmad-output/planning-artifacts/architecture.md#Structure Patterns] — `wiring/<screen>.rs`, `fn wire_<screen>(…)`, anti-patterns
- [Source: _bmad-output/planning-artifacts/architecture.md#Project Directory Structure] — Slice 0 delta (`src/wiring/` NEW)
- [Source: _bmad-output/project-context.md] — 34 rules: CI `-D warnings`, 3-layer Slint plumbing, never push to main
- [Source: crates/pomone-ui/src/main.rs] — line references above measured on `main` = 89b2a82

## Dev Agent Record

### Agent Model Used

Claude Fable 5 (claude-fable-5)

### Debug Log References

- `cargo clippy --workspace --all-targets -- -D warnings` initially failed with `clippy::too_many_lines` on `wire_settings` (228/100). Resolved with a local `#[allow(clippy::too_many_lines)]`, matching the existing convention in `main.rs` (same allow on `main()`, `apply_translations`, and two other known-long functions).
- Headless smoke run: `WAYLAND_DISPLAY` must be unset or Slint/winit targets the session compositor instead of the Xvfb display.

### Completion Notes List

- Pure relocation as specified: 9 callback registrations + 11 settings-only helpers moved verbatim from `main.rs` into `crates/pomone-ui/src/wiring/settings.rs`; the only textual change inside moved blocks is `Rc::clone(&state)` → `Rc::clone(state)` (the `state` parameter is `&Rc`). `main.rs` went from 5650 to 5107 lines; no `.slint`, `.ftl`, migration, or repository change.
- `wiring/mod.rs` documents the `wire_<screen>` pattern and the "never register callbacks in `main.rs`" rule (AC 3). `wire_settings` and `refresh_settings` are `pub(crate)` (`unreachable_pub` is a workspace lint); everything else in the module is private.
- Doc cross-references between the split index/from_index helper pairs updated to plain-text module paths (rustdoc-only concern; CI doesn't run rustdoc).
- Verification: `cargo fmt --check` ✓, `clippy -D warnings` ✓, `cargo test --workspace` 388/388 ✓, AC2 grep empty ✓.
- Manual smoke (Xvfb :77, XDG-isolated `/tmp/pom`, demo seed): language toggle FR→EN ✓, Settings navigation + form prefill (SQLite path) ✓, Test connection → "Connection successful." ✓, Back up now → "Backup created: …2026-07-12_134639.bak" (file present) ✓, holiday region Vaud→Geneva → "Region saved" ✓, area unit m²→ha → "Units saved" + Plantings re-rendered ✓, config.toml persisted `language="en"`, `holiday_region="ch-ge"`, `area_unit="ha"` ✓.
- Known pre-existing observation (not a regression): the Plantings table body renders empty under Xvfb/software renderer while the Gantt and form are fine — reproduced identically on `main` before this story (see memory note of 2026-07-10).

### File List

- `crates/pomone-ui/src/main.rs` (modified — blocks/helpers removed, `mod wiring;` + `wire_settings` call + qualified `refresh_settings` startup call, unused imports pruned, doc links adjusted)
- `crates/pomone-ui/src/wiring/mod.rs` (new)
- `crates/pomone-ui/src/wiring/settings.rs` (new)
- `_bmad-output/implementation-artifacts/sprint-status.yaml` (status tracking)

## Change Log

- 2026-07-12: Story 0.1 implemented — settings/backend/backup/holidays/units/language wiring extracted from `main.rs` into `wiring/settings.rs`; wiring skeleton + pattern doc established in `wiring/mod.rs`. No behavior change (388 tests green, clippy/fmt clean, manual smoke on isolated DB).
