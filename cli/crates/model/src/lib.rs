use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

pub const EVIDENCE_SCHEMA_VERSION: &str = "modernlink.evidence/v1alpha1";

#[derive(Debug, Error)]
pub enum GraphError {
    #[error("unsupported evidence graph schema: {0}")]
    UnsupportedSchema(String),
    #[error("duplicate {kind} identifier: {id}")]
    DuplicateId { kind: &'static str, id: String },
    #[error("{owner_kind} {owner_id} cites missing evidence: {evidence_id}")]
    MissingEvidence {
        owner_kind: &'static str,
        owner_id: String,
        evidence_id: String,
    },
    #[error("claims cannot use the observed fact state: {0}")]
    InvalidClaimState(String),
    #[error("cannot serialize evidence graph: {0}")]
    Serialization(#[from] serde_json::Error),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceGraph {
    pub schema_version: String,
    #[serde(default)]
    pub evidence: Vec<Evidence>,
    #[serde(default)]
    pub nodes: Vec<GraphNode>,
    #[serde(default)]
    pub claims: Vec<Claim>,
}

impl EvidenceGraph {
    pub fn new() -> Self {
        Self {
            schema_version: EVIDENCE_SCHEMA_VERSION.to_owned(),
            evidence: Vec::new(),
            nodes: Vec::new(),
            claims: Vec::new(),
        }
    }

    pub fn validate(&self) -> Result<(), GraphError> {
        if self.schema_version != EVIDENCE_SCHEMA_VERSION {
            return Err(GraphError::UnsupportedSchema(self.schema_version.clone()));
        }
        let evidence_ids = unique_ids("evidence", self.evidence.iter().map(|item| &item.id))?;
        unique_ids("node", self.nodes.iter().map(|item| &item.id))?;
        unique_ids("claim", self.claims.iter().map(|item| &item.id))?;
        for node in &self.nodes {
            validate_evidence_links("node", &node.id, &node.evidence_ids, &evidence_ids)?;
        }
        for claim in &self.claims {
            if claim.state == ClaimState::Fact {
                return Err(GraphError::InvalidClaimState(claim.id.clone()));
            }
            validate_evidence_links("claim", &claim.id, &claim.evidence_ids, &evidence_ids)?;
        }
        Ok(())
    }

    pub fn canonical_json(&self) -> Result<String, GraphError> {
        self.validate()?;
        let mut normalized = self.clone();
        normalized.normalize();
        let mut json = serde_json::to_string_pretty(&normalized)?;
        json.push('\n');
        Ok(json)
    }

    pub fn from_json(json: &str) -> Result<Self, GraphError> {
        let graph: Self = serde_json::from_str(json)?;
        graph.validate()?;
        Ok(graph)
    }

    fn normalize(&mut self) {
        self.evidence.sort_by(|left, right| left.id.cmp(&right.id));
        self.nodes.sort_by(|left, right| left.id.cmp(&right.id));
        self.claims.sort_by(|left, right| left.id.cmp(&right.id));
        for node in &mut self.nodes {
            node.evidence_ids.sort();
            node.evidence_ids.dedup();
        }
        for claim in &mut self.claims {
            claim.evidence_ids.sort();
            claim.evidence_ids.dedup();
        }
    }
}

impl Default for EvidenceGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceLocation {
    pub path: String,
    pub start_byte: usize,
    pub end_byte: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Evidence {
    pub id: String,
    pub kind: String,
    pub value: String,
    pub source: SourceLocation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub kind: String,
    pub name: String,
    #[serde(default)]
    pub evidence_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ClaimState {
    Fact,
    Inference,
    Hypothesis,
    UserConfirmed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Claim {
    pub id: String,
    pub state: ClaimState,
    pub summary: String,
    #[serde(default)]
    pub evidence_ids: Vec<String>,
}

pub fn stable_id(kind: &str, parts: impl IntoIterator<Item = impl AsRef<str>>) -> String {
    let mut digest = Sha256::new();
    digest.update(kind.as_bytes());
    digest.update([0]);
    for part in parts {
        digest.update(part.as_ref().as_bytes());
        digest.update([0]);
    }
    format!("{kind}:sha256:{}", hex::encode(digest.finalize()))
}

fn unique_ids<'a>(
    kind: &'static str,
    ids: impl Iterator<Item = &'a String>,
) -> Result<BTreeSet<String>, GraphError> {
    let mut unique = BTreeSet::new();
    for id in ids {
        if !unique.insert(id.clone()) {
            return Err(GraphError::DuplicateId {
                kind,
                id: id.clone(),
            });
        }
    }
    Ok(unique)
}

fn validate_evidence_links(
    owner_kind: &'static str,
    owner_id: &str,
    evidence_ids: &[String],
    known_evidence: &BTreeSet<String>,
) -> Result<(), GraphError> {
    for evidence_id in evidence_ids {
        if !known_evidence.contains(evidence_id) {
            return Err(GraphError::MissingEvidence {
                owner_kind,
                owner_id: owner_id.to_owned(),
                evidence_id: evidence_id.clone(),
            });
        }
    }
    Ok(())
}
