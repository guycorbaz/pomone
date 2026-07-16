# Story 3.3: Tasks generate at placement

Status: review

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As the grower,
I want the ITK activities of a placed planting's crop — including J-negative preparation — to become dated tasks the moment the planting is placed, with ITK-less crops still falling back to the profile auto-generation,
so that placing a plan turns it into dated work I can carry to the field.

## Context, dependencies & scope boundary

Placement (story 3.2) already turns a planned succession into a real `Planting` and calls the existing `generate_tasks_for_planting` (profile auto-gen: Sow / Transplant / Plant / Harvest from the schedule). **This story makes that generation ITK-aware:** when the planting's crop has an ITK template, its ordered activities become the tasks (each `task_type_id` at `establishment + offset_days`); when it doesn't, the existing profile auto-gen is the fallback — unchanged.

**No migration, no UI, no new screen.** The whole change lives in `task_autogen.rs` (+ a pure date helper). Because `place_planned_planting` → `create_annual_planting`/`create_perennial_planting` → `generate_tasks_for_planting` already, making that one function ITK-aware means **placement automatically emits ITK tasks** — and so does any other planting-creation path, consistently.

**Dependencies (all on `main`):** the ITK model + repo (`ItkTemplate`/`ItkActivity`, `itk_template_get_for_crop`, `itk_activity_list_for_template` — story 2.2), the skip-aware auto-gen guard (story 1.3), and the placement flow (story 3.2).

**In scope:**
1. ITK-driven task generation in `generate_tasks_for_planting`: for a crop with an ITK, each activity → a task at `anchor + offset_days` (signed; J-negative lands **before** establishment), carrying the activity's `task_type_id`, `method_id`, `implement_id`, and label/notes.
2. **Fallback** to the current profile auto-gen when the crop has no ITK template (or an empty one).
3. The **skip-aware guard holds across re-generation** on the same planting: no duplicates (idempotency) and no resurrection of a settled (done/skipped) slot (story 1.3), extended to ITK tasks.
4. A pure **signed date-offset helper** in `date_calc.rs` (overflow-guarded, property-tested) — J-negative and far-future offsets must not `.unwrap()` chrono.

**Out of scope (later stories):**
- **Perennial retro-entry reassurance + "zero past tasks" for decades-old plantings → story 3.4.** This story generates ITK tasks including past-dated prep for a *normal* establishment date; the decades-past guard is 3.4.
- **Perennial termination frees occupancy → story 3.4.**
- **Interleaving state-machine proptests (autogen ∘ reconciliation ∘ edition) → Epic 5** (`tests/fact_invariants.rs`). This story's proptests are the pure date-offset ones.
- Editing an ITK and regenerating existing plantings' tasks in bulk — not here; generation runs at creation/placement.

## Acceptance Criteria

1. **ITK activities become dated tasks.** Given a placed planting whose crop has an ITK template with N activities, generation creates one task per activity at `establishment_date + activity.offset_days`, using the activity's `task_type_id` (not a category-resolved type), and carrying its `method_id` / `implement_id` (and label/notes → task notes). The **establishment anchor** = the planting's bed-establishment date: transplant date if present, else sowing date, else first-harvest (cycle); `established_on` (perennial) — the same anchor `occupancy_window`/`phase_dates` use.
2. **Pre-establishment (J-negative) tasks land before establishment.** An activity with `offset_days < 0` yields a task dated strictly before the establishment date (e.g. bed prep at J-14). Signed offsets are computed with an **overflow-guarded** pure helper — never a chrono `.unwrap()`.
3. **ITK-less crops fall back to profile auto-gen.** A crop with no ITK template (or a template with zero activities) generates exactly the tasks the current `generate_tasks_for_planting` produces today (Sow / Transplant / Plant / Harvest) — behaviour unchanged, existing tests still green.
4. **Skip-aware guard holds across re-generation.** Re-running generation on the same planting (idempotency) creates **no duplicate** (a `(task_type, date)` already present is skipped), and a slot that is already **settled** (done or skipped, story 1.3) is **not resurrected** even if a re-generation would place it at a shifted date. This holds for ITK tasks as well as profile tasks. (Define the guard key for ITK explicitly — see Dev Notes.)
5. **Best-effort, non-aborting.** Generation stays permissive (the pattern of the current function): a missing `TaskType`, a malformed offset, or an absent method/implement is logged and skipped, never rolls back the already-created planting. A `Task::new` over an overflowing offset date is skipped with a warning, not a panic.
6. **Placement emits ITK tasks end-to-end.** Placing a planting (story 3.2) whose crop has an ITK produces the ITK tasks (verified through `place_planned_planting`), and the tasks show on the existing task views/calendar.
7. **Green bar + coverage.** `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all -- --check` pass; workspace coverage ≥ 80%; the auto-gen module keeps its high coverage (NFR20 — autogen is on the ≥95% list).

