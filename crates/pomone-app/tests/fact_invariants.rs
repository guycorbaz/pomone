//! Interleaving state-machine property tests for the Epic-1 fact invariants
//! (story 1.7), plus the kill-injection prefix-replay property.
//!
//! The single write path (`record_fact`) is the only way task state is settled,
//! and [`support::Model`] is the oracle for what a valid gesture walk must
//! produce. The properties checked here:
//!
//! - **I1 — done stays done (absorbing except explicit correction):** verified
//!   at two layers. The *persistence* layer is deliberately last-write-wins and
//!   mutually exclusive (`record_fact…mutually_exclusive…`), so "absorbing" is
//!   a **UI-gating** invariant — the agenda ⋯ menu only offers `Correct` on a
//!   settled task (stories 1.5/1.6), modeled by [`support::Model`]; the walk
//!   proptest checks the projection mirrors that gated model.
//! - **I2 — skipped never resurrects nor counts as done:** the "never counts as
//!   done" half is checked structurally (a skipped task is never observed as
//!   done, never both columns — proven reachable by the adversarial test). The
//!   "never resurrects" half (autogen never regenerates a settled phase after a
//!   replan) is covered by the `task_autogen` unit tests (`settled_task_is_not_
//!   resurrected_after_replan`, story 1.3); the series case is `i3` below.
//! - **I3 — series survive occurrence-skip:** skipping one materialized
//!   occurrence never deletes the series or its sibling occurrences.
//! - **Prefix-replay yields prefix state:** applying a prefix, dropping the pool
//!   (a clean crash between transactions), and reopening the same file yields
//!   exactly the prefix's projection with the journal holding exactly the facts
//!   recorded — none lost, none duplicated. (True torn-write `SIGKILL`
//!   injection is deferred; see the test's scope note.)

mod support;

use proptest::prelude::*;
use support::{apply_prefix, base_date, gesture_walk, observe, seed_tasks, Model};

use pomone_app::facts::{record_fact, Fact};
use pomone_db::{
    seed_defaults, FieldEventRepo, SqliteRepository, TaskRepo, TaskSeriesRepo, TaskTypeRepo,
};
use pomone_domain::SkipReason;

/// A fresh Tokio runtime per proptest case (proptest bodies are synchronous).
fn rt() -> tokio::runtime::Runtime {
    tokio::runtime::Runtime::new().expect("tokio runtime")
}

const NUM_TASKS: usize = 3;
const MAX_LEN: usize = 16;

