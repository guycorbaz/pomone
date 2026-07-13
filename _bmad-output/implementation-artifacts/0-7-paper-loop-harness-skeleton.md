# Story 0.7: Paper-loop harness skeleton

Status: done

## Story

As the test architect,
I want `pomone-app/tests/paper_loop.rs` in CI from day one — XDG-isolated DB, kill/replay runner with `FailureMode = Kill | NetworkDrop`, injected clock, golden normalization helpers, explicit `// TODO(E_n)` no-op steps,
So that every epic extends one harness.

## Acceptance Criteria

1. **Given** an empty isolated database **when** the harness seeds, kills mid-write, restarts, re-opens — on both failure modes **then** the database reopens cleanly and the assertions pass.
2. **And** the harness is a required CI check.
3. **And** normalization policy (fixed clock, stable ordering, locale-stable formatting) ships as shared helpers.

## Tasks / Subtasks

- [x] Task 1: The isolated file-backed loop (AC: 1)
  - [x] `isolated_dir(mode)` — a per-process, per-mode temp dir; explicit `BackendConfig::Sqlite { path }` so the real XDG dirs are never touched.
  - [x] `open_app` reopens the **same** path each time → restart == reopen; drives through `App::new` (migrate + seed), i.e. the real launch path.
- [x] Task 2: Kill/replay runner over both failure modes (AC: 1)
  - [x] `FailureMode { Kill, NetworkDrop }`; `run_paper_loop(mode)` = seed → abandon the app (the "crash") → restart → assert. `NetworkDrop` adds a transient reconnect before the final restart.
  - [x] The `#[tokio::test]` iterates both modes.
- [x] Task 3: Injected clock + normalization helpers (AC: 3)
  - [x] `fixed_today()` — the harness never reads the wall clock; the baseline marker is derived from it, so golden output is machine-stable.
  - [x] `mod normalize` — `sorted` (stable ordering) + `snapshot` (canonical multi-line golden). Exercised in the reopen assertion, incl. an order-independence check.
- [x] Task 4: Per-epic no-op hooks (AC: 1)
  - [x] `step_e1_record_facts` … `step_e5_reconcile` — explicit `// TODO(E_n)` no-ops, all called in `seed_baseline` so the plug points exist and stay live.
- [x] Task 5: One real gesture + reopen assertion (AC: 1)
  - [x] `seed_baseline` drives the `families_view::create_family` view-model; `assert_reopens_clean` reopens and asserts the family survived (proving a clean reopen) on both modes.
- [x] Task 6: Verify (AC: 1–3)
  - [x] `cargo test -p pomone-app --test paper_loop` green; CI-blocking via `cargo test --workspace` (no workflow-YAML change needed).
  - [x] `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` → 391 passed, 0 failed.

## Dev Notes

### Skeleton scope — honest about what's simulated

- **Isolation** is by explicit path, not env mutation: the harness builds its own `AppConfig` pointing at a temp file and never calls `save_default`, so it can't read or write the real `~/.local/share/pomone`. This is strictly isolated and avoids the `set_var` data-race that XDG-env juggling would risk in a threaded test binary.
- **"Kills mid-write"** is modelled at the harness level by abandoning the `App` (dropping the pool) without a graceful close, then reopening the same file and asserting durability + a clean reopen. True `SIGKILL`-mid-transaction fault injection (subprocess + signal) is deferred to the epic where the write path it protects lands — **E1's append-only `field_event`** — where durability actually has teeth. The scaffold is here now so that work only fills in a hook.
- **`NetworkDrop`** on the SQLite backend degenerates to an extra clean reopen (there is no socket); the variant exists so the MariaDB backend and later epics can inject a real dropped connection. Kept behaviourally distinct from `Kill` (transient reconnect) to avoid a same-arms no-op and to keep the 2-mode matrix real from day one.

### Lints

Hooks are **synchronous** while empty (an empty `async fn` trips `clippy::unused_async` under `-D warnings`); an epic that needs I/O flips its own hook to `async`. The `normalize` helpers are `pub(crate)` (a bare `pub` in a test binary trips `unreachable_pub`).

### Files

- `crates/pomone-app/tests/paper_loop.rs` (new, CI-blocking).

## Completion Notes

- No production code touched — the harness is a self-contained integration test driving existing services/view-models (`App::new`, `families_view`). `cargo test --workspace` runs it, so it blocks CI from merge.
- Every later epic extends *this* file: fill a `step_e*` hook, grow `assert_reopens_clean`, and (for E1) swap the drop-based crash for real fault injection.
