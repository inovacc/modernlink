use std::{fs, process::Command};

#[test]
fn plugin_install_materializes_the_canonical_bundle_only_at_an_explicit_new_destination() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let destination = directory.path().join("modernlink-plugin");
    let output = Command::new(env!("CARGO_BIN_EXE_modernlink"))
        .args(["plugin", "install", "--destination"])
        .arg(&destination)
        .output()
        .expect("run plugin install");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        destination
            .join("skills/modernlink-plan/SKILL.md")
            .is_file()
    );
    assert!(
        destination
            .join("agents/modernization-orchestrator.md")
            .is_file()
    );
    assert!(destination.join("commands/verify.md").is_file());
    assert!(destination.join("commands/architecture.md").is_file());
    assert!(destination.join("commands/boundaries.md").is_file());
    assert!(
        destination
            .join("skills/modernlink-architecture/SKILL.md")
            .is_file()
    );
    assert!(destination.join(".codex-plugin/plugin.json").is_file());
    assert!(
        destination
            .join("config/binary-pointer.schema.json")
            .is_file()
    );
    assert!(!destination.join("config/binary-pointer.json").exists());
    let second = Command::new(env!("CARGO_BIN_EXE_modernlink"))
        .args(["plugin", "install", "--destination"])
        .arg(&destination)
        .output()
        .expect("rerun plugin install");
    assert!(!second.status.success());
    assert!(
        fs::read_to_string(destination.join("WORKFLOWS.md"))
            .expect("workflow")
            .contains("MODERNIZE")
    );
}
