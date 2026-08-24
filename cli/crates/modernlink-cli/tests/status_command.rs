use std::{fs, process::Command};

#[test]
fn status_command_recovers_a_read_only_lifecycle_snapshot() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let journal = directory.path().join("events.jsonl");
    let event = state::LifecycleEvent::new(
        1,
        state::LifecyclePhase::Setup,
        state::LifecyclePhase::Discover,
        false,
    );
    fs::write(
        &journal,
        state::event_json_line(&event).expect("journal event"),
    )
    .expect("write journal");

    let output = Command::new(env!("CARGO_BIN_EXE_modernlink"))
        .args(["status", "--journal"])
        .arg(&journal)
        .args(["--run-id", "modernization-001"])
        .output()
        .expect("run modernlink status");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let status: serde_json::Value = serde_json::from_slice(&output.stdout).expect("status JSON");
    assert_eq!(
        status["schema_version"],
        "modernlink.lifecycle-status/v1alpha1"
    );
    assert_eq!(status["snapshot"]["phase"], "DISCOVER");
    assert_eq!(status["snapshot"]["event_count"], 1);
}
