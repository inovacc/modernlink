use std::{fs, process::Command};

#[test]
fn setup_creates_only_owned_local_workspace_metadata() {
    let repository = tempfile::tempdir().expect("temporary repository");
    fs::create_dir(repository.path().join(".codex")).expect("Codex marker");
    fs::write(
        repository.path().join("AGENTS.md"),
        "user-owned instructions\n",
    )
    .expect("user file");

    let output = Command::new(env!("CARGO_BIN_EXE_modernlink"))
        .arg("setup")
        .arg(repository.path())
        .args(["--tools", "codex"])
        .output()
        .expect("run modernlink setup");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read_to_string(repository.path().join("AGENTS.md")).expect("user instructions"),
        "user-owned instructions\n"
    );
    let workspace: serde_json::Value = serde_json::from_slice(
        &fs::read(repository.path().join(".modernlink/workspace.json")).expect("workspace"),
    )
    .expect("workspace JSON");
    assert_eq!(workspace["schema_version"], "modernlink.workspace/v1alpha1");
    assert_eq!(
        workspace["selected_harnesses"],
        serde_json::json!(["codex"])
    );
    assert_eq!(
        workspace["detected_harnesses"],
        serde_json::json!(["codex"])
    );
    assert!(repository.path().join(".modernlink/cache").is_dir());
}

#[test]
fn setup_dry_run_does_not_create_a_workspace() {
    let repository = tempfile::tempdir().expect("temporary repository");
    let output = Command::new(env!("CARGO_BIN_EXE_modernlink"))
        .arg("setup")
        .arg(repository.path())
        .args(["--tools", "none", "--dry-run"])
        .output()
        .expect("run modernlink setup dry run");

    assert!(output.status.success());
    assert!(!repository.path().join(".modernlink").exists());
}
