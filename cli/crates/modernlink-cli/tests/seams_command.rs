use std::{fs, process::Command};

#[test]
fn seams_command_writes_decomposed_vendor_seams() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let evidence_path = directory.path().join("evidence.json");
    let output_path = directory.path().join("seams.json");
    let graph = model::EvidenceGraph {
        schema_version: "modernlink.evidence/v1alpha1".to_owned(),
        evidence: vec![model::Evidence {
            id: "evidence:vendor".to_owned(),
            kind: "import".to_owned(),
            value: "weblogic.jndi.Environment".to_owned(),
            source: model::SourceLocation {
                path: "Payment.java".to_owned(),
                start_byte: 0,
                end_byte: 20,
            },
        }],
        nodes: vec![
            model::GraphNode {
                id: "node:payment".to_owned(),
                kind: "type".to_owned(),
                name: "com.bank.PaymentService".to_owned(),
                evidence_ids: vec!["evidence:vendor".to_owned()],
            },
            model::GraphNode {
                id: "node:vendor".to_owned(),
                kind: "external-reference".to_owned(),
                name: "weblogic.jndi.Environment".to_owned(),
                evidence_ids: Vec::new(),
            },
        ],
        edges: vec![model::GraphEdge {
            id: "edge:vendor".to_owned(),
            kind: "imports".to_owned(),
            source_id: "node:payment".to_owned(),
            target_id: "node:vendor".to_owned(),
            evidence_ids: vec!["evidence:vendor".to_owned()],
        }],
        claims: Vec::new(),
    };
    fs::write(
        &evidence_path,
        graph.canonical_json().expect("canonical graph"),
    )
    .expect("write graph");

    let output = Command::new(env!("CARGO_BIN_EXE_modernlink"))
        .args(["seams", "--evidence"])
        .arg(&evidence_path)
        .arg("--output")
        .arg(&output_path)
        .output()
        .expect("run modernlink seams");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value =
        serde_json::from_slice(&fs::read(output_path).expect("seam report")).expect("seam JSON");
    assert_eq!(report["schema_version"], "modernlink.seams/v1alpha1");
    assert_eq!(report["seams"][0]["current_technology"], "weblogic");
    assert_eq!(
        report["seams"][0]["location_name"],
        "com.bank.PaymentService"
    );
    assert!(
        report["seams"][0]["score_components"]
            .as_array()
            .unwrap()
            .iter()
            .any(|component| component["factor"] == "vendor-lock")
    );
}
