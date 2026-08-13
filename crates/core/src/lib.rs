use std::collections::BTreeMap;
use std::fmt;
use std::time::Duration;
use base64::Engine;

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
            return Err(Error::InvalidRequest("only https:// URLs are supported".to_string()));
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

pub fn base64_encode(value: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(value)
}

pub fn request_json(request: &Request) -> String {
    serde_json::json!({
        "url": request.url,
        "method": request.method,
        "headers": request.headers,
        "bodyBase64": base64_encode(&request.body),
        "followRedirects": request.follow_redirects,
        "maxRedirects": request.max_redirects,
        "minimumTlsVersion": match request.minimum_tls_version {
            TlsVersion::Tls12 => "TLSv1.2",
            TlsVersion::Tls13 => "TLSv1.3",
        },
    }).to_string()
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
    use super::{base64_encode, request_json, uuid_v7, Request};

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
    fn base64_and_json_helpers_encode_request_data() {
        assert_eq!(base64_encode(b"modernlink"), "bW9kZXJubGluaw==");
        let mut request = Request::new("https://example.com").unwrap();
        request.body = b"payload".to_vec();
        let json = request_json(&request);
        assert!(json.contains("bodyBase64"));
        assert!(json.contains("cGF5bG9hZA=="));
    }
}
