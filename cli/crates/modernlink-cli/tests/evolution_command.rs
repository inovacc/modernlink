use std::{fs, process::Command};

#[test]
fn evolution_command_writes_a_non_ranked_history_xray() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let history_path = directory.path().join("history.json");
    let output_path = directory.path().join("evolution.json");
    let history = git::GitHistorySnapshot::empty(git::RepositoryIdentity {
        repository_digest: "repository:fixture".to_owned(),
        object_format: "sha1".to_owned(),
    });
    fs::write(
        &history_path,
        history.canonical_json().expect("history JSON"),
    )
    .expect("write history");
    let output = Command::new(env!("CARGO_BIN_EXE_modernlink"))
        .args(["evolution", "--history"])
        .arg(&history_path)
        .args(["--output"])
        .arg(&output_path)
        .output()
        .expect("run evolution");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value =
        serde_json::from_slice(&fs::read(output_path).expect("evolution report"))
            .expect("evolution JSON");
    assert_eq!(
        report["schema_version"],
        "modernlink.git-evolution/v1alpha1"
    );
    assert!(
        report["limitations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item.as_str().unwrap().contains("not productivity"))
    );
}
