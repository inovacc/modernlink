use inference::{assess_compatibility, infer_seams, infer_structure};
use model::{Evidence, EvidenceGraph, GraphEdge, GraphNode, SourceLocation};

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

#[test]
fn seam_inference_decomposes_vendor_coupling_without_claiming_a_fact() {
    let graph = EvidenceGraph {
        schema_version: "modernlink.evidence/v1alpha1".to_owned(),
        evidence: vec![Evidence {
            id: "evidence:vendor-import".to_owned(),
            kind: "import".to_owned(),
            value: "weblogic.jndi.Environment".to_owned(),
            source: SourceLocation {
                path: "PaymentService.java".to_owned(),
                start_byte: 0,
                end_byte: 20,
            },
        }],
        nodes: vec![
            GraphNode {
                id: "node:service".to_owned(),
                kind: "type".to_owned(),
                name: "com.bank.payments.PaymentService".to_owned(),
                evidence_ids: vec!["evidence:vendor-import".to_owned()],
            },
            GraphNode {
                id: "node:vendor".to_owned(),
                kind: "external-reference".to_owned(),
                name: "weblogic.jndi.Environment".to_owned(),
                evidence_ids: Vec::new(),
            },
        ],
        edges: vec![GraphEdge {
            id: "edge:vendor-import".to_owned(),
            kind: "imports".to_owned(),
            source_id: "node:service".to_owned(),
            target_id: "node:vendor".to_owned(),
            evidence_ids: vec!["evidence:vendor-import".to_owned()],
        }],
        claims: Vec::new(),
    };

    let report = infer_seams(&graph).expect("seam inference");
    let seam = report.seams.first().expect("vendor seam");
    assert_eq!(seam.state, model::ClaimState::Inference);
    assert_eq!(seam.current_technology, "weblogic");
    assert!(
        seam.score_components
            .iter()
            .any(|component| { component.factor == "vendor-lock" && component.points > 0 })
    );
    assert_eq!(seam.recommended_mode, "PASSTHROUGH -> SHADOW -> REDIRECT");
}

#[test]
fn compatibility_assessment_links_target_reviews_to_import_evidence() {
    let graph = EvidenceGraph {
        schema_version: "modernlink.evidence/v1alpha1".to_owned(),
        evidence: vec![Evidence {
            id: "evidence:jaxb".to_owned(),
            kind: "import".to_owned(),
            value: "javax.xml.bind.JAXBContext".to_owned(),
            source: SourceLocation {
                path: "PaymentXml.java".to_owned(),
                start_byte: 0,
                end_byte: 25,
            },
        }],
        nodes: vec![GraphNode {
            id: "node:jaxb".to_owned(),
            kind: "external-reference".to_owned(),
            name: "javax.xml.bind.JAXBContext".to_owned(),
            evidence_ids: Vec::new(),
        }],
        edges: vec![GraphEdge {
            id: "edge:jaxb".to_owned(),
            kind: "imports".to_owned(),
            source_id: "node:jaxb".to_owned(),
            target_id: "node:jaxb".to_owned(),
            evidence_ids: vec!["evidence:jaxb".to_owned()],
        }],
        claims: Vec::new(),
    };

    let report = assess_compatibility(&graph, 21).expect("compatibility assessment");
    let finding = report.findings.first().expect("JAXB review finding");
    assert_eq!(report.target_version, 21);
    assert_eq!(finding.state, model::ClaimState::Inference);
    assert_eq!(finding.category, "java-ee-api-review");
    assert_eq!(finding.evidence_ids, vec!["evidence:jaxb"]);
}
