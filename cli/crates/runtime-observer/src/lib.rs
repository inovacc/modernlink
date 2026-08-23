mod generic_http;
mod http;
mod model;
mod profile;
mod redaction;

pub use generic_http::GenericHttpConnector;
pub use http::{HttpResponse, HttpTransport, ReqwestHttpTransport};
pub use model::{
    Capability, CapabilityReport, ObservationDepth, ObservationSnapshot, RuntimeConnector,
    RuntimeEvidence, RuntimeObservation,
};
pub use profile::{Authorization, ConnectorKind, CredentialRef, RuntimeProfile, Scope, TlsPolicy};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ObservationError {
    #[error("invalid runtime profile: {0}")]
    InvalidProfile(String),
    #[error("HTTP transport failed: {0}")]
    Transport(String),
    #[error("HTTP response rejected: {0}")]
    Response(String),
    #[error("cannot serialize runtime observation: {0}")]
    Serialize(#[from] serde_json::Error),
}
