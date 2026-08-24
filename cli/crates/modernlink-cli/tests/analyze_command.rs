use std::{fs, process::Command};

use gix::bstr::ByteSlice;

#[test]
fn analyze_command_writes_a_report_for_a_real_java_repository() {
    let repository = tempfile::tempdir().expect("temporary repository");
    fs::write(
        repository.path().join("Customer.java"),
        "package customers; public class Customer {}",
    )
    .expect("Java fixture");
    let report = repository.path().join("modernlink-analysis.json");

    let output = Command::new(env!("CARGO_BIN_EXE_modernlink"))
        .args(["analyze", repository.path().to_str().unwrap(), "--output"])
        .arg(&report)
        .output()
        .expect("run modernlink");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(report.is_file());
    let json: serde_json::Value =
        serde_json::from_slice(&fs::read(report).expect("analysis report")).expect("JSON report");
    assert_eq!(json["summary"]["java_files"], 1);
    assert_eq!(json["nodes"][0]["qualified_name"], "customers");
}

#[test]
fn analyze_history_writes_a_sibling_git_artifact_without_changing_analysis_schema() {
    let repository_directory = tempfile::tempdir().expect("temporary repository");
    fs::write(
        repository_directory.path().join("Customer.java"),
        "package customers; public class Customer {}",
    )
    .expect("Java fixture");
    let repository = gix::init(repository_directory.path()).expect("initialize Git fixture");
    let signature = gix::actor::SignatureRef {
        name: b"Analyzer Historian".as_bstr(),
        email: b"analyzer@example.test".as_bstr(),
        time: "1 +0000",
    };
    repository
        .commit_as(
            signature,
            signature,
            "refs/heads/main",
            "fixture\n",
            repository.empty_tree().id().detach(),
            std::iter::empty::<gix::ObjectId>(),
        )
        .expect("create Git fixture commit");
    let report = repository_directory.path().join("reports/analysis.json");

    let output = Command::new(env!("CARGO_BIN_EXE_modernlink"))
        .args([
            "analyze",
            repository_directory.path().to_str().unwrap(),
            "--history",
        ])
        .args(["--output"])
        .arg(&report)
        .output()
        .expect("run modernlink analyze history");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let analysis: serde_json::Value =
        serde_json::from_slice(&fs::read(&report).expect("analysis report"))
            .expect("analysis JSON");
    let history: serde_json::Value = serde_json::from_slice(
        &fs::read(repository_directory.path().join("reports/git-history.json"))
            .expect("history report"),
    )
    .expect("history JSON");
    assert_eq!(analysis["schema_version"], "modernlink.analysis/v1alpha1");
    assert_eq!(history["schema_version"], "modernlink.git-history/v1alpha1");
}
