use std::{fs, process::Command};

#[test]
fn domains_command_writes_candidate_context_hypotheses_only() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let evidence = directory.path().join("evidence.json");
    let output = directory.path().join("domains.json");
    let graph = model::EvidenceGraph {
        schema_version: "modernlink.evidence/v1alpha1".to_owned(),
        evidence: vec![model::Evidence {
            id: "evidence:authorization".to_owned(),
            kind: "type".to_owned(),
            value: "AuthorizationService".to_owned(),
            source: model::SourceLocation {
                path: "AuthorizationService.java".to_owned(),
                start_byte: 0,
                end_byte: 20,
            },
        }],
        nodes: vec![model::GraphNode {
            id: "node:authorization".to_owned(),
            kind: "type".to_owned(),
            name: "com.bank.authorization.AuthorizationService".to_owned(),
            evidence_ids: vec!["evidence:authorization".to_owned()],
        }],
        edges: Vec::new(),
        claims: Vec::new(),
    };
    fs::write(&evidence, graph.canonical_json().expect("evidence JSON")).expect("write evidence");

    let result = Command::new(env!("CARGO_BIN_EXE_modernlink"))
        .args(["domains", "--evidence"])
        .arg(&evidence)
        .args(["--output"])
        .arg(&output)
        .output()
        .expect("run domains");
    assert!(
        result.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    let report: serde_json::Value =
        serde_json::from_slice(&fs::read(output).expect("domains report")).expect("report JSON");
    assert_eq!(report["schema_version"], "modernlink.domains/v1alpha1");
    assert_eq!(report["contexts"][0]["state"], "HYPOTHESIS");
    assert_eq!(report["contexts"][0]["name"], "authorization");
}
