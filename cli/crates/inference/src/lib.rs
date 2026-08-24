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
