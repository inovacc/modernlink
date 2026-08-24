use inference::infer_structure;
use model::{Evidence, EvidenceGraph, GraphNode, SourceLocation};

#[test]
fn structural_inference_labels_layers_and_candidate_contexts_as_inferences() {
    let graph = EvidenceGraph {
        schema_version: "modernlink.evidence/v1alpha1".to_owned(),
        evidence: vec![Evidence {
            id: "evidence:service".to_owned(),
            kind: "java-type".to_owned(),
            value: "PaymentService".to_owned(),
            source: SourceLocation {
                path: "src/payments/PaymentService.java".to_owned(),
                start_byte: 0,
                end_byte: 14,
            },
        }],
        nodes: vec![GraphNode {
            id: "node:service".to_owned(),
            kind: "type".to_owned(),
            name: "com.bank.payments.PaymentService".to_owned(),
            evidence_ids: vec!["evidence:service".to_owned()],
        }],
        edges: Vec::new(),
        claims: Vec::new(),
    };

    let report = infer_structure(&graph).expect("structural inference");

    assert!(report.layers.iter().any(|layer| {
        layer.layer == "service"
            && layer.state == model::ClaimState::Inference
            && layer.evidence_ids == vec!["evidence:service"]
    }));
    assert!(report.contexts.iter().any(|context| {
        context.name == "payments"
            && context.state == model::ClaimState::Hypothesis
            && context.evidence_ids == vec!["evidence:service"]
    }));
}
