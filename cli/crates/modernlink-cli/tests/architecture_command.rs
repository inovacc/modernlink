use std::{fs, process::Command};

#[test]
fn architecture_command_derives_labeled_structure_from_a_shared_graph() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let evidence_path = directory.path().join("evidence.json");
    let output_path = directory.path().join("architecture.json");
    let graph = model::EvidenceGraph {
        schema_version: "modernlink.evidence/v1alpha1".to_owned(),
        evidence: vec![model::Evidence {
            id: "evidence:service".to_owned(),
            kind: "java-type".to_owned(),
            value: "PaymentService".to_owned(),
            source: model::SourceLocation {
                path: "PaymentService.java".to_owned(),
                start_byte: 0,
                end_byte: 14,
            },
        }],
        nodes: vec![model::GraphNode {
            id: "node:service".to_owned(),
            kind: "type".to_owned(),
            name: "com.bank.payments.PaymentService".to_owned(),
            evidence_ids: vec!["evidence:service".to_owned()],
        }],
        edges: Vec::new(),
        claims: Vec::new(),
    };
    fs::write(
        &evidence_path,
        graph.canonical_json().expect("canonical graph"),
    )
    .expect("write graph");

    let output = Command::new(env!("CARGO_BIN_EXE_modernlink"))
        .args(["architecture", "--evidence"])
        .arg(&evidence_path)
        .arg("--output")
        .arg(&output_path)
        .output()
        .expect("run modernlink architecture");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value =
        serde_json::from_slice(&fs::read(output_path).expect("architecture report"))
            .expect("architecture JSON");
    assert_eq!(report["schema_version"], "modernlink.structure/v1alpha1");
    assert_eq!(report["layers"][0]["state"], "INFERENCE");
    assert_eq!(report["contexts"][0]["state"], "HYPOTHESIS");
}
