//! Shared request, response, TLS metadata, and error types for ModernLink.
//!
//! Every other crate in the workspace depends on this one; it holds no I/O. The package is
//! `modernlink-core` while the folder stays `crates/core`: a bare `core` shadowed Rust's
//! built-in crate, so `use core::{...}` in a dependent read as the standard library while
//! resolving to this one. Dependents now write `use modernlink_core::{...}` — see
//! `docs/ISSUES.md` I-002.

use base64::Engine;
use std::collections::BTreeMap;
use std::fmt;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    InvalidRequest(String),
    Transport(String),
    Tls(String),
    Native(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TlsVersion {
    Tls12,
    Tls13,
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidRequest(message)
            | Error::Transport(message)
            | Error::Tls(message)
            | Error::Native(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for Error {}

#[derive(Debug, Clone)]
pub struct Request {
    pub url: String,
    pub method: String,
    pub headers: BTreeMap<String, String>,
    pub body: Vec<u8>,
    pub connect_timeout: Option<Duration>,
    pub read_timeout: Option<Duration>,
    pub follow_redirects: bool,
    pub max_redirects: u32,
    pub minimum_tls_version: TlsVersion,
}

impl Request {
    pub fn new(url: &str) -> Result<Self, Error> {
        if url.trim().is_empty() {
            return Err(Error::InvalidRequest("URL must not be empty".to_string()));
        }
        if !url.starts_with("https://") {
            return Err(Error::InvalidRequest(
                "only https:// URLs are supported".to_string(),
            ));
        }
        Ok(Self {
            url: url.to_string(),
            method: "GET".to_string(),
            headers: BTreeMap::new(),
            body: Vec::new(),
            connect_timeout: None,
            read_timeout: None,
            follow_redirects: true,
            max_redirects: 10,
            minimum_tls_version: TlsVersion::Tls12,
        })
    }
}

pub fn uuid_v7() -> String {
    uuid::Uuid::now_v7().to_string()
}

pub fn uuid_v4() -> String {
    uuid::Uuid::new_v4().to_string()
}

pub fn base64_encode(value: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(value)
}

pub fn base64_decode(value: &str) -> Result<Vec<u8>, Error> {
    base64::engine::general_purpose::STANDARD
        .decode(value)
        .map_err(|error| Error::InvalidRequest(error.to_string()))
}

pub fn json_object(fields: &[String]) -> Result<String, Error> {
    if !fields.len().is_multiple_of(2) {
        return Err(Error::InvalidRequest(
            "JSON object fields must be name/value pairs".to_string(),
        ));
    }
    let mut object = serde_json::Map::new();
    for pair in fields.chunks(2) {
        object.insert(pair[0].clone(), serde_json::Value::String(pair[1].clone()));
    }
    serde_json::to_string(&serde_json::Value::Object(object))
        .map_err(|error| Error::Native(error.to_string()))
}

pub fn json_array(values: &[String]) -> Result<String, Error> {
    serde_json::to_string(
        &values
            .iter()
            .map(|value| serde_json::Value::String(value.clone()))
            .collect::<Vec<_>>(),
    )
    .map_err(|error| Error::Native(error.to_string()))
}

pub fn json_decode(value: &str) -> Result<String, Error> {
    let parsed: serde_json::Value =
        serde_json::from_str(value).map_err(|error| Error::InvalidRequest(error.to_string()))?;
    serde_json::to_string(&parsed).map_err(|error| Error::Native(error.to_string()))
}

#[derive(Debug, Clone)]
pub struct Response {
    pub final_url: String,
    pub status: u16,
    pub status_message: String,
    pub headers: BTreeMap<String, String>,
    pub body: Vec<u8>,
    pub tls: Option<TlsInfo>,
}

#[derive(Debug, Clone)]
pub struct TlsInfo {
    pub protocol: Option<String>,
    pub cipher_suite: Option<String>,
    pub peer_certificates_der: Vec<Vec<u8>>,
}

#[cfg(test)]
mod tests;
