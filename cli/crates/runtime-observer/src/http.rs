use crate::ObservationError;
use std::{io::Read, time::Duration};
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
        let mut response = self
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
        let body = read_limited(&mut response)?;
        Ok(HttpResponse {
            status,
            content_type,
            body,
        })
    }
}

fn read_limited(reader: &mut impl Read) -> Result<Vec<u8>, ObservationError> {
    const MAX_RESPONSE_BYTES: usize = 1_048_576;
    let mut body = Vec::with_capacity(MAX_RESPONSE_BYTES.min(16_384));
    reader
        .take((MAX_RESPONSE_BYTES + 1) as u64)
        .read_to_end(&mut body)
        .map_err(|error| ObservationError::Transport(error.to_string()))?;
    if body.len() > MAX_RESPONSE_BYTES {
        return Err(ObservationError::Response(
            "response exceeds 1 MiB limit".to_owned(),
        ));
    }
    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::read_limited;
    use std::io::Read;

    struct CountingReader {
        remaining: usize,
        bytes_read: usize,
    }

    impl Read for CountingReader {
        fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
            let count = self.remaining.min(buffer.len());
            buffer[..count].fill(b'x');
            self.remaining -= count;
            self.bytes_read += count;
            Ok(count)
        }
    }

    #[test]
    fn read_limit_stops_after_limit_plus_one_byte() {
        let mut reader = CountingReader {
            remaining: 2_000_000,
            bytes_read: 0,
        };
        assert!(read_limited(&mut reader).is_err());
        assert_eq!(reader.bytes_read, 1_048_577);
    }
}
