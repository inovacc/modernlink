use crate::{ObservationError, redaction::is_sensitive_query_key};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{net::IpAddr, str::FromStr};

pub const PROFILE_SCHEMA: &str = "modernlink.runtime-profile/v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ConnectorKind {
    GenericHttp,
    Kubernetes,
    Ssh,
    JbossHttp,
    WeblogicHttp,
    GlassfishHttp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum CredentialRef {
    Environment { variable: String },
    OsKeyring { service: String, account: String },
    ExternalHelper { command: String },
    MaskedPrompt,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scope {
    pub environment: Option<String>,
    pub namespace: Option<String>,
    pub server_group: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Authorization {
    pub owner: String,
    pub reference: String,
    pub expires_at: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TlsPolicy {
    pub mode: String,
    pub ca_ref: Option<String>,
    pub pinned_sha256: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeProfile {
    pub schema_version: String,
    pub name: String,
    pub kind: ConnectorKind,
    pub endpoint: String,
    #[serde(default)]
    pub scope: Option<Scope>,
    #[serde(default)]
    pub authorization: Option<Authorization>,
    #[serde(default)]
    pub credential_ref: Option<CredentialRef>,
    #[serde(default)]
    pub tls: Option<TlsPolicy>,
    #[serde(default)]
    pub tunnel_bind: Option<String>,
    #[serde(default = "get_method")]
    pub http_method: String,
}
fn get_method() -> String {
    "GET".to_owned()
}

impl RuntimeProfile {
    pub fn from_json(json: &str) -> Result<Self, ObservationError> {
        let value: serde_json::Value = serde_json::from_str(json)
            .map_err(|error| ObservationError::InvalidProfile(error.to_string()))?;
        reject_inline_credentials(&value)?;
        let profile: Self = serde_json::from_value(value)
            .map_err(|error| ObservationError::InvalidProfile(error.to_string()))?;
        profile.validate()?;
        Ok(profile)
    }
    pub fn validate(&self) -> Result<(), ObservationError> {
        if self.schema_version != PROFILE_SCHEMA {
            return Err(ObservationError::InvalidProfile(format!(
                "schema_version must be {PROFILE_SCHEMA}"
            )));
        }
        if self.name.trim().is_empty() || self.endpoint.trim().is_empty() {
            return Err(ObservationError::InvalidProfile(
                "name and endpoint must be non-empty".to_owned(),
            ));
        }
        let endpoint = reqwest::Url::parse(&self.endpoint).map_err(|_| {
            ObservationError::InvalidProfile("endpoint must be an absolute HTTP(S) URL".to_owned())
        })?;
        if !matches!(endpoint.scheme(), "http" | "https") {
            return Err(ObservationError::InvalidProfile(
                "endpoint must use HTTP or HTTPS".to_owned(),
            ));
        }
        if !endpoint.username().is_empty() || endpoint.password().is_some() {
            return Err(ObservationError::InvalidProfile(
                "endpoint must not contain inline credentials".to_owned(),
            ));
        }
        if endpoint
            .query_pairs()
            .any(|(key, _)| is_sensitive_query_key(&key))
        {
            return Err(ObservationError::InvalidProfile(
                "endpoint query must not contain credential material".to_owned(),
            ));
        }
        let method = self.http_method.to_ascii_uppercase();
        if method != "GET" {
            return Err(ObservationError::InvalidProfile(format!(
                "HTTP method {method} is not permitted; only GET is allowed"
            )));
        }
        if let Some(bind) = &self.tunnel_bind {
            let address = IpAddr::from_str(bind).map_err(|_| {
                ObservationError::InvalidProfile(
                    "tunnel_bind must be a loopback IP address".to_owned(),
                )
            })?;
            if !address.is_loopback() {
                return Err(ObservationError::InvalidProfile(
                    "tunnel_bind must be loopback-only".to_owned(),
                ));
            }
        }
        Ok(())
    }
    pub fn digest(&self) -> Result<String, ObservationError> {
        Ok(format!(
            "sha256:{}",
            hex::encode(Sha256::digest(serde_json::to_vec(self)?))
        ))
    }
}

fn reject_inline_credentials(value: &serde_json::Value) -> Result<(), ObservationError> {
    match value {
        serde_json::Value::Object(object) => {
            for (key, value) in object {
                let key = key.to_ascii_lowercase();
                if matches!(
                    key.as_str(),
                    "password" | "token" | "private_key" | "credential" | "credentials"
                ) || (key == "kind" && value == "inline")
                {
                    return Err(ObservationError::InvalidProfile(
                        "inline credentials are forbidden; use credential_ref".to_owned(),
                    ));
                }
                reject_inline_credentials(value)?;
            }
        }
        serde_json::Value::Array(values) => {
            for value in values {
                reject_inline_credentials(value)?;
            }
        }
        _ => {}
    }
    Ok(())
}