## Tasks / Subtasks

- [x] **Task 1 — Pure signed date-offset helper in `date_calc.rs` (AC: 2, 5)**
  - [x] Add `pub fn offset_days(date: NaiveDate, offset: i32) -> DomainResult<NaiveDate>` (or similarly named) that shifts by a **signed** day count, returning `DomainError::DateOverflow` on under/overflow — mirroring `add_days`'s guard. **Never `.unwrap()` chrono arithmetic** (year boundaries / extreme offsets are real inputs).
  - [x] Unit + proptest: positive/negative offsets, 0, leap-year boundaries, and extreme offsets that saturate to an error rather than panic. Reuse the `date_calc` proptest style (story 3.1 precedent).

- [x] **Task 2 — ITK-aware generation in `task_autogen.rs` (AC: 1, 2, 3, 5)**
  - [x] In `generate_tasks_for_planting`: fetch the crop's ITK — resolve `planting.variety_id → variety.crop_id`, then `repo.itk_template_get_for_crop(crop_id)`; if `Some` and it has activities (`itk_activity_list_for_template`), take the **ITK path**, else the existing **profile path** (fallback, untouched).
  - [x] ITK path: compute the **establishment anchor** (a small pure helper reusing the cycle transplant→sow→first_harvest / perennial `established_on` rule). For each activity, `date = offset_days(anchor, activity.offset_days)`; on `Err`, log + skip that activity (AC 5). Build the task with the activity's `task_type_id`, `method_id`, `implement_id`, and `notes` (activity label/notes → task notes).
  - [x] Keep the function's **best-effort** contract: a missing type/date never aborts; the planting is already saved.

- [x] **Task 3 — Skip-aware guard for ITK tasks (AC: 4)**
  - [x] Extend the idempotency guard (`(task_type_id, date)` already present → skip) to the ITK path.
  - [x] Extend the settled guard so a settled slot isn't resurrected on re-generation. **Decide + document the ITK guard key** (see Dev Notes): the existing profile guard keys on **category** (because establishment resolves to two types); ITK activities carry an explicit `task_type_id`, so key the ITK settled-guard on **`task_type_id`** (documenting the "at most one activity per type per planting" assumption, mirroring the existing "one per category" assumption). Prove it: a settled (skipped) ITK task is not regenerated at a shifted date after a re-generation.
  - [x] Tests: (a) two generations → no duplicate; (b) skip a generated ITK task, regenerate → not resurrected; (c) DONE branch too.

- [x] **Task 4 — End-to-end + fallback tests (AC: 1, 2, 3, 6)**
  - [x] Test ITK generation dates: an activity at offset +20 lands 20 days after establishment; an activity at −14 lands 14 days before; method/implement carried onto the task.
  - [x] Test the fallback: a crop with no ITK still gets the profile tasks (assert existing behaviour).
  - [x] Test through `place_planned_planting` (story 3.2): place a planting whose crop has an ITK → the ITK tasks exist for the created planting (AC 6).

- [x] **Task 5 — Green bar (AC: 7)**
  - [x] `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all -- --check`; confirm the auto-gen module coverage stays high (`cargo llvm-cov`).

## Dev Notes

### The one file that changes: `task_autogen.rs`

Read it fully before editing — it's the file whose behaviour this story extends, and it must keep working for ITK-less crops. Current shape (`crates/pomone-app/src/task_autogen.rs`):
- `generate_tasks_for_planting(repo, planting)` lists task types, reads the planting's existing tasks, builds an **idempotency set** `(task_type_id, planned_on)` and a **settled-category set** (`is_settled()` tasks → their category), then for each `(Trigger, date)` from `phase_dates(planting)` resolves a type and creates a task unless it's a duplicate or its category is settled.
- `phase_dates(planting)` (pure) yields `(Sow, sown)`, `(Transplant|Plant, transplanted)`, `(Harvest, first_harvest)` for cycles; `(Plant, established_on)` for perennials.
- `resolve_type` maps a `Trigger` to a `TaskType` by category (with the Plantation-vs-Transplant nuance).

**This story adds an ITK branch in front of the profile branch and leaves the profile branch as the fallback.** Do not delete or rewrite the profile path — AC3 requires it byte-for-byte for ITK-less crops. [Source: crates/pomone-app/src/task_autogen.rs]

### The establishment anchor (reuse, don't reinvent)

The ITK offsets are anchored on **establishment on the bed**. Use the same rule the rest of the code uses:
- **Cycle:** `transplanted_on` if present, else `sown_on`, else `first_harvest_on`.
- **Perennial:** `established_on`.

