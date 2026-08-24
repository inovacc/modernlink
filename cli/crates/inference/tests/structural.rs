use inference::{
    assess_compatibility, infer_boundaries, infer_seams, infer_structure, plan_migration,
};
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
fn boundary_inference_distinguishes_transaction_descriptor_evidence() {
    let graph = EvidenceGraph {
        schema_version: "modernlink.evidence/v1alpha1".to_owned(),
        evidence: vec![Evidence {
            id: "evidence:jta-descriptor".to_owned(),
            kind: "transaction-descriptor-reference".to_owned(),
            value: "JTA".to_owned(),
            source: SourceLocation {
                path: "WEB-INF/weblogic.xml".to_owned(),
                start_byte: 20,
                end_byte: 23,
            },
        }],
        nodes: Vec::new(),
        edges: Vec::new(),
        claims: Vec::new(),
    };

    let boundary = infer_boundaries(&graph)
        .expect("boundary inference")
        .boundaries
        .remove(0);
    assert_eq!(boundary.kind, "transaction");
    assert_eq!(boundary.source_kind, "transaction-descriptor-reference");
    assert_eq!(boundary.annotation, "JTA");
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
    assert_eq!(seam.location_name, "com.bank.payments.PaymentService");
    assert!(
        seam.score_components
            .iter()
            .any(|component| { component.factor == "vendor-lock" && component.points > 0 })
    );
    assert_eq!(seam.recommended_mode, "PASSTHROUGH -> SHADOW -> REDIRECT");
}

#[test]
fn seam_inference_identifies_a_jms_boundary_and_its_migration_mode() {
    let graph = EvidenceGraph {
        schema_version: "modernlink.evidence/v1alpha1".to_owned(),
        evidence: vec![Evidence {
            id: "evidence:jms".to_owned(),
            kind: "import".to_owned(),
            value: "javax.jms.Queue".to_owned(),
            source: SourceLocation {
                path: "QueueAdapter.java".to_owned(),
                start_byte: 0,
                end_byte: 15,
            },
        }],
        nodes: vec![
            GraphNode {
                id: "node:adapter".to_owned(),
                kind: "type".to_owned(),
                name: "com.bank.QueueAdapter".to_owned(),
                evidence_ids: vec!["evidence:jms".to_owned()],
            },
            GraphNode {
                id: "node:jms".to_owned(),
                kind: "external-reference".to_owned(),
                name: "javax.jms.Queue".to_owned(),
                evidence_ids: Vec::new(),
            },
        ],
        edges: vec![GraphEdge {
            id: "edge:jms".to_owned(),
            kind: "imports".to_owned(),
            source_id: "node:adapter".to_owned(),
            target_id: "node:jms".to_owned(),
            evidence_ids: vec!["evidence:jms".to_owned()],
        }],
        claims: Vec::new(),
    };
    let seam = infer_seams(&graph).expect("JMS seam").seams.remove(0);
    assert_eq!(seam.current_technology, "jms");
    assert_eq!(seam.recommended_mode, "SHADOW -> MIRROR -> REDIRECT");
    assert!(
        seam.score_components
            .iter()
            .any(|component| component.factor == "messaging-boundary")
    );
}

#[test]
fn seam_inference_keeps_sql_table_writes_as_low_confidence_data_candidates() {
    let graph = EvidenceGraph {
        schema_version: "modernlink.evidence/v1alpha1".to_owned(),
        evidence: vec![Evidence {
            id: "evidence:sql-write".to_owned(),
            kind: "sql-table-write".to_owned(),
            value: "ledger.payment".to_owned(),
            source: SourceLocation {
                path: "db/migrations/V001.sql".to_owned(),
                start_byte: 12,
                end_byte: 26,
            },
        }],
        nodes: vec![
            GraphNode {
                id: "node:sql".to_owned(),
                kind: "artifact".to_owned(),
                name: "db/migrations/V001.sql".to_owned(),
                evidence_ids: Vec::new(),
            },
            GraphNode {
                id: "node:table".to_owned(),
                kind: "database-table".to_owned(),
                name: "ledger.payment".to_owned(),
                evidence_ids: vec!["evidence:sql-write".to_owned()],
            },
        ],
        edges: vec![GraphEdge {
            id: "edge:sql-write".to_owned(),
            kind: "writes".to_owned(),
            source_id: "node:sql".to_owned(),
            target_id: "node:table".to_owned(),
            evidence_ids: vec!["evidence:sql-write".to_owned()],
        }],
        claims: Vec::new(),
    };

    let seam = infer_seams(&graph).expect("SQL seam").seams.remove(0);

    assert_eq!(seam.state, model::ClaimState::Inference);
    assert_eq!(seam.seam_type, "database-table-writes");
    assert_eq!(seam.current_technology, "database");
    assert_eq!(seam.confidence_percent, 55);
    assert_eq!(seam.isolation_score, 45);
    assert!(
        seam.score_components
            .iter()
            .any(|component| component.factor == "static-table-access")
    );
}

