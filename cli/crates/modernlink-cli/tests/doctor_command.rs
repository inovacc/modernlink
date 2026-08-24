use std::{fs, process::Command};

#[test]
fn doctor_reports_local_preconditions_without_creating_workspace_state() {
    let repository = tempfile::tempdir().expect("temporary repository");
    fs::create_dir(repository.path().join(".codex")).expect("Codex marker");
    let output = Command::new(env!("CARGO_BIN_EXE_modernlink"))
        .arg("doctor")
        .arg(repository.path())
        .output()
        .expect("run modernlink doctor");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!repository.path().join(".modernlink").exists());
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).expect("doctor JSON");
    assert_eq!(report["schema_version"], "modernlink.doctor/v1alpha1");
    assert_eq!(report["checks"]["workspace"]["status"], "MISSING");
    assert!(
        report["checks"]["harnesses"]["detected"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("codex"))
    );
}