proptest! {
    // Kept modest so the suite stays fast under `cargo test --workspace` while
    // still exploring thousands of interleavings across the whole run.
    #![proptest_config(ProptestConfig { cases: 48, ..ProptestConfig::default() })]

    /// I1 + I2 (structural): after any valid gesture walk, the database
    /// projection equals the state-machine model. Since the model only leaves a
    /// settled state via an explicit `Correct`, a matching projection proves
    /// done is absorbing except correction, and a skipped task is never
    /// observed as done (nor both at once).
    #[test]
    fn projection_mirrors_state_machine(steps in gesture_walk(NUM_TASKS, MAX_LEN)) {
        rt().block_on(async {
            let repo = SqliteRepository::in_memory().await.unwrap();
            seed_defaults(&repo).await.unwrap();
            let ids = seed_tasks(&repo, NUM_TASKS).await;

            let mut model = Model::new(NUM_TASKS);
            apply_prefix(&repo, &mut model, &ids, &steps).await;

            let observed = observe(&repo, &ids).await;
            prop_assert_eq!(&observed, &model.states);

            // I2 sharpened: a task is never simultaneously done and skipped.
            for id in &ids {
                let task = repo.task_get(*id).await.unwrap().unwrap();
                prop_assert!(
                    !(task.completed_on.is_some() && task.skipped_on.is_some()),
                    "a task must never be both done and skipped"
                );
            }
            Ok(())
        })?;
    }

    /// Prefix-replay yields prefix state: apply a *prefix* of length `cut` on a
    /// file-backed DB, drop the pool (a clean crash *between* transactions —
    /// `record_fact` commits each fact with `synchronous=FULL`, so a dropped
    /// pool loses nothing already committed), reopen the same file, and assert
    /// the projection equals the model's state after exactly that prefix, and
    /// the append-only journal holds exactly the facts recorded (none lost, none
    /// duplicated). `cut` varies the prefix length so different-sized histories
    /// are replayed.
    ///
    /// Scope note: this exercises **durability + prefix consistency**, the
    /// AC's "prefix-replay yields prefix state". True *torn-write* injection
    /// (a `SIGKILL` mid-transaction) is deferred — same posture as the
    /// paper-loop harness (story 0.7); per-transaction atomicity of the
    /// event+projection write is asserted separately in `sqlite/facts.rs` /
    /// `cross_backend_tests`.
    #[test]
    fn prefix_replay_yields_prefix_state(
        steps in gesture_walk(NUM_TASKS, MAX_LEN),
        cut in 0usize..=MAX_LEN,
    ) {
        rt().block_on(async {
            let dir = tempfile::tempdir().unwrap();
            let url = format!("sqlite:{}?mode=rwc", dir.path().join("pomone.sqlite").display());
            let k = cut.min(steps.len());
            let prefix = &steps[..k];

            // Apply the prefix on the file DB, then "crash" (drop the pool).
            let mut model = Model::new(NUM_TASKS);
            let (ids, recorded) = {
                let repo = SqliteRepository::connect(&url).await.unwrap();
                seed_defaults(&repo).await.unwrap();
                let ids = seed_tasks(&repo, NUM_TASKS).await;
                let recorded = apply_prefix(&repo, &mut model, &ids, prefix).await;
                drop(repo); // the crash — no graceful close
                (ids, recorded)
            };

            // Reopen the same file: state must equal the model's prefix state.
            let repo = SqliteRepository::connect(&url).await.unwrap();
            let observed = observe(&repo, &ids).await;
            prop_assert_eq!(&observed, &model.states, "reopen != prefix replay");

            // No fact lost or duplicated: the journal has exactly what we wrote.
            let events = repo.field_event_list_all().await.unwrap();
            prop_assert_eq!(events.len(), recorded, "journal count != facts recorded");
            Ok(())
        })?;
    }
}

/// I1, explicit and legible: once a task is done it stays done through further
/// (non-correcting) gestures, and only an explicit correction reopens it.
#[tokio::test]
async fn i1_done_is_absorbing_except_explicit_correction() {
    let repo = SqliteRepository::in_memory().await.unwrap();
    seed_defaults(&repo).await.unwrap();
    let ids = seed_tasks(&repo, 1).await;
    let id = ids[0];
    let on = base_date();
    let at = |secs: i64| on.and_hms_opt(0, 0, 0).unwrap() + chrono::Duration::seconds(secs);

    // Mark done, then record it done again (idempotent gesture) — still done.
    record_fact(&repo, Fact::Done { task_id: id, on }, at(1))
        .await
        .unwrap();
    record_fact(&repo, Fact::Done { task_id: id, on }, at(2))
        .await
        .unwrap();
    let task = repo.task_get(id).await.unwrap().unwrap();
    assert!(task.completed_on.is_some() && task.skipped_on.is_none());

    // An explicit correction (and only that) reopens it.
    record_fact(&repo, Fact::Reopened { task_id: id, on }, at(3))
        .await
        .unwrap();
    let task = repo.task_get(id).await.unwrap().unwrap();
    assert!(task.completed_on.is_none() && task.skipped_on.is_none());
}