#[test]
fn seam_inference_uses_validated_bytecode_references_when_source_is_unavailable() {
    let graph = EvidenceGraph {
        schema_version: "modernlink.evidence/v1alpha1".to_owned(),
        evidence: vec![Evidence {
            id: "evidence:bytecode-jms".to_owned(),
            kind: "bytecode-class-reference".to_owned(),
            value: "javax.jms.Queue".to_owned(),
            source: SourceLocation {
                path: "legacy.war!WEB-INF/classes/QueueAdapter.class".to_owned(),
                start_byte: 0,
                end_byte: 0,
            },
        }],
        nodes: vec![
            GraphNode {
                id: "node:archive".to_owned(),
                kind: "artifact".to_owned(),
                name: "legacy.war".to_owned(),
                evidence_ids: Vec::new(),
            },
            GraphNode {
                id: "node:jms".to_owned(),
                kind: "external-reference".to_owned(),
                name: "javax.jms.Queue".to_owned(),
                evidence_ids: Vec::new(),
            },
        ],
        edges: vec![GraphEdge {
            id: "edge:bytecode-jms".to_owned(),
            kind: "bytecode-references".to_owned(),
            source_id: "node:archive".to_owned(),
            target_id: "node:jms".to_owned(),
            evidence_ids: vec!["evidence:bytecode-jms".to_owned()],
        }],
        claims: Vec::new(),
    };
    let seam = infer_seams(&graph).expect("bytecode seam").seams.remove(0);
    assert_eq!(seam.seam_type, "outbound-bytecode-dependency");
    assert_eq!(seam.current_technology, "jms");
    assert_eq!(seam.confidence_percent, 80);
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

#[test]
fn compatibility_assessment_links_target_reviews_to_bytecode_evidence() {
    let graph = EvidenceGraph {
        schema_version: "modernlink.evidence/v1alpha1".to_owned(),
        evidence: vec![Evidence {
            id: "evidence:bytecode-jaxb".to_owned(),
            kind: "bytecode-class-reference".to_owned(),
            value: "javax.xml.bind.JAXBContext".to_owned(),
            source: SourceLocation {
                path: "legacy.jar!LegacyXml.class".to_owned(),
                start_byte: 0,
                end_byte: 0,
            },
        }],
        nodes: vec![
            GraphNode {
                id: "node:archive".to_owned(),
                kind: "artifact".to_owned(),
                name: "legacy.jar".to_owned(),
                evidence_ids: Vec::new(),
            },
            GraphNode {
                id: "node:jaxb".to_owned(),
                kind: "external-reference".to_owned(),
                name: "javax.xml.bind.JAXBContext".to_owned(),
                evidence_ids: Vec::new(),
            },
        ],
        edges: vec![GraphEdge {
            id: "edge:bytecode-jaxb".to_owned(),
            kind: "bytecode-references".to_owned(),
            source_id: "node:archive".to_owned(),
            target_id: "node:jaxb".to_owned(),
            evidence_ids: vec!["evidence:bytecode-jaxb".to_owned()],
        }],
        claims: Vec::new(),
    };
    let finding = assess_compatibility(&graph, 21)
        .expect("bytecode compatibility")
        .findings
        .remove(0);
    assert_eq!(finding.category, "java-ee-api-review");
    assert_eq!(finding.evidence_ids, vec!["evidence:bytecode-jaxb"]);
}

#[test]
fn migration_plan_makes_seam_work_depend_on_matching_compatibility_review() {
    let graph = EvidenceGraph {
        schema_version: "modernlink.evidence/v1alpha1".to_owned(),
        evidence: vec![Evidence {
            id: "evidence:vendor".to_owned(),
            kind: "import".to_owned(),
            value: "weblogic.jndi.Environment".to_owned(),
            source: SourceLocation {
                path: "Legacy.java".to_owned(),
                start_byte: 0,
                end_byte: 20,
            },
        }],
        nodes: vec![GraphNode {
            id: "node:vendor".to_owned(),
            kind: "external-reference".to_owned(),
            name: "weblogic.jndi.Environment".to_owned(),
            evidence_ids: Vec::new(),
        }],
        edges: vec![GraphEdge {
            id: "edge:vendor".to_owned(),
            kind: "imports".to_owned(),
            source_id: "node:vendor".to_owned(),
            target_id: "node:vendor".to_owned(),
            evidence_ids: vec!["evidence:vendor".to_owned()],
        }],
        claims: Vec::new(),
    };
    let seams = infer_seams(&graph).expect("seams");
    let compatibility = assess_compatibility(&graph, 21).expect("compatibility");
    let plan = plan_migration(&seams, &compatibility);
    let migration = plan
        .tasks
        .iter()
        .find(|task| task.kind == "seam-migration")
        .expect("migration task");
    assert_eq!(migration.state, model::ClaimState::Hypothesis);
    assert!(migration.approval_required);
    assert!(migration.depends_on.len() >= 2);
}
