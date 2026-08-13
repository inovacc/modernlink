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
        })
    }
}

#[derive(Debug, Clone)]
pub struct Response {
    pub final_url: String,
    pub status: u16,
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
    use super::Request;

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
    }
}
