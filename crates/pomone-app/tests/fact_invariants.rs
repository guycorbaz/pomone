//! Interleaving state-machine property tests for the Epic-1 fact invariants
//! (story 1.7), plus the kill-injection prefix-replay property.
//!
//! The single write path (`record_fact`) is the only way task state is settled,
//! and [`support::Model`] is the oracle for what a valid gesture walk must
//! produce. The properties checked here:
//!
//! - **I1 — done stays done (absorbing except explicit correction):** a task in
//!   `Done` can only leave it via a `Correct` gesture; the model encodes that
//!   and the projection mirrors the model.
//! - **I2 — skipped never resurrects nor counts as done:** a `Skipped` task is
//!   never observed as done (never both columns set; `completed_on` stays
//!   `None`); and, separately, task auto-generation never regenerates a settled
//!   phase (`series_survives`… covers the series case; the autogen guard is
//!   unit-tested in `task_autogen`).
//! - **I3 — series survive occurrence-skip:** skipping one materialized
//!   occurrence never deletes the series or its sibling occurrences.
//! - **Crash = exact prefix:** applying a prefix, dropping the pool mid-life,
//!   and reopening the same file yields exactly the prefix's projection — no
//!   fact lost or duplicated (NFR6).

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

    /// Crash = exact prefix (NFR6): apply a prefix, "crash" by dropping the pool
    /// mid-life, reopen the same file — the projection equals the model's prefix
    /// state, and the append-only journal holds exactly the facts recorded
    /// (none lost, none duplicated).
    #[test]
    fn kill_injection_yields_exact_prefix(
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
            prop_assert_eq!(&observed, &model.states, "crash reopen != prefix replay");

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
