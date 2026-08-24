use std::{fs, process::Command};

#[test]
fn lifecycle_advance_appends_only_the_next_valid_event() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let journal = directory.path().join("events.jsonl");
    let output = Command::new(env!("CARGO_BIN_EXE_modernlink"))
        .args(["lifecycle", "advance", "--journal"])
        .arg(&journal)
        .args([
            "--run-id",
            "modernization-002",
            "--artifact",
            "sha256:evidence",
        ])
        .output()
        .expect("run lifecycle advance");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let transition: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("transition JSON");
    assert_eq!(transition["event"]["from"], "SETUP");
    assert_eq!(transition["event"]["to"], "DISCOVER");
    assert_eq!(
        transition["event"]["artifact_hashes"],
        serde_json::json!(["sha256:evidence"])
    );
    assert_eq!(
        fs::read_to_string(journal)
            .expect("journal")
            .lines()
            .count(),
        1
    );
}

#[test]
fn lifecycle_advance_refuses_to_enter_modernize_without_approval() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let journal = directory.path().join("events.jsonl");
    let run_id = "modernization-003";
    let mut snapshot = state::LifecycleSnapshot::new(run_id);
    for _ in 0..7 {
        let event = snapshot.next_event(false).expect("next preparation event");
        snapshot.apply(&event).expect("apply preparation event");
        state::append_event(&journal, &event).expect("append preparation event");
    }
    let output = Command::new(env!("CARGO_BIN_EXE_modernlink"))
        .args(["lifecycle", "advance", "--journal"])
        .arg(&journal)
        .args(["--run-id", run_id])
        .output()
        .expect("run unapproved lifecycle advance");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("requires explicit approval"));
    assert_eq!(
        fs::read_to_string(journal)
            .expect("journal")
            .lines()
            .count(),
        7
    );
}
