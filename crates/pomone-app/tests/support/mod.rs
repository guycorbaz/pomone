//! Shared proptest support for the Epic-1 fact-sequence invariants (story 1.7).
//!
//! A small **state-machine model** of the task settle lifecycle, a proptest
//! strategy that generates valid gesture walks over a seeded task set, and
//! helpers that apply those walks through the *real* single write path
//! (`record_fact`). The model is the oracle: after any prefix of a walk, the
//! database projection must equal the model's state.
//!
//! This is deliberately reusable — later epics plug their interleaving
//! properties (autogen ∘ reconciliation ∘ edition) into the same generator
//! rather than re-inventing one. Each integration-test binary that wants it
//! declares `mod support;`.

// Shared across several test binaries; not every binary exercises every item.
#![allow(dead_code)]

use chrono::{Duration, NaiveDate};
use proptest::prelude::*;

use pomone_app::facts::{record_fact, Fact};
use pomone_db::{Repository, SqliteRepository, TaskRepo, TaskTypeRepo};
use pomone_domain::ids::TaskId;
use pomone_domain::{SkipReason, Task};

/// The button the user could press on a task row (mirrors the agenda ⋯ menu).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GestureKind {
    MarkDone,
    Skip,
    Correct,
}

/// One generated step: which seeded task (by index) + the button pressed.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Step {
    pub(crate) task: usize,
    pub(crate) kind: GestureKind,
}

/// Modeled settle state of a task — the ground truth the DB projection must
/// mirror at every point in a sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TaskState {
    Pending,
    Done,
    Skipped,
}

/// The fixed date every seeded task is planned on (and every gesture is
/// *about*). Clock-stable so runs are reproducible.
#[must_use]
pub(crate) fn base_date() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 3, 2).unwrap()
}

/// proptest strategy: a bounded walk of steps over `num_tasks` seeded tasks.
/// Each step names a task and a button; the [`Model`] interprets it against the
/// task's current state (invalid button for the state = no-op, exactly as the
/// UI wouldn't offer it).
pub(crate) fn gesture_walk(num_tasks: usize, max_len: usize) -> impl Strategy<Value = Vec<Step>> {
    let kind = prop_oneof![
        Just(GestureKind::MarkDone),
        Just(GestureKind::Skip),
        Just(GestureKind::Correct),
    ];
    proptest::collection::vec(
        (0..num_tasks, kind).prop_map(|(task, kind)| Step { task, kind }),
        0..=max_len,
    )
}

/// The state-machine oracle: per-task state advanced by interpreting each step
/// exactly as the UI would.
///
/// Transitions (and *only* these): `Pending --MarkDone--> Done`,
/// `Pending --Skip--> Skipped`, `Done|Skipped --Correct--> Pending`. Every other
/// `(state, button)` pair is a no-op — the button wouldn't exist on that row.
/// Because a settled task can only leave `Done`/`Skipped` via an explicit
/// `Correct`, a projection that mirrors this model proves **I1** (done is
/// absorbing except correction) and **I2** (skipped is never observed as done).
pub(crate) struct Model {
    pub(crate) states: Vec<TaskState>,
}

impl Model {
    #[must_use]
    pub(crate) fn new(num_tasks: usize) -> Self {
        Self {
            states: vec![TaskState::Pending; num_tasks],
        }
    }

    /// Interpret one step against the current state, mutating the model and
    /// returning the real [`Fact`] to record — or `None` for a no-op step.
    pub(crate) fn interpret(
        &mut self,
        step: Step,
        task_ids: &[TaskId],
        on: NaiveDate,
    ) -> Option<Fact> {
        let state = &mut self.states[step.task];
        let id = task_ids[step.task];
        match (*state, step.kind) {
            (TaskState::Pending, GestureKind::MarkDone) => {
                *state = TaskState::Done;
                Some(Fact::Done { task_id: id, on })
            }
            (TaskState::Pending, GestureKind::Skip) => {
                *state = TaskState::Skipped;
                Some(Fact::Skipped {
                    task_id: id,
                    on,
                    reason: SkipReason::Weather,
                    note: None,
                })
            }
            (TaskState::Done | TaskState::Skipped, GestureKind::Correct) => {
                *state = TaskState::Pending;
                Some(Fact::Reopened { task_id: id, on })
            }
            // Any other (state, button) pair is not offered by the UI → no-op.
            _ => None,
        }
    }
}

/// Seed `num_tasks` free-standing pending tasks on [`base_date`]; return their
/// ids in creation order (parallel to the model's task indices).
pub(crate) async fn seed_tasks(repo: &SqliteRepository, num_tasks: usize) -> Vec<TaskId> {
    let tt = repo.task_type_list().await.unwrap()[0].id;
    let mut ids = Vec::with_capacity(num_tasks);
    for _ in 0..num_tasks {
        let task = Task::new(
            None,
            None,
            tt,
            None,
            None,
            base_date(),
            None,
            None,
            None,
            None,
        );
        repo.task_create(&task).await.unwrap();
        ids.push(task.id);
    }
    ids
}

/// Apply a prefix of `steps` through the single write path (`record_fact`),
/// advancing `model` in lockstep. Returns the count of real (non-no-op) facts
/// recorded. `recorded_at` strictly increases with the step index, so it is
/// always ≥ the occurred date and never ties (deterministic ordering).
pub(crate) async fn apply_prefix(
    repo: &dyn Repository,
    model: &mut Model,
    task_ids: &[TaskId],
    steps: &[Step],
) -> usize {
    let on = base_date();
    let base_dt = on.and_hms_opt(0, 0, 0).unwrap();
    let mut recorded = 0usize;
    for (i, step) in steps.iter().enumerate() {
        if let Some(fact) = model.interpret(*step, task_ids, on) {
            let recorded_at = base_dt + Duration::seconds(i64::try_from(i).unwrap() + 1);
            record_fact(repo, fact, recorded_at).await.unwrap();
            recorded += 1;
        }
    }
    recorded
}

/// Read the DB projection for each task and classify it as a [`TaskState`].
pub(crate) async fn observe(repo: &dyn Repository, task_ids: &[TaskId]) -> Vec<TaskState> {
    let mut out = Vec::with_capacity(task_ids.len());
    for id in task_ids {
        let task = repo.task_get(*id).await.unwrap().unwrap();
        out.push(
            match (task.completed_on.is_some(), task.skipped_on.is_some()) {
                (true, _) => TaskState::Done,
                (false, true) => TaskState::Skipped,
                (false, false) => TaskState::Pending,
            },
        );
    }
    out
}
