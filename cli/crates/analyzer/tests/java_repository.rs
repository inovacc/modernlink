use std::fs;

use modernlink_analyzer::{AnalysisReport, analyze_repository};

#[test]
fn analyzes_java_repository_into_deterministic_evidence_graph() {
    let repository = tempfile::tempdir().expect("temporary repository");
    let source_dir = repository.path().join("src/main/java/com/acme/orders");
    fs::create_dir_all(&source_dir).expect("source directory");
    fs::write(
        source_dir.join("OrderService.java"),
        r#"package com.acme.orders;

import com.acme.payments.PaymentGateway;

public class OrderService {
    private final PaymentGateway payments;

    public OrderService(PaymentGateway payments) {
        this.payments = payments;
    }

    public void placeOrder() {
        payments.charge();
    }
}
"#,
    )
    .expect("Java fixture");

    let first = analyze_repository(repository.path()).expect("first analysis");
    let second = analyze_repository(repository.path()).expect("second analysis");

    assert_eq!(first, second);
    assert_eq!(first.schema_version, "modernlink.analysis/v1alpha1");
    assert_eq!(first.summary.java_files, 1);
    assert_eq!(first.summary.parse_errors, 0);
    assert!(first.artifacts.iter().any(|artifact| {
        artifact.path == "src/main/java/com/acme/orders/OrderService.java"
            && artifact.digest.starts_with("sha256:")
    }));
    assert!(
        first
            .nodes
            .iter()
            .any(|node| { node.kind == "package" && node.qualified_name == "com.acme.orders" })
    );
    assert!(first.nodes.iter().any(|node| {
        node.kind == "type" && node.qualified_name == "com.acme.orders.OrderService"
    }));
    assert!(first.edges.iter().any(|edge| {
        edge.kind == "imports" && edge.target_name == "com.acme.payments.PaymentGateway"
    }));
    assert!(first.evidence.iter().all(|evidence| {
        evidence.path == "src/main/java/com/acme/orders/OrderService.java"
            && evidence.start_byte < evidence.end_byte
            && evidence.collector == "java-tree-sitter"
    }));

    let json = first.canonical_json().expect("canonical JSON");
    let decoded: AnalysisReport = serde_json::from_str(&json).expect("report round trip");
    assert_eq!(decoded, first);
}

#[test]
fn coalesces_shared_semantic_nodes_and_preserves_all_evidence() {
    let repository = tempfile::tempdir().expect("temporary repository");
    fs::write(
        repository.path().join("First.java"),
        "package shared; import external.Port; public class First {}",
    )
    .expect("first Java fixture");
    fs::write(
        repository.path().join("Second.java"),
        "package shared; import external.Port; public class Second {}",
    )
    .expect("second Java fixture");

    let report = analyze_repository(repository.path()).expect("analysis");
    let packages = report
        .nodes
        .iter()
        .filter(|node| node.kind == "package" && node.qualified_name == "shared")
        .collect::<Vec<_>>();
    let imports = report
        .edges
        .iter()
        .filter(|edge| edge.kind == "imports" && edge.target_name == "external.Port")
        .collect::<Vec<_>>();

    assert_eq!(packages.len(), 1);
    assert_eq!(packages[0].evidence_ids.len(), 2);
    assert_eq!(imports.len(), 1);
    assert_eq!(imports[0].evidence_ids.len(), 2);
}
