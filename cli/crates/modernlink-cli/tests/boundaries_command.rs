use std::{fs, process::Command};

#[test]
fn boundaries_command_writes_annotation_backed_candidates() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let evidence_path = directory.path().join("evidence.json");
    let output_path = directory.path().join("boundaries.json");
    let graph = model::EvidenceGraph {
        schema_version: "modernlink.evidence/v1alpha1".to_owned(),
        evidence: vec![model::Evidence {
            id: "evidence:path".to_owned(),
            kind: "annotation".to_owned(),
            value: "Path".to_owned(),
            source: model::SourceLocation {
                path: "Api.java".to_owned(),
                start_byte: 0,
                end_byte: 5,
            },
        }],
        nodes: Vec::new(),
        edges: Vec::new(),
        claims: Vec::new(),
    };
    fs::write(&evidence_path, graph.canonical_json().expect("graph JSON")).expect("write graph");
    let output = Command::new(env!("CARGO_BIN_EXE_modernlink"))
        .args(["boundaries", "--evidence"])
        .arg(&evidence_path)
        .args(["--output"])
        .arg(&output_path)
        .output()
        .expect("run boundaries");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value =
        serde_json::from_slice(&fs::read(output_path).expect("report")).expect("JSON");
    assert_eq!(report["schema_version"], "modernlink.boundaries/v1alpha1");
    assert_eq!(report["boundaries"][0]["kind"], "http");
}