/// Adversarial persistence check — the half the model-gated walk can't reach.
///
/// The generated walks only ever submit *state-valid* gestures (the agenda ⋯
/// menu never offers Skip on a done task), so they never challenge `record_fact`
/// with a conflicting settle. This test does, going straight to the write path:
/// it pins the persistence-layer contract as **mutually exclusive + last-write
/// -wins** — every projection NULLs its sibling column, so a task is *never*
/// both done and skipped, whatever the sequence. This is what makes I1's
/// "absorbing" a genuine **UI-gating** invariant (enforced by the button set,
/// stories 1.5/1.6) rather than a persistence one — recorded here explicitly so
/// the layering isn't mistaken.
#[tokio::test]
async fn record_fact_projections_are_mutually_exclusive_and_last_write_wins() {
    let repo = SqliteRepository::in_memory().await.unwrap();
    seed_defaults(&repo).await.unwrap();
    let id = seed_tasks(&repo, 1).await[0];
    let on = base_date();
    let mut secs = 0i64;
    let mut at = || {
        secs += 1;
        on.and_hms_opt(0, 0, 0).unwrap() + chrono::Duration::seconds(secs)
    };

    let done = || Fact::Done { task_id: id, on };
    let skip = || Fact::Skipped {
        task_id: id,
        on,
        reason: SkipReason::Weather,
        note: None,
    };

    // A deliberately "illegal" walk the UI would never offer: done → skip →
    // done → reopen. After each fact the projection must reflect exactly that
    // last fact, and never both settled columns at once.
    record_fact(&repo, done(), at()).await.unwrap();
    let t = repo.task_get(id).await.unwrap().unwrap();
    assert!(t.completed_on.is_some() && t.skipped_on.is_none());

    // Skip an already-done task (illegal for the UI) — last write wins, and the
    // done column is cleared, so it is Skipped, never both.
    record_fact(&repo, skip(), at()).await.unwrap();
    let t = repo.task_get(id).await.unwrap().unwrap();
    assert!(
        t.skipped_on.is_some() && t.completed_on.is_none(),
        "skip must clear completed_on — never both settled columns"
    );

    // Done again — back to done-only (skip column cleared).
    record_fact(&repo, done(), at()).await.unwrap();
    let t = repo.task_get(id).await.unwrap().unwrap();
    assert!(t.completed_on.is_some() && t.skipped_on.is_none());

    // Reopen clears everything.
    record_fact(&repo, Fact::Reopened { task_id: id, on }, at())
        .await
        .unwrap();
    let t = repo.task_get(id).await.unwrap().unwrap();
    assert!(t.completed_on.is_none() && t.skipped_on.is_none());
}

/// I3: skipping one materialized occurrence of a recurring series never deletes
/// the series or any sibling occurrence — only the victim is skipped.
#[tokio::test]
async fn i3_series_survives_occurrence_skip() {
    use pomone_app::tasks_view::create_recurring_task;

    let repo = SqliteRepository::in_memory().await.unwrap();
    seed_defaults(&repo).await.unwrap();
    let tt = repo.task_type_list().await.unwrap()[0].id.to_string();

    // A weekly free-standing series over ~5 weeks → several occurrences.
    let series_id_str = create_recurring_task(
        &repo,
        "",
        &tt,
        "2026-03-02",
        "",
        1,
        "weeks",
        Some("2026-04-06"),
        base_date(),
    )
    .await
    .unwrap();
    let series_id =
        pomone_domain::ids::TaskSeriesId::from(uuid::Uuid::parse_str(&series_id_str).unwrap());

    let occurrences: Vec<_> = repo
        .task_list()
        .await
        .unwrap()
        .into_iter()
        .filter(|t| t.series_id == Some(series_id))
        .collect();
    assert!(
        occurrences.len() >= 3,
        "expected several occurrences, got {}",
        occurrences.len()
    );

    // Skip the middle occurrence.
    let victim = occurrences[occurrences.len() / 2].id;
    record_fact(
        &repo,
        Fact::Skipped {
            task_id: victim,
            on: base_date(),
            reason: SkipReason::Weather,
            note: None,
        },
        base_date().and_hms_opt(9, 0, 0).unwrap(),
    )
    .await
    .unwrap();

    // The series survives, no occurrence is deleted, only the victim is skipped.
    assert!(
        repo.task_series_get(series_id).await.unwrap().is_some(),
        "series must survive an occurrence skip (I3)"
    );
    let after: Vec<_> = repo
        .task_list()
        .await
        .unwrap()
        .into_iter()
        .filter(|t| t.series_id == Some(series_id))
        .collect();
    assert_eq!(
        after.len(),
        occurrences.len(),
        "no sibling occurrence deleted"
    );
    for task in &after {
        if task.id == victim {
            assert!(task.skipped_on.is_some(), "the victim is skipped");
        } else {
            assert!(
                task.completed_on.is_none() && task.skipped_on.is_none(),
                "siblings stay pending"
            );
        }
    }
}
