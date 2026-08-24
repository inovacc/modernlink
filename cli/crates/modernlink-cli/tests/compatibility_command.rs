use std::{fs, process::Command};

#[test]
fn compatibility_command_writes_target_review_findings() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let evidence_path = directory.path().join("evidence.json");
    let output_path = directory.path().join("compatibility.json");
    let graph = model::EvidenceGraph {
        schema_version: "modernlink.evidence/v1alpha1".to_owned(),
        evidence: vec![model::Evidence {
            id: "evidence:internal-jdk".to_owned(),
            kind: "import".to_owned(),
            value: "sun.misc.Unsafe".to_owned(),
            source: model::SourceLocation {
                path: "Legacy.java".to_owned(),
                start_byte: 0,
                end_byte: 15,
            },
        }],
        nodes: vec![model::GraphNode {
            id: "node:internal-jdk".to_owned(),
            kind: "external-reference".to_owned(),
            name: "sun.misc.Unsafe".to_owned(),
            evidence_ids: Vec::new(),
        }],
        edges: vec![model::GraphEdge {
            id: "edge:internal-jdk".to_owned(),
            kind: "imports".to_owned(),
            source_id: "node:internal-jdk".to_owned(),
            target_id: "node:internal-jdk".to_owned(),
            evidence_ids: vec!["evidence:internal-jdk".to_owned()],
        }],
        claims: Vec::new(),
    };
    fs::write(
        &evidence_path,
        graph.canonical_json().expect("canonical graph"),
    )
    .expect("write graph");

    let output = Command::new(env!("CARGO_BIN_EXE_modernlink"))
        .args(["compatibility", "--evidence"])
        .arg(&evidence_path)
        .args(["--target", "21", "--output"])
        .arg(&output_path)
        .output()
        .expect("run modernlink compatibility");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value =
        serde_json::from_slice(&fs::read(output_path).expect("compatibility report"))
            .expect("compatibility JSON");
    assert_eq!(
        report["schema_version"],
        "modernlink.compatibility/v1alpha1"
    );
    assert_eq!(report["target_version"], 21);
    assert_eq!(report["findings"][0]["category"], "internal-jdk-api");
}
