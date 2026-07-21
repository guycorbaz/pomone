# Story 3.4: Retro-entry and perennial lifecycle

Status: ready-for-dev

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As the grower,
I want a decades-old perennial (the 1996 apple row) to enter `active` with **zero past tasks** and an explicit reassurance line telling me so, and a terminated perennial to **stop occupying its ground at the termination date**,
so that retro-entering thirty years of orchard never floods my agenda, and a dead bush stops haunting the capacity curve.

## Context, dependencies & scope boundary

Two independent halves of the same journey (PRD J3, FR14 + FR15), closing Epic 3.

**Half A — retro-entry.** `create_perennial_planting` → `generate_tasks_for_planting` currently generates tasks at `establishment ± offset` with **no notion of today**: a perennial established in 1996 produces past-dated ITK/profile tasks (the avalanche J3 explicitly fails on). The fix is a **caller-injected `today`** (project rule AR12: no `now()` below the UI/CLI layer) and a past-cutoff applied to **perennial schedules only**, plus a localized notice the planting form shows after creation.

**Half B — termination frees occupancy.** `PlantingStatus::{Completed,Failed,Abandoned}` exists (issue #63) but carries **no date**, and `capacity::occupancy_window` ignores status entirely — so a dead bush occupies to the horizon (FR15 broken, and it silently inflates every capacity curve from story 3.2). The fix is a persisted `terminated_on` date (migration **0014**, additive, both backends) that becomes the occupancy interval's exclusive end.

**Dependencies (all on `main`):** the pure engine + `occupancy_window` (3.1), `capacity_view::build_placed` (3.2), ITK + profile generation (3.3), request structs (0.5), the paper-loop harness (0.7).

**In scope:**
1. Injected `today` into task generation; **perennial** past-dated tasks suppressed (zero past tasks on retro-entry).
2. A localized **reassurance line** after creating a past-established perennial, listing the next generated tasks (or stating there are none).
3. Migration **0014** `planting.terminated_on` + domain field/method + codec-free date column + **both** backend row mappers + `cross_backend_tests` + `copy_all` coverage.
4. `occupancy_window` honours `terminated_on` (end = the earlier of the scheduled end and the termination date); `capacity_view` passes it; engine proptest extended.
5. `set_planting_status` becomes date-carrying and **reversible** (FR24/FR26): a terminal status requires a date, going back to `Active` clears it. Wired in the planting-detail screen (3 Slint layers) with fr+en keys.
6. `paper_loop::step_e3_placement` implemented: the perennial + capacity datasets (place → curve → retro-entered 1996 perennial → terminate → curve drops).

**Out of scope (later / deferred):**
- **A `planting.terminated` fact-journal write.** `FactKind::PlantingTerminated` exists in `codec.rs` but `FactsRepo::record_fact` only accepts a `TaskProjection`; adding a `PlantingProjection` (db trait + 2 backends + transaction) belongs with Epic 5's reversible corrections — **decided and already recorded in `deferred-work.md` (Guy, 2026-07-21)**. Do **not** append a bare `field_event` next to a separate `planting_update`: two non-atomic writes is exactly the split-brain the 1.2 single-write-path rule forbids.
- **Recurring perennial tasks** (a yearly winter pruning derived from `PruningSeason`). ITK offsets are anchored once on establishment; there is no per-year regeneration in R1. The notice must therefore state honestly what exists — see AC 3. User-created recurring series (`extend_series_if_needed`) remain the only recurrence.
- Any curve/peak UI change (3.2 owns the screen), the printed occupancy map (7.2), the PeakPanel (7.1).
- Retro-entry of past **annual** cycles: a placed annual whose sow date has passed keeps its past task (the grower needs it to mark done/skipped). **Do not generalize the cutoff to cycles** — it would silently regress 3.3.

## Acceptance Criteria

1. **No clock below the UI.** `generate_tasks_for_planting(repo, planting, today: NaiveDate)` takes the reference date; `PerennialPlantingRequest`, `AnnualPlantingRequest` and `PlacementRequest` carry a `today` field supplied by the UI/CLI (`Local::now().date_naive()`). No `Local::now()` / `Utc::now()` appears in `pomone-app` or below.
2. **Zero past tasks on retro-entry.** Creating a `Perennial` planting whose `established_on` is decades past (1996) generates **no task dated strictly before `today`** — via the ITK path *and* the profile path. The planting's status is `Active`. A perennial established *today or in the future* still generates its tasks unchanged.
3. **The reassurance line.** After creating a past-established perennial, the plantings screen shows one localized line stating the guarantee and the next tasks — e.g. FR *«Établi en 1996 — aucune tâche passée ne sera créée ; prochaines tâches : Taille (2027-02-10), …»*. When generation produced no upcoming task, the line says so explicitly (a distinct key) — it never shows an empty list or invents a task. fr **and** en keys, alphabetical in their section.
4. **Termination carries a date and frees the ground.** `planting.terminated_on` is persisted (migration 0014, additive, in **both** migration trees, no new CHECK), round-trips identically on SQLite and MariaDB (`cross_backend_tests` + `copy_all`), and is `NULL` for every pre-existing row. Setting a terminal status (`Completed`/`Failed`/`Abandoned`) requires a date; setting `Active` clears it (reversible, FR24). The domain rejects a `terminated_on` earlier than the planting's occupancy start.
5. **The engine honours it.** `occupancy_window` returns the exclusive end `min(scheduled_end, terminated_on)` (open-ended perennial + termination → `Some(terminated_on)`); a terminated placement is **not** active at any `t >= terminated_on`. `capacity_view::build_placed` passes it, so the story-3.2 curve drops the day a perennial is terminated. Proptest: adding a termination never *increases* occupancy at any `t` (monotone decrease), and termination at/after the scheduled end is a no-op.
6. **Wired end-to-end.** The planting-detail life-cycle card gains a termination-date input (defaulting to today, ISO, validated like the other date fields) next to the existing status combo; the property/callback exists in `planting_detail.slint` → `main.slint` → `main.rs`/`wiring/planting_detail.rs` (all three layers). An invalid or missing date on a terminal status surfaces a localized form error, never a panic.
7. **The harness gains the datasets.** `paper_loop::step_e3_placement` is no longer a no-op: it places the E2-planned successions, asserts the occupancy curve is non-empty and within capacity, retro-enters a 1996 perennial (asserting zero past tasks), terminates it, and asserts occupancy at a post-termination date drops back. It survives both `FailureMode`s and stays a required CI check.
8. **Green bar + coverage.** `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all -- --check` all pass; workspace coverage ≥ 80%; `capacity.rs` stays ≥ 95% (NFR20). MariaDB legs verified with `cargo test -- --ignored` (Docker) or explicitly reported as unrun.

## Tasks / Subtasks

- [x] **Task 1 — Inject `today` into generation (AC: 1, 2)**
  - [x] `task_autogen::generate_tasks_for_planting(repo, planting, today: NaiveDate)`. In **both** `generate_from_itk` and `generate_from_profile`, skip a candidate whose date `< today` **iff** `matches!(planting.schedule, PlantingSchedule::Perennial { .. })`. Put the rule in one tiny predicate (`fn suppresses_past(planting) -> bool` + the date test) so both paths cannot drift; document *why* cycles are exempt.
  - [x] Add `today: NaiveDate` to `PerennialPlantingRequest`, `AnnualPlantingRequest` and `PlacementRequest` (required arg of each `new`/`from_*` constructor — these structs exist precisely to absorb this, story 0.5). Thread it to the generator in `create_annual_planting`, `create_perennial_planting`, `place_planned_planting`.
  - [x] Migrate every call site: `wiring/plantings.rs`, `wiring/placement.rs`, `demo.rs`, CLI, and tests. UI/CLI supply `Local::now().date_naive()`; tests supply a fixed date.
  - [x] Tests: 1996 perennial + ITK → 0 tasks; 1996 perennial, ITK-less → 0 tasks; perennial established today → tasks unchanged; **annual placed with a past sow date → task still created** (anti-regression for 3.3).

- [x] **Task 2 — The reassurance line (AC: 3)**
  - [x] `plantings_view::retro_entry_notice(repo, i18n, planting: &Planting, today) -> AppResult<Option<String>>`: `None` unless the planting is `Perennial` with `established_on < today`; otherwise a localized string built from the planting's tasks with `planned_on >= today` (task-type label + ISO date, first 3, ordered by date).
  - [x] Fluent keys in **both** `fr/main.ftl` and `en/main.ftl`, alphabetical in their section: `planting-retro-entry-notice = … { $year } … { $tasks }` and `planting-retro-entry-notice-none = … { $year } …`. **Never `perl` on a `.ftl`** (UTF-8 mojibake, AI-E2-4) — use Edit.
  - [x] `wiring/plantings.rs`: on successful perennial creation, show the notice in `status_text` (non-error) instead of the plain `status-planting-created`.
  - [x] Tests: notice present with the tasks listed; the `-none` variant when nothing upcoming; `None` for an annual and for a perennial established today.

- [x] **Task 3 — Persist `terminated_on` (AC: 4) — walk the 8-touchpoint checklist**
  - [x] `migrations/sqlite/0014_planting_terminated_on.sql` **and** `migrations/mariadb/0014_planting_terminated_on.sql`: `ALTER TABLE planting ADD COLUMN terminated_on <TEXT|DATE> NULL`. Additive only, **no new CHECK** (the existing table CHECK doesn't mention it — leave it alone). Mirror the header-comment style of `0012_geometry.sql`.
  - [x] Domain: `Planting.terminated_on: Option<NaiveDate>` (defaults `None` in `Planting::new`, whose signature stays unchanged) + `Planting::terminate(on) -> DomainResult<()>` / `Planting::reopen()`. Invariant: `on >= schedule.start_date()` else a structured `DomainError` (reuse `DateBefore`/`DateAfter` — check `error.rs` for the existing variant before inventing one).
  - [x] **Both** backend row mappers (`sqlite/`, `mariadb/`) read/write the column; `planting_create`/`planting_update` carry it. It is a plain nullable date — **no `codec.rs` change** (codec owns sum types), but say so in the PR so a reviewer doesn't hunt for it.
  - [x] `cross_backend_tests.rs`: a terminated planting round-trips identically on both backends and survives `copy_all` (backend swap).
  - [x] Check `test_helpers.rs` / any `Planting { .. }` struct literal in tests and `migration.rs` — a new field breaks literals at compile time; fix, don't `..Default::default()` around it.

- [x] **Task 4 — The engine honours termination (AC: 5)**
  - [x] `capacity::occupancy_window(schedule, terminated_on: Option<NaiveDate>)` — one function, two args (no second source of truth). End = the earlier of the scheduled exclusive end and `terminated_on`; open-ended + terminated → `Some(terminated_on)`. `terminated_on` is the **exclusive** end (the ground is free that day), matching how `expected_removal_on` already behaves — document it on the function.
  - [x] Update the caller `capacity_view::build_placed` (`p.terminated_on`) and the re-export in `pomone-domain/src/lib.rs` if the signature is public.
  - [x] Unit tests: terminated before the scheduled end shortens it; terminated after it is a no-op; terminated perennial with no removal date ends at the termination date; `terminated_on == start` → the interval is empty and never active (the engine already treats inverted/empty as inactive — assert it).
  - [x] Proptest in `capacity.rs` (`≥95%` module, AI-E2-5 style): for arbitrary placements and an arbitrary termination date, occupancy at every sampled `t` is `<=` the untermined occupancy (monotone decrease), and is `0` for that placement at `t >= terminated_on`.

- [x] **Task 5 — Date-carrying, reversible status (AC: 4, 6)**
  - [x] `services::set_planting_status(repo, planting_id, status, terminated_on: Option<NaiveDate>)`: a terminal status with `None` → `AppError` (localized, closed variant — no stringly-typed user text); `Active` forces `terminated_on = None`. Route through `Planting::terminate`/`reopen` so the domain invariant runs (never assign the field by hand — project rule).
  - [x] UI, all three Slint layers: `planting_detail.slint` gains `in-out property <string> terminated-on-text` (+ its label key) inside the existing life-cycle card; `main.slint` re-declares and forwards it; `wiring/planting_detail.rs` reads it, validates with the shared `validate_iso_date` helper, defaults to `today_iso()`, and renders errors through `render_form_error` / `localize_app_error`.
  - [x] fr+en keys for the field label and the "termination date required" error (`error-*` convention).
  - [x] Tests: terminate persists status + date; reopening clears the date; terminal-without-date is rejected; a date before establishment is rejected.

- [ ] **Task 6 — Harness step E3 (AC: 7)**
  - [ ] Implement `step_e3_placement(app)` in `crates/pomone-app/tests/paper_loop.rs` — make it `async` like `step_e1_record_facts` / `step_e2_plan_lines` and `.await` it in `seed_baseline` (it is currently called synchronously there): place the successions seeded by `step_e2_plan_lines` via `place_planned_planting`, assert `occupancy_curve` has non-zero occupancy in the expected weeks; add a 1996 perennial and assert `task_list_for_planting` is empty; terminate it and assert occupancy at a later date drops. Use the harness's `fixed_today()` clock and the existing `normalize::snapshot` helpers — no wall-clock, no ordering flakiness.
  - [ ] Add the survival assertions to `assert_reopens_clean` (the placed plantings and the terminated perennial still there after the crash/reopen), mirroring the E2 `planned.len() == 6` / needs-list assertions already present.
  - [ ] Keep it green under both `FailureMode`s.

- [ ] **Task 7 — Green bar + hygiene (AC: 8)**
  - [ ] `cargo test --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo fmt --all -- --check`; `cargo llvm-cov --workspace` (≥80% overall, `capacity.rs` ≥95%).
  - [ ] `cargo test -- --ignored` for the MariaDB legs if Docker is available; otherwise state plainly in the completion notes that they were not run.
  - [ ] (No deferral note to write — the `planting.terminated` fact-projection deferral to Epic 5 is already recorded in `deferred-work.md`, decision by Guy 2026-07-21. Do **not** re-open it in this story.)
  - [ ] Smoke-run against an isolated DB: `XDG_DATA_HOME=/tmp/pom XDG_CONFIG_HOME=/tmp/pom cargo run -p pomone-ui` — create a 1996 perennial, read the notice, terminate it, watch the placement curve.

## Dev Notes

### Read these three files fully before editing

1. **`crates/pomone-app/src/task_autogen.rs`** — the 3.3 dispatcher (`itk_activities` → `generate_from_itk` | `generate_from_profile`), its idempotency set `(task_type_id, planned_on)` and its settled-guard. The cutoff is a *third* filter alongside those two; it must not disturb either. The existing 12 tests must stay green.
2. **`crates/pomone-domain/src/capacity.rs`** — `Placement`, `active_at`, `occupancy_window`, and the saturating posture. The engine is **pure**: it never learns about `PlantingStatus`; it only receives a shorter interval. Keep it that way.
3. **`crates/pomone-app/src/capacity_view.rs` (`build_placed`, ~line 230)** — it turns **every** `planting_list()` row into a `Placement` with **no status filter**. That is the FR15 bug: today a `Failed` perennial still occupies to the horizon. The fix is the shortened interval, not a status filter in the view (a terminated planting must still show its *past* occupancy — history is not erased).

### Termination semantics — the decision to nail

`terminated_on` is the **exclusive** end of the occupancy interval: the ground is free *on* that day. This mirrors `expected_removal_on`, which `occupancy_window` already returns unchanged as the exclusive end, and it is the same half-open `[start, end)` convention that makes adjacent successions not double-count (3.1). Do not add a `+1 day` — a `Cycle`'s end already carries its own `add_days(last_harvest_on, 1)` because *harvest day is occupied*; a termination date is a removal date, not a work date.

`min(scheduled_end, terminated_on)` — not "terminated_on wins": a perennial terminated *after* its expected removal must not resurrect occupancy. Guard the `Option` combination explicitly; `Option::min` on `None` does **not** do what you want (`None.min(Some(x)) == None`), so write the match by hand and test all four combinations.

### The past-cutoff — why perennials only

FR14 is about *pre-existing perennials entered retroactively*. A placed **annual** whose sow date is a fortnight past is normal January-planning behaviour and the grower needs that task on the sheet to mark it done or skipped. Suppressing it would silently regress story 3.3 (whose J-negative bed-prep tasks are *deliberately* pre-establishment) and quietly delete work from the weekly print. So: cutoff on `PlantingSchedule::Perennial` only, in one shared predicate, with the rationale in a code comment — a reviewer *will* ask.

Note the interaction with the 3.3 J-negative rule: for a perennial, an activity at `established - 14` is by definition past when establishment is past — correctly suppressed. For a perennial established next spring, a J-negative prep task lands before establishment but still after today — correctly kept.

### `today` flows down from the UI (AR12)

Story 1.3 froze the rule: **no `now()` below the UI/CLI layer** — the API takes the timestamp. `pomone-ui` already does this everywhere (`wiring/agenda.rs:213`, `wiring/home.rs:41`, `forms.rs::today_iso`). Add `today` to the three request structs rather than a bare extra parameter: that is exactly what story 0.5 introduced them for, and it keeps the creation functions at ≤3 parameters (0.5's acceptance criterion — don't break it by adding a positional arg).

### The reassurance line is a *promise*, so keep it honest

The UX spec's example — *«Établi en 1996 — aucune tâche passée ne sera créée ; prochaines tâches : taille hiver 2027»* — implies a recurring pruning that R1 does not generate (ITK offsets are anchored once on establishment; `PruningSeason` drives no autogen). Build the line from the tasks that were **actually generated** (`planned_on >= today`), and use the `-none` variant when there are none. Do not fabricate a plausible next task. [Source: ux-design-specification.md#Flow-3b; crates/pomone-domain/src/crop.rs `PruningSeason`]

### The 8-touchpoint checklist for `terminated_on`

```
domain      → Planting field + terminate()/reopen() invariant
codec       → (none — plain nullable date, not a sum type)
migration   → 0014_planting_terminated_on.sql in sqlite AND mariadb (additive, no CHECK)
db backends → row mappers + create/update in BOTH SqliteRepository and MariaDbRepository
app/view    → planting_detail DTO field (String) + parser; capacity_view passes it
ui (Slint)  → planting_detail.slint + main.slint + wiring/planting_detail.rs  (all three)
i18n        → label + error keys in fr/main.ftl AND en/main.ftl
tests       → cross_backend_tests round-trip + copy_all
```

A missed backend row mapper is a **silent divergence**, not a compile error — that is what `cross_backend_tests` is for. [Source: project-context.md#Adding-a-persisted-field]

### Read-path defensive posture (AI-E2-3, review-checklist item)

`terminated_on` arrives from a persisted, free-TEXT SQLite column and can be absurd (year 9999, or before establishment on a hand-edited row). The domain invariant guards the *write* path; the **read** path (`occupancy_window`, `build_placed`) must tolerate an out-of-order row without panicking — an inverted interval is already "never active" in the engine, which is the correct degradation. Assert it with a test rather than assuming it.

### Review process (Guy's decision)

**Focused 2-reviewer review for 3.4** (retro-entry / lifecycle); 3-layer adversarial was reserved for 3.1 + 3.3. [Source: epic-2-retro-2026-07-15.md §Decisions, AI-E2-1]

### Traps carried from prior stories

- **Never `perl` on UTF-8 `.ftl`** — mojibake. Use Edit. (AI-E2-4)
- `Option::is_none_or` is Rust **1.82**; MSRV here is **1.80** — use `map_or`.
- `RUSTFLAGS: -D warnings` in CI: an unused import or a stale doc-link fails the build.
- Keep flipping `sprint-status.yaml` in-branch before merge (AI-E2-2).
- No `sqlite3` CLI on this machine — inspect DBs with `python3`'s `sqlite3` module.
- Never push to `main`: branch + `gh pr create`, and reference issue #110.

### Project Structure Notes

- Layer discipline holds: the field and its invariant are `pomone-domain`; the interval maths stay in `capacity.rs` (pure, no `Repository`); `capacity_view`/`plantings_view` are the string-DTO boundary; only `pomone-ui` reads the clock.
- Files touched stay well under the 2000-line target (`services.rs` is the largest at 1289; the additions are small).
- Migration numbering: **0014** is next in both trees (0013 = `planned_planting_placed`).

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Epic-3 / Story-3.4] — BDD: zero past tasks + the confirmation line; termination ends occupancy (engine proptest extended); harness gains the perennial + capacity datasets.
- [Source: _bmad-output/planning-artifacts/prd.md FR14, FR15, FR24, FR26; Journey 3 §Opening/Resolution; "The journey fails if…"] — the two failure modes this story removes.
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Flow-3b] — one screen, one reassurance line; state the fact, show what happens next.
- [Source: _bmad-output/planning-artifacts/architecture.md P3 (capacity engine, perennial-to-horizon), D1 fact kinds (`planting.terminated`), AR12 (no clock below UI)].
- [Source: crates/pomone-app/src/task_autogen.rs] — the dispatcher + guards to extend.
- [Source: crates/pomone-domain/src/capacity.rs `occupancy_window`, `active_at`] — the interval contract.
- [Source: crates/pomone-app/src/capacity_view.rs `build_placed`] — the unfiltered placement builder (the FR15 bug site).
- [Source: crates/pomone-app/src/services.rs `set_planting_status`, `create_perennial_planting`, `place_planned_planting`, `PlacementRequest`] — the services to extend.
- [Source: crates/pomone-app/tests/paper_loop.rs `step_e3_placement`] — the no-op to implement.
- [Source: _bmad-output/implementation-artifacts/3-3-tasks-generate-at-placement.md] — ITK path, establishment anchor, skip-aware guard this story must not disturb.
- [Source: _bmad-output/implementation-artifacts/epic-2-retro-2026-07-15.md] — AI-E2-1 review mode, AI-E2-3 read-path posture, AI-E2-4 `.ftl` trap.
- [Source: _bmad-output/project-context.md] — layering, 8-touchpoint checklist, migration rules, MSRV, lints.

## Dev Agent Record

### Agent Model Used

### Debug Log References

### Completion Notes List

### File List

## Change Log

- 2026-07-21 — Story 3.4 drafted (ready-for-dev).
