use std::{fs, process::Command};

use gix::bstr::ByteSlice;

#[test]
fn history_command_writes_versioned_local_history_evidence() {
    let repository_directory = tempfile::tempdir().expect("temporary repository");
    let repository = gix::init(repository_directory.path()).expect("initialize repository");
    let tree = repository.empty_tree().id().detach();
    let signature = gix::actor::SignatureRef {
        name: b"Historian".as_bstr(),
        email: b"historian@example.test".as_bstr(),
        time: "1 +0000",
    };
    let first = repository
        .commit_as(
            signature,
            signature,
            "refs/heads/main",
            "private fixture message\n",
            tree,
            std::iter::empty::<gix::ObjectId>(),
        )
        .expect("create fixture commit")
        .detach();
    repository
        .commit_as(
            signature,
            signature,
            "refs/heads/main",
            "second private fixture message\n",
            tree,
            std::iter::once(first),
        )
        .expect("create second fixture commit");
    let report = repository_directory.path().join("git-history.json");

    let output = Command::new(env!("CARGO_BIN_EXE_modernlink"))
        .args([
            "history",
            repository_directory.path().to_str().unwrap(),
            "--output",
        ])
        .arg(&report)
        .args(["--refs", "all", "--max-commits", "1", "--mailmap", "off"])
        .output()
        .expect("run modernlink history");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value =
        serde_json::from_slice(&fs::read(&report).expect("history report")).expect("JSON report");
    assert_eq!(json["schema_version"], "modernlink.git-history/v1alpha1");
    assert_eq!(json["commits"].as_array().unwrap().len(), 1);
    assert!(
        json["completeness"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["code"] == "commit-limit-reached")
    );
    assert!(
        json["commits"][0]["message_fingerprint"]
            .as_str()
            .unwrap()
            .starts_with("sha256:")
    );
    assert!(
        !String::from_utf8_lossy(&fs::read(&report).unwrap()).contains("private fixture message")
    );

    let repeat = Command::new(env!("CARGO_BIN_EXE_modernlink"))
        .args([
            "history",
            repository_directory.path().to_str().unwrap(),
            "--output",
        ])
        .arg(&report)
        .output()
        .expect("rerun modernlink history");
    assert!(!repeat.status.success());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&fs::read(&report).unwrap()).unwrap(),
        json
    );
}
