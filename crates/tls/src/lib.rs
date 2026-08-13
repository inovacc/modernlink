use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TlsVersion {
    Tls12,
    Tls13,
}

#[derive(Debug, Clone, Copy)]
pub struct TlsConfig {
    minimum_version: TlsVersion,
}

impl TlsConfig {
    pub fn secure_default() -> Self {
        Self { minimum_version: TlsVersion::Tls12 }
    }

    pub fn minimum_version(&self) -> TlsVersion {
        self.minimum_version
    }
}

pub fn build_client(
    config: TlsConfig,
    connect_timeout: Option<Duration>,
    read_timeout: Option<Duration>,
) -> Result<reqwest::blocking::Client, reqwest::Error> {
    let mut builder = reqwest::blocking::Client::builder()
        .min_tls_version(match config.minimum_version {
            TlsVersion::Tls12 => reqwest::tls::Version::TLS_1_2,
            TlsVersion::Tls13 => reqwest::tls::Version::TLS_1_3,
        });
    if let Some(timeout) = connect_timeout {
        builder = builder.connect_timeout(timeout);
    }
    if let Some(timeout) = read_timeout {
        builder = builder.timeout(timeout);
    }
    builder.build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secure_default_requires_tls_12() {
        assert_eq!(TlsConfig::secure_default().minimum_version(), TlsVersion::Tls12);
    }

    #[test]
    fn secure_default_client_can_be_constructed() {
        assert!(build_client(TlsConfig::secure_default(), None, None).is_ok());
    }
}
