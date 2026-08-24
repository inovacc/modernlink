use std::collections::BTreeMap;

use model::{ClaimState, EvidenceGraph, GraphError};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const STRUCTURE_SCHEMA_VERSION: &str = "modernlink.structure/v1alpha1";
pub const COMPATIBILITY_SCHEMA_VERSION: &str = "modernlink.compatibility/v1alpha1";

#[derive(Debug, Error)]
pub enum InferenceError {
    #[error("cannot use invalid evidence graph: {0}")]
    Graph(#[from] GraphError),
    #[error("cannot serialize structural inference: {0}")]
    Serialization(#[from] serde_json::Error),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructureReport {
    pub schema_version: String,
    pub layers: Vec<LayerInference>,
    pub contexts: Vec<CandidateContext>,
}

impl StructureReport {
    pub fn canonical_json(&self) -> Result<String, InferenceError> {
        let mut normalized = self.clone();
        normalized
            .layers
            .sort_by(|left, right| left.node_id.cmp(&right.node_id));
        normalized
            .contexts
            .sort_by(|left, right| left.name.cmp(&right.name));
        let mut json = serde_json::to_string_pretty(&normalized)?;
        json.push('\n');
        Ok(json)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayerInference {
    pub node_id: String,
    pub layer: String,
    pub state: ClaimState,
    pub confidence_percent: u8,
    pub evidence_ids: Vec<String>,
    pub reasoning: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateContext {
    pub name: String,
    pub state: ClaimState,
    pub confidence_percent: u8,
    pub evidence_ids: Vec<String>,
    pub reasoning: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeamReport {
    pub schema_version: String,
    pub seams: Vec<ModernizationSeam>,
}

impl SeamReport {
    pub fn canonical_json(&self) -> Result<String, InferenceError> {
        let mut normalized = self.clone();
        normalized
            .seams
            .sort_by(|left, right| left.id.cmp(&right.id));
        let mut json = serde_json::to_string_pretty(&normalized)?;
        json.push('\n');
        Ok(json)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModernizationSeam {
    pub id: String,
    pub state: ClaimState,
    pub location_node_id: String,
    pub seam_type: String,
    pub current_technology: String,
    pub leverage_score: u8,
    pub migration_risk_score: u8,
    pub isolation_score: u8,
    pub confidence_percent: u8,
    pub score_components: Vec<ScoreComponent>,
    pub evidence_ids: Vec<String>,
    pub recommended_mode: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScoreComponent {
    pub factor: String,
    pub points: u8,
    pub reasoning: String,
}

/// A target-specific review item.  It deliberately reports observed evidence and
/// does not claim a migration outcome or a synthetic readiness percentage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompatibilityFinding {
    pub id: String,
    pub state: ClaimState,
    pub category: String,
    pub severity: String,
    pub summary: String,
    pub evidence_ids: Vec<String>,
    pub target_version: u16,
    pub recommendation: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompatibilityReport {
    pub schema_version: String,
    pub target_version: u16,
    pub findings: Vec<CompatibilityFinding>,
    pub limitations: Vec<String>,
}

impl CompatibilityReport {
    pub fn canonical_json(&self) -> Result<String, InferenceError> {
        let mut normalized = self.clone();
        normalized
            .findings
            .sort_by(|left, right| left.id.cmp(&right.id));
        normalized.limitations.sort();
        let mut json = serde_json::to_string_pretty(&normalized)?;
        json.push('\n');
        Ok(json)
    }
}

pub fn infer_structure(graph: &EvidenceGraph) -> Result<StructureReport, InferenceError> {
    graph.validate()?;
    let mut layers = Vec::new();
    let mut contexts = BTreeMap::<String, Vec<String>>::new();
    for node in &graph.nodes {
        if node.kind != "type" {
            continue;
        }
        if let Some(layer) = layer_for(&node.name) {
            layers.push(LayerInference {
                node_id: node.id.clone(),
                layer: layer.to_owned(),
                state: ClaimState::Inference,
                confidence_percent: 70,
                evidence_ids: node.evidence_ids.clone(),
                reasoning: "inferred from the declared type name".to_owned(),
            });
        }
        if let Some(context) = context_for(&node.name) {
            contexts
                .entry(context.to_owned())
                .or_default()
                .extend(node.evidence_ids.clone());
        }
    }
    let contexts = contexts
        .into_iter()
        .map(|(name, mut evidence_ids)| {
            evidence_ids.sort();
            evidence_ids.dedup();
            CandidateContext {
                name,
                state: ClaimState::Hypothesis,
                confidence_percent: 55,
                evidence_ids,
                reasoning:
                    "candidate context inferred from package cohesion; human confirmation required"
                        .to_owned(),
            }
        })
        .collect();
    Ok(StructureReport {
        schema_version: STRUCTURE_SCHEMA_VERSION.to_owned(),
        layers,
        contexts,
    })
}

pub fn infer_seams(graph: &EvidenceGraph) -> Result<SeamReport, InferenceError> {
    graph.validate()?;
    let nodes = graph
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node))
        .collect::<BTreeMap<_, _>>();
    let mut seams = Vec::new();
    for edge in &graph.edges {
        if edge.kind != "imports" {
            continue;
        }
        let Some(target) = nodes.get(edge.target_id.as_str()) else {
            continue;
        };
        if target.kind != "external-reference" {
            continue;
        }
        let technology = vendor_technology(&target.name).unwrap_or("external-api");
        let mut score_components = vec![ScoreComponent {
            factor: "external-boundary".to_owned(),
            points: 25,
            reasoning: "an import crosses from an owned node to an external reference".to_owned(),
        }];
        if technology != "external-api" {
            score_components.push(ScoreComponent {
                factor: "vendor-lock".to_owned(),
                points: 45,
                reasoning: format!("target name identifies {technology} vendor coupling"),
            });
        }
        let leverage_score = score_components.iter().map(|item| item.points).sum::<u8>();
        seams.push(ModernizationSeam {
            id: model::stable_id("modernization-seam", [edge.id.as_str()]),
            state: ClaimState::Inference,
            location_node_id: edge.source_id.clone(),
            seam_type: "outbound-import-dependency".to_owned(),
            current_technology: technology.to_owned(),
            leverage_score,
            migration_risk_score: if technology == "external-api" { 40 } else { 60 },
            isolation_score: 70,
            confidence_percent: if technology == "external-api" { 60 } else { 80 },
            score_components,
            evidence_ids: edge.evidence_ids.clone(),
            recommended_mode: "PASSTHROUGH -> SHADOW -> REDIRECT".to_owned(),
        });
    }
    Ok(SeamReport {
        schema_version: "modernlink.seams/v1alpha1".to_owned(),
        seams,
    })
}

/// Produces an evidence-linked target review from imports already present in the
/// graph. It is intentionally conservative: a finding asks for a review rather
/// than asserting that a dependency or source edit is sufficient.
pub fn assess_compatibility(
    graph: &EvidenceGraph,
    target_version: u16,
) -> Result<CompatibilityReport, InferenceError> {
    graph.validate()?;
    let mut findings = Vec::new();
    for node in &graph.nodes {
        if node.kind != "external-reference" {
            continue;
        }
        let Some((category, severity, summary, recommendation)) =
            compatibility_rule(&node.name, target_version)
        else {
            continue;
        };
        let mut evidence_ids = node.evidence_ids.clone();
        evidence_ids.extend(
            graph
                .edges
                .iter()
                .filter(|edge| edge.target_id == node.id && edge.kind == "imports")
                .flat_map(|edge| edge.evidence_ids.clone()),
        );
        evidence_ids.sort();
        evidence_ids.dedup();
        findings.push(CompatibilityFinding {
            id: model::stable_id(
                "compatibility-finding",
                [target_version.to_string().as_str(), node.id.as_str()],
            ),
            state: ClaimState::Inference,
            category: category.to_owned(),
            severity: severity.to_owned(),
            summary: summary.to_owned(),
            evidence_ids,
            target_version,
            recommendation: recommendation.to_owned(),
        });
    }
    Ok(CompatibilityReport {
        schema_version: COMPATIBILITY_SCHEMA_VERSION.to_owned(),
        target_version,
        findings,
        limitations: vec![
            "This report uses import evidence only; it does not yet inspect resolved dependency versions, bytecode, runtime configuration, or deployment behavior.".to_owned(),
            "A finding is an evidence-backed review request, not a claim that a migration will fail or that one change is sufficient.".to_owned(),
        ],
    })
}

fn layer_for(name: &str) -> Option<&'static str> {
    let lower = name.to_ascii_lowercase();
    if lower.ends_with("controller") || lower.ends_with("resource") {
        Some("presentation")
    } else if lower.ends_with("service") || lower.ends_with("usecase") {
        Some("service")
    } else if lower.ends_with("repository") || lower.ends_with("dao") {
        Some("persistence")
    } else if lower.ends_with("client") || lower.ends_with("gateway") || lower.ends_with("adapter")
    {
        Some("integration")
    } else {
        None
    }
}

fn context_for(name: &str) -> Option<&str> {
    let segments = name.split('.').collect::<Vec<_>>();
    (segments.len() >= 2).then(|| segments[segments.len() - 2])
}

fn vendor_technology(name: &str) -> Option<&'static str> {
    if name.starts_with("weblogic.") {
        Some("weblogic")
    } else if name.starts_with("org.jboss.") {
        Some("jboss")
    } else if name.starts_with("com.ibm.websphere.") || name.starts_with("com.ibm.ws.") {
        Some("websphere")
    } else {
        None
    }
}

fn compatibility_rule(
    name: &str,
    target_version: u16,
) -> Option<(&'static str, &'static str, &'static str, &'static str)> {
    if name.starts_with("sun.") || name.starts_with("com.sun.") {
        Some((
            "internal-jdk-api",
            "HIGH",
            "Import targets a non-standard JDK namespace.",
            "Establish the supported replacement and characterize behavior before selecting a target-runtime migration step.",
        ))
    } else if name.starts_with("weblogic.")
        || name.starts_with("org.jboss.")
        || name.starts_with("com.ibm.websphere.")
        || name.starts_with("com.ibm.ws.")
    {
        Some((
            "application-server-api",
            "HIGH",
            "Import targets an application-server-specific API.",
            "Isolate the vendor boundary and assess whether ModernLink can bridge it before changing the target runtime.",
        ))
    } else if target_version >= 11
        && (name.starts_with("javax.xml.bind.") || name.starts_with("javax.xml.ws."))
    {
        Some((
            "java-ee-api-review",
            "HIGH",
            "Import targets a legacy Java EE namespace that requires explicit target-runtime review.",
            "Inventory the deployment-provided API and dependency path, then choose a compatible migration boundary with characterization tests.",
        ))
    } else if name.starts_with("javax.") {
        Some((
            "java-ee-api-review",
            "MEDIUM",
            "Import targets a Java EE namespace whose runtime provider must be established for the selected target.",
            "Record the provider and deployment contract before changing runtime or namespace assumptions.",
        ))
    } else {
        None
    }
}
