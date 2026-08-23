use std::{fs, process::Command};

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
