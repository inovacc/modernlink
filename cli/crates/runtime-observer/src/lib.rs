mod command;
mod generic_http;
mod http;
mod kubernetes;
mod model;
mod profile;
mod redaction;
mod ssh;
mod tunnel;

pub use command::{
    CommandExecutor, CommandOutput, CredentialResolver, CredentialValue, PromptProvider,
    SystemCommandExecutor,
};
pub use generic_http::GenericHttpConnector;
pub use http::{HttpResponse, HttpTransport, ReqwestHttpTransport};
pub use kubernetes::{KubernetesConnector, kubernetes_port_forward_args};
pub use model::{
    Capability, CapabilityReport, ObservationDepth, ObservationSnapshot, RuntimeConnector,
    RuntimeEvidence, RuntimeObservation,
};
pub use profile::{Authorization, ConnectorKind, CredentialRef, RuntimeProfile, Scope, TlsPolicy};
pub use ssh::{SSH_METADATA_SCRIPT, SshConnector, ssh_local_forward_args};
pub use tunnel::{TunnelApproval, TunnelGuard};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ObservationError {
    #[error("invalid runtime profile: {0}")]
    InvalidProfile(String),
    #[error("HTTP transport failed: {0}")]
    Transport(String),
    #[error("HTTP response rejected: {0}")]
    Response(String),
    #[error("approved command failed: {0}")]
    Command(String),
    #[error("credential resolution failed: {0}")]
    Credential(String),
    #[error("tunnel lifecycle failed: {0}")]
    Tunnel(String),
    #[error("cannot serialize runtime observation: {0}")]
    Serialize(#[from] serde_json::Error),
}
