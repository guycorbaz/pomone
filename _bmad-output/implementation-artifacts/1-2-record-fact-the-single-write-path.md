# Story 1.2: record_fact — the single write path

Status: in-review

## Story

As the grower,
I want every gesture (done, skipped, terminate, correction) recorded through one `facts::record_fact` (event insert + state projection, one transaction),
So that marked means persisted and re-applying is harmless.

## Acceptance Criteria

1. **Given** a pending task **when** a `task.done` fact is recorded **then** event + projection commit atomically; same-id re-record returns the existing result.
2. **And** `task.skipped` projects the skip columns (FR18 semantics); corrections re-project without touching the original event.
3. **And** a lint test asserts no `UPDATE task SET (completed_on|skipped_on…)` statement exists outside `facts.rs`.

## Scope decisions (confirmed with the owner)

- **1.2 redirects the "done" path**: the calendar toggle and the task-form completion checkbox now go through `facts::record_fact` (done + reopen). Skip/correct *UI* stays for story 1.5.
- **Task facts only** (done / skipped / correction-reopen). `planting.terminated` projection lands later (its `FactKind` slot already exists from 1.1).

## Tasks / Subtasks

- [x] Task 1: Transactional write path in the DB layer (AC: 1, 2)
  - [x] `FactsRepo::record_fact(event, projection)` — one `sqlx::Transaction`: idempotency check (existing id → `AlreadyRecorded`, no re-projection) → event insert → task projection → commit. In `sqlite/facts.rs` + `mariadb/facts.rs`.
  - [x] `TaskProjection` (`Done`/`Skipped`/`Reopen`) + `FactOutcome` in `repository.rs`, aggregated into `Repository`, re-exported.
- [x] Task 2: `task_update` stops writing settled columns (AC: 3)
  - [x] Removed `completed_on` (and skip columns) from `task_update`'s SET on both backends — projected exclusively by `facts.rs` now.
- [x] Task 3: The single app-layer write path (AC: 1, 2)
  - [x] `pomone-app/src/facts.rs` — `Fact { Done, Skipped, Reopened }` + `record_fact(repo, fact, recorded_at)`. `recorded_at` caller-injected (no clock below the UI). A reopen finds the latest settling event and points `corrects` at it.
  - [x] Added `FactKind::TaskReopened` (`task.reopened`) + codec; a correction is an event with `corrects` set. `SkipReason::as_str`/`from_literal` + `skip_payload` moved to the domain (single source of the literals; codec delegates).
- [x] Task 4: Redirect the done path through facts (AC: 1)
  - [x] `task_calendar_view::toggle_task_completion`, `tasks_view::create_task`/`update_task`, and a `services.rs` test now record facts instead of a direct task UPDATE.
- [x] Task 5: The lint (AC: 3)
  - [x] `crates/pomone-db/tests/facts_write_path.rs` — scans the workspace for `UPDATE task SET … <settled column> =` outside a `facts.rs` file. Verified positive **and** negative (injecting a stray UPDATE fails it).
- [x] Task 6: Cross-backend + verify (AC: 1, 2)
  - [x] `cross_backend_tests::scenario_record_fact` (both entry points): atomic done, idempotent replay (`AlreadyRecorded`, no 2nd event), skip clears completion.
  - [x] `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` → 413 passed, 0 failed; coverage 81.6% lines. `seed-demo` smoke green.

## Dev Notes

- **Idempotency is at the event level.** `record_fact` no-ops on a *replayed event id* (crash recovery / the paper-loop). A fresh UI gesture mints a new id, so normal double-clicks record two facts — that's correct. AC1's "same-id re-record returns the existing result" is exercised at the repo level.
- **Reopen (`TaskReopened`)** is a new `FactKind` (no schema CHECK ⇒ free to add). It clears the settled state and points `corrects` at the most recent settling event; the original event is never mutated (append-only).
- **No new migration** — 1.2 rides on 1.1's schema; it is a code-only write-path refactor.
- `recorded_at` is currently derived from the caller-supplied date at midnight (no clock read below the UI). Story 1.3 refines this to a properly injected timestamp and adds the `occurred_at ≤ recorded_at` invariant.

## Completion Notes

_(review pending — 3-layer adversarial review per retro AI-2.)_
