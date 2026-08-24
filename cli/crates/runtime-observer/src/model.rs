use crate::{
    ObservationError, RuntimeProfile,
    redaction::{redact_projection, redact_reference},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, str::FromStr};
pub const OBSERVATION_SCHEMA: &str = "modernlink.runtime-observation/v1alpha1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ObservationDepth {
    Metadata,
    Deep,
}
impl FromStr for ObservationDepth {
    type Err = ObservationError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "metadata" => Ok(Self::Metadata),
            "deep" => Ok(Self::Deep),
            _ => Err(ObservationError::InvalidProfile(format!(
                "unknown observation depth: {value}"
            ))),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Capability {
    pub id: String,
    pub supported: bool,
    pub detail: Option<String>,
}
impl Capability {
    pub fn supported(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            supported: true,
            detail: None,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityReport {
    pub profile_digest: String,
    pub connector: String,
    pub capabilities: Vec<Capability>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeEvidence {
    pub id: String,
    pub collector: String,
    pub target: String,
    pub resource_key: String,
    pub source_operation: String,
    pub payload_digest: String,
    pub payload: serde_json::Value,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeObservation {
    pub capabilities: Vec<Capability>,
    pub evidence: Vec<RuntimeEvidence>,
    pub redaction_counters: BTreeMap<String, usize>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObservationSnapshot {
    pub schema_version: String,
    pub profile_digest: String,
    pub connector: String,
    pub captured_at: String,
    pub depth: ObservationDepth,
    pub capabilities: Vec<Capability>,
    pub evidence: Vec<RuntimeEvidence>,
    pub redaction_counters: BTreeMap<String, usize>,
    pub content_digest: String,
}
pub trait RuntimeConnector {
    fn probe(&self, profile: &RuntimeProfile) -> Result<CapabilityReport, ObservationError>;
    fn observe_metadata(
        &self,
        profile: &RuntimeProfile,
        depth: ObservationDepth,
        captured_at: &str,
    ) -> Result<ObservationSnapshot, ObservationError>;
}
impl ObservationSnapshot {
    pub fn from_observation(
        profile: &RuntimeProfile,
        connector: impl Into<String>,
        captured_at: impl Into<String>,
        depth: ObservationDepth,
        mut observation: RuntimeObservation,
    ) -> Result<Self, ObservationError> {
        profile.validate()?;
        observation.capabilities.sort_by(|a, b| a.id.cmp(&b.id));
        let profile_digest = profile.digest()?;
        let connector = connector.into();
        let mut redaction_counters = observation.redaction_counters;
        for evidence in &mut observation.evidence {
            let redacted = redact_projection(std::mem::take(&mut evidence.payload));
            for (rule, count) in redacted.counters {
                *redaction_counters.entry(rule).or_default() += count;
            }
            evidence.payload = redacted.value;
            evidence.target = redact_reference(&evidence.target, &mut redaction_counters);
            evidence.resource_key =
                redact_reference(&evidence.resource_key, &mut redaction_counters);
            evidence.payload_digest = format!(
                "sha256:{}",
                hex::encode(Sha256::digest(serde_json::to_vec(&evidence.payload)?))
            );
            evidence.id = stable_id(
                "runtime-evidence",
                vec![
                    profile_digest.clone(),
                    evidence.collector.clone(),
                    evidence.resource_key.clone(),
                    evidence.source_operation.clone(),
                    evidence.payload_digest.clone(),
                ],
            );
        }
        observation.evidence.sort_by(|a, b| a.id.cmp(&b.id));
        let digestable = serde_json::json!({"schema_version": OBSERVATION_SCHEMA, "profile_digest": profile_digest, "connector": connector, "depth": depth, "capabilities": observation.capabilities, "evidence": observation.evidence, "redaction_counters": redaction_counters});
        let content_digest = format!(
            "sha256:{}",
            hex::encode(Sha256::digest(serde_json::to_vec(&digestable)?))
        );
        Ok(Self {
            schema_version: OBSERVATION_SCHEMA.to_owned(),
            profile_digest,
            connector,
            captured_at: captured_at.into(),
            depth,
            capabilities: observation.capabilities,
            evidence: observation.evidence,
            redaction_counters,
            content_digest,
        })
    }
    pub fn canonical_json(&self) -> Result<String, ObservationError> {
        let mut json = serde_json::to_string_pretty(self)?;
        json.push('\n');
        Ok(json)
    }
}
pub(crate) fn stable_id(prefix: &str, fields: impl IntoIterator<Item = String>) -> String {
    let mut hash = Sha256::new();
    for field in fields {
        hash.update((field.len() as u64).to_be_bytes());
        hash.update(field.as_bytes());
    }
    format!("{prefix}:sha256:{}", hex::encode(hash.finalize()))
}
