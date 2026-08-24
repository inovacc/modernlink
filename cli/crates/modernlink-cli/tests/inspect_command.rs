use std::{fs, process::Command};

use gix::bstr::ByteSlice;

#[test]
fn inspect_command_writes_the_shared_evidence_graph() {
    let repository = tempfile::tempdir().expect("temporary repository");
    fs::write(
        repository.path().join("PaymentService.java"),
        "package payments; import weblogic.jndi.Environment; public class PaymentService {}",
    )
    .expect("Java fixture");
    let output_path = repository.path().join("evidence.json");

    let output = Command::new(env!("CARGO_BIN_EXE_modernlink"))
        .args(["inspect", repository.path().to_str().unwrap(), "--output"])
        .arg(&output_path)
        .output()
        .expect("run modernlink inspect");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let graph: serde_json::Value =
        serde_json::from_slice(&fs::read(output_path).expect("evidence graph"))
            .expect("evidence JSON");
    assert_eq!(graph["schema_version"], "modernlink.evidence/v1alpha1");
    assert!(!graph["claims"].as_array().unwrap().is_empty());
}

#[test]
fn inspect_history_merges_syntax_and_git_evidence() {
    let repository_directory = tempfile::tempdir().expect("temporary repository");
    fs::write(
        repository_directory.path().join("PaymentService.java"),
        "package payments; public class PaymentService {}",
    )
    .expect("Java fixture");
    let repository = gix::init(repository_directory.path()).expect("initialize Git fixture");
    let signature = gix::actor::SignatureRef {
        name: b"Historian".as_bstr(),
        email: b"historian@example.test".as_bstr(),
        time: "1 +0000",
    };
    let tree = repository.empty_tree().id().detach();
    repository
        .commit_as(
            signature,
            signature,
            "refs/heads/main",
            "fixture\n",
            tree,
            std::iter::empty::<gix::ObjectId>(),
        )
        .expect("fixture commit");
    let output_path = repository_directory.path().join("evidence.json");

    let output = Command::new(env!("CARGO_BIN_EXE_modernlink"))
        .args([
            "inspect",
            repository_directory.path().to_str().unwrap(),
            "--history",
            "--output",
        ])
        .arg(&output_path)
        .output()
        .expect("run modernlink inspect history");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let graph: serde_json::Value =
        serde_json::from_slice(&fs::read(output_path).expect("evidence graph"))
            .expect("evidence JSON");
    assert!(
        graph["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|node| node["kind"] == "commit")
    );
}
