use crate::model::stable_id;
use crate::{
    Capability, CapabilityReport, ConnectorKind, HttpTransport, ObservationDepth, ObservationError,
    ObservationSnapshot, RuntimeConnector, RuntimeEvidence, RuntimeObservation, RuntimeProfile,
    redaction::redact_projection,
};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
pub struct GenericHttpConnector<T> {
    transport: T,
}
impl<T> GenericHttpConnector<T> {
    pub fn new(transport: T) -> Self {
        Self { transport }
    }
}
impl<T: HttpTransport> GenericHttpConnector<T> {
    fn fetch_projection(
        &self,
        profile: &RuntimeProfile,
    ) -> Result<(serde_json::Value, BTreeMap<String, usize>), ObservationError> {
        profile.validate()?;
        if profile.kind != ConnectorKind::GenericHttp {
            return Err(ObservationError::InvalidProfile(
                "generic HTTP connector requires kind generic-http".to_owned(),
            ));
        }
        let response = self.transport.get(&profile.endpoint)?;
        if !(200..300).contains(&response.status) {
            return Err(ObservationError::Response(format!(
                "GET returned status {}",
                response.status
            )));
        }
        if !response
            .content_type
            .as_deref()
            .is_some_and(|content_type| content_type.starts_with("application/json"))
        {
            return Err(ObservationError::Response(
                "GET response must declare application/json".to_owned(),
            ));
        }
        let value: serde_json::Value = serde_json::from_slice(&response.body).map_err(|error| {
            ObservationError::Response(format!("GET response is not JSON: {error}"))
        })?;
        if !value.is_object() {
            return Err(ObservationError::Response(
                "GET response must be a JSON object".to_owned(),
            ));
        }
        let redacted = redact_projection(value);
        Ok((redacted.value, redacted.counters))
    }
}
impl<T: HttpTransport> RuntimeConnector for GenericHttpConnector<T> {
    fn probe(&self, profile: &RuntimeProfile) -> Result<CapabilityReport, ObservationError> {
        self.fetch_projection(profile)?;
        Ok(CapabilityReport {
            profile_digest: profile.digest()?,
            connector: "generic-http".to_owned(),
            capabilities: vec![Capability::supported("http-json-metadata")],
        })
    }
    fn observe_metadata(
        &self,
        profile: &RuntimeProfile,
        depth: ObservationDepth,
        captured_at: &str,
    ) -> Result<ObservationSnapshot, ObservationError> {
        if depth != ObservationDepth::Metadata {
            return Err(ObservationError::InvalidProfile(
                "generic HTTP connector supports metadata depth only".to_owned(),
            ));
        }
        let (payload, redaction_counters) = self.fetch_projection(profile)?;
        let payload_digest = format!(
            "sha256:{}",
            hex::encode(Sha256::digest(serde_json::to_vec(&payload)?))
        );
        let profile_digest = profile.digest()?;
        let evidence = RuntimeEvidence {
            id: stable_id(
                "runtime-evidence",
                vec![
                    profile_digest,
                    "generic-http-metadata".to_owned(),
                    profile.endpoint.clone(),
                    "GET".to_owned(),
                    payload_digest.clone(),
                ],
            ),
            collector: "generic-http".to_owned(),
            target: profile.endpoint.clone(),
            resource_key: profile.endpoint.clone(),
            source_operation: "GET".to_owned(),
            payload_digest,
            payload,
        };
        ObservationSnapshot::from_observation(
            profile,
            "generic-http",
            captured_at,
            depth,
            RuntimeObservation {
                capabilities: vec![Capability::supported("http-json-metadata")],
                evidence: vec![evidence],
                redaction_counters,
            },
        )
    }
}
