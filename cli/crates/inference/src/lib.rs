use std::collections::BTreeMap;

use model::{ClaimState, EvidenceGraph, GraphError};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const STRUCTURE_SCHEMA_VERSION: &str = "modernlink.structure/v1alpha1";

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