This is exactly `capacity::occupancy_window`'s start (story 3.1) and the intent of `phase_dates`. Extract a tiny pure helper `establishment_anchor(planting) -> NaiveDate` (or inline) — one source of truth. [Source: crates/pomone-domain/src/capacity.rs `occupancy_window`; task_autogen.rs `phase_dates`]

### ITK model (story 2.2) — what an activity carries

`ItkActivity { id, template_id, task_type_id, offset_days: i32 (signed), method_id: Option, implement_id: Option, label: Option, position: u32, notes: Option }`. Ordered by `position`. `ItkTemplate` is per **crop** (`itk_template_get_for_crop(crop_id)`). `Task::new(planting_id, location_id, task_type_id, task_method_id, implement_id, planned_on, …, notes)` already takes method/implement — thread the activity's straight through. [Source: crates/pomone-domain/src/itk.rs; crates/pomone-domain/src/task.rs `Task::new`; crates/pomone-db/src/repository.rs `ItkRepo`]

### Signed offsets — date math stays in `date_calc.rs`

`add_days` only accepts `u16` (positive). ITK `offset_days` is signed `i32` (J-negative prep). **Add a pure `date_calc` helper** for signed shift, overflow-guarded to `DomainError::DateOverflow` — do **not** compute dates in `task_autogen` with raw chrono `.unwrap()` (project rule: date logic lives in `date_calc`, never `.unwrap()` chrono; year boundaries/leap days/extreme offsets are real inputs). [Source: _bmad-output/project-context.md#Domain-dates; crates/pomone-domain/src/date_calc.rs]

### The skip-aware guard for ITK — the decision to nail (3-layer review will scrutinize this)

Story 1.3's guard: **don't resurrect settled work across a replan.** The profile path keys the settled guard on **category** because one agronomic slot (establishment) can resolve to two types (Repiquage/Plantation). ITK is different: each activity names an **explicit `task_type_id`**, so:
- **Idempotency:** key on `(task_type_id, date)` — same as today. Prevents duplicates when generation re-runs at the same anchor.
- **Settled (no-resurrection):** key on **`task_type_id`** for ITK tasks (not category). This means a settled ITK task of a given type is not regenerated at a shifted date. **Documented assumption:** at most one activity per `task_type_id` per planting (mirrors the existing "at most one task per category per planting" assumption in the current guard's comment). If a future ITK has two activities of the same type, they'd share a settled-slot — note it; the exact fix (a `task.itk_activity_id` link) is a **future migration, out of scope here**.
- **Re-placement note:** in 3.2, "un-place" *deletes* the planting (tasks cascade) and "re-place" creates a **new** planting id — so a re-placed planting legitimately gets fresh tasks (the prior placement, including its skip decisions, was explicitly undone). The 1.3 guard is about **re-generation/replan on the same planting id**, which is what AC4 tests. Do not conflate the two; state this in a code comment so a reviewer doesn't read it as a resurrection bug. [Source: task_autogen.rs guard comment (lines 48–64); story 1.3; story 3.2 unplace semantics]

### Best-effort contract (keep it)

The generator never rolls back the planting over a taxonomy/date quibble — missing type, missing method, or an offset that overflows the date range is **logged + skipped**, not fatal. Preserve this for the ITK path (AC5). [Source: task_autogen.rs module doc]

### Read-path / defensive posture

`offset_days` is a persisted `i32` (unbounded within i32). The signed helper must **return an error, not panic**, on an offset that pushes the date out of chrono's range — then generation skips that activity. Prove it with a test (extreme offset). [Source: project-context.md#Read-path-defensive-posture]

### Review process (Guy's decision)

**3-layer adversarial review is default-ON for 3.3** (task-generation story, alongside 3.1) — the skip-aware guard for ITK is exactly the kind of interleaving invariant that hides bugs from happy-path tests. Focused review was for 3.2 (screen). [Source: epic-2-retro §Decisions, AI-E2-1]

### Traps carried from Epic 2 / prior stories

- **Never `perl` on UTF-8 `.ftl`** (not expected here — no new strings). Use Edit.
- The settled guard's key choice is the subtle part — get it wrong and you either resurrect skipped work (key too loose) or suppress legitimate distinct tasks (key too tight). Test both directions.
- Auto-gen is best-effort by design; don't turn a skip into an abort.

### Project Structure Notes

- Layer discipline: `date_calc` helper is pure `pomone-domain`; `task_autogen` is `pomone-app` reading `&dyn Repository`. No new module needed; no migration; no UI.
- File-size: `task_autogen.rs` is well under the cap; keep it there.
- MSRV 1.80, `unsafe_code = deny`, clippy `pedantic`, `-D warnings`. Note `Option::is_none_or` is **1.82** — use `map_or` (1.80).

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Epic-3 / Story-3.3] — BDD: each activity → task (anchor+offset), pre-establishment before establishment, ITK-less fallback, skip-aware guard across re-placement.
- [Source: _bmad-output/planning-artifacts/architecture.md AR7 (D5 ITK model), AR11 (autogen guard idempotence keyed), lines 109/113/238] — ITK activities {task_type_id, offset_days signed, optional method/implement}; guard upgraded, done AND skipped count as existing.
- [Source: _bmad-output/planning-artifacts/prd.md FR1–FR8] — ITK/plan → dated work (the "plan → dated tasks" aha moment).
- [Source: crates/pomone-app/src/task_autogen.rs] — the function to extend + its skip-aware guard.
- [Source: crates/pomone-domain/src/itk.rs; crates/pomone-db/src/repository.rs#ItkRepo] — ITK model + repo methods.
- [Source: crates/pomone-app/src/services.rs] — `create_annual_planting`/`create_perennial_planting`/`place_planned_planting` call `generate_tasks_for_planting`.
- [Source: _bmad-output/implementation-artifacts/1-3-*.md] — the skip-aware guard this story extends.
- [Source: _bmad-output/project-context.md] — date-in-Rust rule, best-effort autogen, read-path posture, MSRV.

## Dev Agent Record

### Agent Model Used

claude-opus-4-8[1m] (Opus 4.8, 1M context) — dev-story workflow.

### Debug Log References

- `task_autogen.rs` coverage **98.8%** (632/640) — above the ≥95% autogen gate (NFR20). New `date_calc::offset_days` covered by 4 unit tests + a proptest.
- Full suite green: `cargo test --workspace` (15 test binaries ok, 0 failed); `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo fmt --check` clean.
- End-to-end fallback: `seed-demo` on a fresh DB generated **85 tasks** via the profile path (the demo has 0 ITK templates) — the ITK-less fallback (AC3) works unchanged; no regression.

### Completion Notes List

- **`date_calc::offset_days(date, i32)`** — pure, signed, overflow-guarded (`DateOverflow`, never a chrono `.unwrap()`); handles J-negative prep and far-future offsets. Unit + proptest (never panics; `offset` then `-offset` is invertible in range).
- **`generate_tasks_for_planting` is now a dispatcher**: `itk_activities()` resolves `planting → variety → crop → template`; if the crop has a non-empty ITK, the **ITK path** runs, else the **profile path** (the previous body, extracted verbatim into `generate_from_profile`, unchanged). AC3 — all 8 pre-existing autogen tests still pass.
- **ITK path** (`generate_from_itk`): anchor = `establishment_anchor()` (transplant→sow→first-harvest / `established_on` — same rule as `occupancy_window`); each activity → a task at `offset_days(anchor, offset)` carrying its `task_type_id`, `method_id`, `implement_id`, and label/notes → task notes. An out-of-range offset is logged and skipped (best-effort, AC5).
- **Skip-aware guard for ITK** keyed on **`task_type_id`** (idempotency on `(type, date)`; settled = no-resurrect on type). Documented assumption: at most one activity per type per planting (mirrors the profile path's per-category assumption); a future same-type ITK would need a `task.itk_activity_id` link. A code comment states that un-place→re-place (a *new* planting) legitimately regenerates — it is not a resurrection, which is about same-planting re-generation.
- **Tests (4 new + 8 preserved):** ITK activities land at anchor±offset with J-negative before establishment + notes; method/implement carried onto the task; settled (skipped) ITK type not resurrected on replan; placement (`place_planned_planting`) emits the ITK tasks end-to-end.
- **No migration, no UI, no new strings.** The whole change is `task_autogen.rs` + the `date_calc` helper.

### File List

**Modified**
- `crates/pomone-domain/src/date_calc.rs` — `offset_days` signed helper + unit tests + proptest.
- `crates/pomone-app/src/task_autogen.rs` — ITK-aware dispatcher (`itk_activities`, `establishment_anchor`, `itk_notes`, `generate_from_itk`, `generate_from_profile`) + 4 ITK tests.

## Change Log

- 2026-07-16 — Story 3.3 implemented: ITK-driven task generation at placement (`generate_from_itk`) with the profile path as an unchanged fallback, a signed `date_calc::offset_days` helper, and the skip-aware guard extended to ITK (keyed by `task_type_id`). All green (test/clippy/fmt); `task_autogen` at 98.8% coverage; fallback verified via seed-demo. No migration/UI.
