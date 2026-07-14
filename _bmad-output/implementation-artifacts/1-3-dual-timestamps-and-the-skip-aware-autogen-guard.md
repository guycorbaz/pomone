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

### Review Findings

3-layer adversarial review (retro AI-2). Strong convergence on the settled-guard key. 3 patch, 0 defer, 2 dismissed.

- [x] [Review][Patch] Skip-aware guard now keys on **`TaskCategory`**, not `task_type_id` — closes the Repiquage↔Plantation method-flip hole (establishment resolves to two types but one category, so a replan dropping the sow date could resurrect a settled establishment as a "Plantation"). Documented the single-window assumption + the future recurring-per-planting caveat (Blind/Auditor's campaign-window point). [crates/pomone-app/src/task_autogen.rs]
- [x] [Review][Patch] Added `settled_establishment_survives_method_flip_on_replan` — marks the Repiquage **done**, replans as bought-plants, asserts no Plantation is regenerated (covers the method-flip AND the done branch of the guard, which the first test didn't). [crates/pomone-app/src/task_autogen.rs]
- [x] [Review][Patch] Marked the 1.1 + 1.2 deferrals **resolved** in `deferred-work.md`: pre-1.3 inverted rows load unvalidated (decode builds the struct literally, never re-runs `new()`); the 1.2 same-day-tie deferral is gone now that `recorded_at` is a real injected timestamp. [_bmad-output/implementation-artifacts/deferred-work.md]
- [x] [Review][Dismissed] Two Lows dismissed: (a) `create_task` at 8 args — it already carries `#[allow(clippy::too_many_arguments)]`; clippy clean. (b) non-atomic `task_create`+`record_fact` / future-dated completion — unreachable: both record `Fact::Done { on: today }` (not `planned_on`) and the UI sources `today` + `recorded_at` from the same `Local::now()`, so `occurred_at == recorded_at.date()` always; the invariant can't fire there. Pre-existing two-step-write seam, noted in the 1.2 review.

## Completion Notes

- 3-layer adversarial review: **3 patch, 0 defer, 2 dismissed, 0 blocking**. The substantive fix was re-keying the skip-aware guard on `TaskCategory` to survive the establishment method-flip on a future replan. Both prior deferrals (1.1, 1.2) are now genuinely resolved.
- Post-fix: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` → **417 passed, 0 failed**; coverage **81.9% lines**.
