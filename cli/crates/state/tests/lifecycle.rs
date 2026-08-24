use state::{LifecycleEvent, LifecyclePhase, LifecycleSnapshot, StateError};
use tempfile::tempdir;

#[test]
fn lifecycle_rejects_skipped_or_unapproved_transitions() {
    let mut snapshot = LifecycleSnapshot::new("run:fixture");
    let skipped = LifecycleEvent::new(1, LifecyclePhase::Setup, LifecyclePhase::Assess, false);
    assert!(matches!(
        snapshot.apply(&skipped),
        Err(StateError::InvalidTransition { .. })
    ));

    for (sequence, from, to) in [
        (1, LifecyclePhase::Setup, LifecyclePhase::Discover),
        (2, LifecyclePhase::Discover, LifecyclePhase::Understand),
        (3, LifecyclePhase::Understand, LifecyclePhase::Model),
        (4, LifecyclePhase::Model, LifecyclePhase::Assess),
        (5, LifecyclePhase::Assess, LifecyclePhase::Design),
        (6, LifecyclePhase::Design, LifecyclePhase::Plan),
        (7, LifecyclePhase::Plan, LifecyclePhase::Prepare),
    ] {
        snapshot
            .apply(&LifecycleEvent::new(sequence, from, to, false))
            .expect("allowed preparation transition");
    }
    let modernize =
        LifecycleEvent::new(8, LifecyclePhase::Prepare, LifecyclePhase::Modernize, false);
    assert!(matches!(
        snapshot.apply(&modernize),
        Err(StateError::ApprovalRequired { .. })
    ));
}

#[test]
fn replay_recovers_the_same_snapshot_from_append_only_events() {
    let events = vec![
        LifecycleEvent::new(1, LifecyclePhase::Setup, LifecyclePhase::Discover, false),
        LifecycleEvent::new(
            2,
            LifecyclePhase::Discover,
            LifecyclePhase::Understand,
            false,
        ),
    ];

    let recovered = LifecycleSnapshot::replay("run:fixture", &events).expect("replay events");
    assert_eq!(recovered.phase, LifecyclePhase::Understand);
    assert_eq!(recovered.last_sequence, 2);
    assert_eq!(recovered.event_count, 2);
}

#[test]
fn append_only_journal_recovers_a_snapshot_after_restart() {
    let directory = tempdir().expect("temporary journal directory");
    let journal = directory.path().join("events.jsonl");
    state::append_event(
        &journal,
        &LifecycleEvent::new(1, LifecyclePhase::Setup, LifecyclePhase::Discover, false),
    )
    .expect("append first event");
    state::append_event(
        &journal,
        &LifecycleEvent::new(
            2,
            LifecyclePhase::Discover,
            LifecyclePhase::Understand,
            false,
        ),
    )
    .expect("append second event");

    let recovered = state::recover_journal("run:fixture", &journal).expect("recover journal");
    assert_eq!(recovered.phase, LifecyclePhase::Understand);
    assert_eq!(recovered.last_sequence, 2);
}
