use std::{fs, process::Command};

#[test]
fn plan_command_writes_an_evidence_linked_dependency_dag() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let graph_path = directory.path().join("evidence.json");
    let seams_path = directory.path().join("seams.json");
    let compatibility_path = directory.path().join("compatibility.json");
    let plan_path = directory.path().join("plan.json");
    let graph = model::EvidenceGraph {
        schema_version: "modernlink.evidence/v1alpha1".to_owned(),
        evidence: vec![model::Evidence {
            id: "evidence:vendor".to_owned(),
            kind: "import".to_owned(),
            value: "weblogic.jndi.Environment".to_owned(),
            source: model::SourceLocation {
                path: "Legacy.java".to_owned(),
                start_byte: 0,
                end_byte: 20,
            },
        }],
        nodes: vec![model::GraphNode {
            id: "node:vendor".to_owned(),
            kind: "external-reference".to_owned(),
            name: "weblogic.jndi.Environment".to_owned(),
            evidence_ids: Vec::new(),
        }],
        edges: vec![model::GraphEdge {
            id: "edge:vendor".to_owned(),
            kind: "imports".to_owned(),
            source_id: "node:vendor".to_owned(),
            target_id: "node:vendor".to_owned(),
            evidence_ids: vec!["evidence:vendor".to_owned()],
        }],
        claims: Vec::new(),
    };
    fs::write(&graph_path, graph.canonical_json().expect("graph JSON")).expect("write graph");
    let seams = inference::infer_seams(&graph).expect("seams");
    fs::write(&seams_path, seams.canonical_json().expect("seam JSON")).expect("write seams");
    let compatibility = inference::assess_compatibility(&graph, 21).expect("compatibility");
    fs::write(
        &compatibility_path,
        compatibility.canonical_json().expect("compatibility JSON"),
    )
    .expect("write compatibility");
    let output = Command::new(env!("CARGO_BIN_EXE_modernlink"))
        .args(["plan", "--seams"])
        .arg(&seams_path)
        .args(["--compatibility"])
        .arg(&compatibility_path)
        .args(["--output"])
        .arg(&plan_path)
        .output()
        .expect("run plan");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let plan: serde_json::Value =
        serde_json::from_slice(&fs::read(plan_path).expect("plan report")).expect("plan JSON");
    assert_eq!(plan["schema_version"], "modernlink.migration-plan/v1alpha1");
    assert!(
        plan["tasks"]
            .as_array()
            .expect("tasks")
            .iter()
            .any(|task| task["kind"] == "seam-migration")
    );
}
