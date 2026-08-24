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
    assert_eq!(
        fs::read_to_string(repository.path().join(".modernlink/.gitignore"))
            .expect("workspace ignore"),
        "# ModernLink local operational state\ncache/\nlocal/\nstate/\nworkspace.json\n"
    );
}

#[test]
fn setup_preserves_an_existing_workspace_ignore_file_that_has_required_rules() {
    let repository = tempfile::tempdir().expect("temporary repository");
    fs::create_dir(repository.path().join(".modernlink")).expect("workspace root");
    fs::write(
        repository.path().join(".modernlink/.gitignore"),
        "workspace.json\ncache/\nlocal/\nstate/\ncustom-rule\n",
    )
    .expect("existing workspace ignore");
    let output = Command::new(env!("CARGO_BIN_EXE_modernlink"))
        .arg("setup")
        .arg(repository.path())
        .args(["--tools", "none"])
        .output()
        .expect("run setup");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        fs::read_to_string(repository.path().join(".modernlink/.gitignore"))
            .expect("ignore")
            .contains("custom-rule")
    );
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

#[test]
fn setup_requires_explicit_tools_without_a_terminal() {
    let repository = tempfile::tempdir().expect("temporary repository");
    let output = Command::new(env!("CARGO_BIN_EXE_modernlink"))
        .arg("setup")
        .arg(repository.path())
        .output()
        .expect("run noninteractive setup");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("requires --tools"));
}

#[test]
fn harness_selection_can_add_remove_refresh_and_report_without_touching_user_files() {
    let repository = tempfile::tempdir().expect("temporary repository");
    fs::write(repository.path().join("AGENTS.md"), "user-owned\n").expect("user file");
    let binary = env!("CARGO_BIN_EXE_modernlink");

    let setup = Command::new(binary)
        .args(["setup"])
        .arg(repository.path())
        .args(["--tools", "none"])
        .output()
        .expect("setup workspace");
    assert!(setup.status.success());

    let add = Command::new(binary)
        .args(["harness", "add", "--repository"])
        .arg(repository.path())
        .arg("codex")
        .output()
        .expect("add harness");
    assert!(
        add.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&add.stderr)
    );
    let workspace: serde_json::Value = serde_json::from_slice(
        &fs::read(repository.path().join(".modernlink/workspace.json")).expect("workspace"),
    )
    .expect("workspace JSON");
    assert_eq!(
        workspace["selected_harnesses"],
        serde_json::json!(["codex"])
    );

    let doctor = Command::new(binary)
        .args(["harness", "doctor"])
        .arg(repository.path())
        .output()
        .expect("harness doctor");
    assert!(doctor.status.success());
    assert!(String::from_utf8_lossy(&doctor.stdout).contains("materialized"));

    let remove = Command::new(binary)
        .args(["harness", "remove", "--repository"])
        .arg(repository.path())
        .arg("codex")
        .output()
        .expect("remove harness");
    assert!(remove.status.success());
    assert_eq!(
        fs::read_to_string(repository.path().join("AGENTS.md")).expect("user file"),
        "user-owned\n"
    );
}
