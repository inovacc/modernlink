//! Shared request, response, TLS metadata, and error types for ModernLink.
//!
//! Every other crate in the workspace depends on this one; it holds no I/O. Note that the
//! package is named `core`, which shadows Rust's built-in `core` crate — see
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
    if fields.len() % 2 != 0 {
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
mod tests {
    use super::{base64_decode, base64_encode, json_decode, uuid_v4, uuid_v7, Request};

    #[test]
    fn request_rejects_empty_url() {
        let error = Request::new("");
        assert!(error.is_err());
    }

    #[test]
    fn request_accepts_https_url_with_get_as_default() {
        let request = Request::new("https://example.com").unwrap();
        assert_eq!(request.method, "GET");
        assert!(request.follow_redirects);
        assert_eq!(request.max_redirects, 10);
        assert_eq!(request.minimum_tls_version, super::TlsVersion::Tls12);
    }

    #[test]
    fn uuid_v7_has_uuid_shape() {
        let value = uuid_v7();
        assert_eq!(value.len(), 36);
        assert_eq!(&value[14..15], "7");
    }

    #[test]
    fn uuid_v4_has_uuid_shape() {
        let value = uuid_v4();
        assert_eq!(value.len(), 36);
        assert_eq!(&value[14..15], "4");
    }

    #[test]
    fn base64_and_json_helpers_encode_data() {
        assert_eq!(base64_encode(b"modernlink"), "bW9kZXJubGluaw==");
        assert_eq!(base64_decode("bW9kZXJubGluaw==").unwrap(), b"modernlink");
        assert_eq!(
            json_decode("{ \"message\": \"hello\" }").unwrap(),
            "{\"message\":\"hello\"}"
        );
    }
}
