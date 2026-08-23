use crate::ObservationError;
use std::time::Duration;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpResponse {
    pub status: u16,
    pub content_type: Option<String>,
    pub body: Vec<u8>,
}
pub trait HttpTransport {
    fn get(&self, endpoint: &str) -> Result<HttpResponse, ObservationError>;
}
pub struct ReqwestHttpTransport {
    client: reqwest::blocking::Client,
}
impl ReqwestHttpTransport {
    pub fn new() -> Result<Self, ObservationError> {
        let client = reqwest::blocking::Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(15))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|error| ObservationError::Transport(error.to_string()))?;
        Ok(Self { client })
    }
}
impl HttpTransport for ReqwestHttpTransport {
    fn get(&self, endpoint: &str) -> Result<HttpResponse, ObservationError> {
        let response = self
            .client
            .get(endpoint)
            .send()
            .map_err(|error| ObservationError::Transport(error.to_string()))?;
        let status = response.status().as_u16();
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        let body = response
            .bytes()
            .map_err(|error| ObservationError::Transport(error.to_string()))?
            .to_vec();
        if body.len() > 1_048_576 {
            return Err(ObservationError::Response(
                "response exceeds 1 MiB limit".to_owned(),
            ));
        }
        Ok(HttpResponse {
            status,
            content_type,
            body,
        })
    }
}
