# Story 1.3: Dual timestamps and the skip-aware autogen guard

Status: in-review

## Story

As the grower,
I want occurred_at (backdatable) distinct from caller-injected recorded_at, and autogen never resurrecting a done/skipped task,
So that backdated entry is safe and regeneration never undoes decisions.

## Acceptance Criteria

1. **Given** a skipped task for (planting, task_type, campaign window) **when** autogen re-runs after planting edit/replan **then** no new task inserts for that slot (done AND skipped count as existing).
2. **And** occurred_at ≤ recorded_at enforced in the domain constructor.
3. **And** no `now()` below the UI/CLI layer (API takes the timestamp).

## Tasks / Subtasks

- [x] Task 1: The temporal invariant (AC: 2)
  - [x] `FieldEvent::new` rejects `occurred_at > recorded_at.date()` with `DomainError::DateAfter { field: "occurred_at", … }`. Backdated and same-day are accepted.
- [x] Task 2: Inject `recorded_at` — no clock below UI/CLI (AC: 3)
  - [x] `toggle_task_completion`, `create_task`, `update_task` now take `recorded_at: NaiveDateTime` from the caller (dropped the midnight-fabrication). The UI (`task_form.rs`) reads `Local::now().naive_local()` and injects it. `facts::record_fact` already took it. This also removes the 1.2 same-day-tie deferral (real timestamps ⇒ no midnight ties).
- [x] Task 3: Skip-aware autogen guard (AC: 1)
  - [x] `Task::is_settled()` (done **or** skipped). `generate_tasks_for_planting` now also collects `settled_types` and skips a trigger whose task type is already settled for the planting — so a replan that shifts dates can't resurrect a done/skipped slot. The exact `(type, date)` idempotency guard stays for pending tasks.
- [x] Task 4: Verify (AC: 1–3)
  - [x] New tests: domain `occurred_at_after_recorded_at_is_rejected`; app `settled_task_is_not_resurrected_after_replan` (skip the sow, replan two weeks later, assert the sow type isn't regenerated).
  - [x] `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` → 416 passed, 0 failed; coverage 81.8% lines; `seed-demo` smoke green.

## Dev Notes

- **`occurred_at` vs `recorded_at`.** `occurred_at` (`NaiveDate`) is the agronomic date the fact is *about* — backdatable. `recorded_at` (`NaiveDateTime`) is when it was written — injected by the UI/CLI. The invariant compares `occurred_at ≤ recorded_at.date()`. The full backdating *UI* (a date picker for `occurred_at`) is story 1.5; 1.3 lays the domain + plumbing so callers pass both.
- **Only the UI/CLI reads the clock.** After this story the sole clock reads below the UI are `backup.rs` (a backup *filename* stamp — not a fact) — no fact clock exists below the UI layer.
- **Skip-aware guard keys on `(planting, task_type)`.** Each phase (sow/transplant/harvest) resolves to a distinct task type, so the task type identifies the slot; a settled task of that type blocks regeneration regardless of the recomputed date. Non-settled (pending) tasks keep the exact `(type, date)` idempotency, unchanged.

## Completion Notes

_(review pending — 3-layer adversarial review per retro AI-2.)_
