use model::{Claim, ClaimState, Evidence, EvidenceGraph, GraphError, GraphNode, SourceLocation};

#[test]
fn graph_rejects_claims_that_cite_missing_evidence() {
    let graph = EvidenceGraph {
        schema_version: "modernlink.evidence/v1alpha1".to_owned(),
        evidence: Vec::new(),
        nodes: Vec::new(),
        claims: vec![Claim {
            id: "claim:missing".to_owned(),
            state: ClaimState::Hypothesis,
            summary: "Candidate payment boundary".to_owned(),
            evidence_ids: vec!["evidence:missing".to_owned()],
        }],
    };

    assert!(matches!(
        graph.validate(),
        Err(GraphError::MissingEvidence { .. })
    ));
}

#[test]
fn graph_canonicalizes_facts_and_preserves_traceability() {
    let graph = EvidenceGraph {
        schema_version: "modernlink.evidence/v1alpha1".to_owned(),
        evidence: vec![Evidence {
            id: "evidence:payment-service".to_owned(),
            kind: "java-type".to_owned(),
            value: "PaymentService".to_owned(),
            source: SourceLocation {
                path: "src/PaymentService.java".to_owned(),
                start_byte: 0,
                end_byte: 20,
            },
        }],
        nodes: vec![GraphNode {
            id: "node:payment-service".to_owned(),
            kind: "class".to_owned(),
            name: "PaymentService".to_owned(),
            evidence_ids: vec!["evidence:payment-service".to_owned()],
        }],
        claims: vec![Claim {
            id: "claim:payment-context".to_owned(),
            state: ClaimState::Hypothesis,
            summary: "Candidate payment context".to_owned(),
            evidence_ids: vec!["evidence:payment-service".to_owned()],
        }],
    };

    let canonical = graph.canonical_json().expect("canonical graph");
    assert!(canonical.contains("Candidate payment context"));
    assert!(canonical.contains("evidence:payment-service"));
    assert!(
        EvidenceGraph::from_json(&canonical)
            .expect("parse canonical graph")
            .validate()
            .is_ok()
    );
}

#[test]
fn graph_rejects_an_observed_fact_as_an_agent_claim() {
    let graph = EvidenceGraph {
        schema_version: "modernlink.evidence/v1alpha1".to_owned(),
        evidence: Vec::new(),
        nodes: Vec::new(),
        claims: vec![Claim {
            id: "claim:misclassified".to_owned(),
            state: ClaimState::Fact,
            summary: "Agent assertion presented as a fact".to_owned(),
            evidence_ids: Vec::new(),
        }],
    };

    assert!(matches!(
        graph.validate(),
        Err(GraphError::InvalidClaimState(_))
    ));
}
