//! The TLS policy boundary for ModernLink, backed by rustls with webpki roots.
//!
//! TLS terminates and is verified here rather than in Java, which is why the Java facade
//! rejects custom `HostnameVerifier` and `SSLSocketFactory` instances instead of silently
//! ignoring them (`docs/ISSUES.md` I-008). The floor is TLS 1.2; callers may select 1.2 or
//! 1.3, and unsupported values are rejected before a request starts.

pub use modernlink_core::TlsVersion;
use rustls::{ClientConfig, RootCertStore};
use std::sync::Arc;

#[derive(Debug, Clone, Copy)]
pub struct TlsConfig {
    minimum_version: TlsVersion,
}

impl TlsConfig {
    pub fn secure_default() -> Self {
        Self {
            minimum_version: TlsVersion::Tls12,
        }
    }

    pub fn minimum_version(&self) -> TlsVersion {
        self.minimum_version
    }

    pub fn with_minimum_version(minimum_version: TlsVersion) -> Self {
        Self { minimum_version }
    }
}

pub fn client_config(config: TlsConfig) -> Arc<ClientConfig> {
    let roots = RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let versions: &[&'static rustls::SupportedProtocolVersion] = match config.minimum_version {
        TlsVersion::Tls12 => &[&rustls::version::TLS13, &rustls::version::TLS12],
        TlsVersion::Tls13 => &[&rustls::version::TLS13],
    };
    Arc::new(
        ClientConfig::builder_with_provider(provider)
            .with_protocol_versions(versions)
            .expect("ModernLink TLS versions are supported")
            .with_root_certificates(roots)
            .with_no_client_auth(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secure_default_requires_tls_12() {
        assert_eq!(
            TlsConfig::secure_default().minimum_version(),
            TlsVersion::Tls12
        );
    }

    #[test]
    fn secure_default_client_config_can_be_constructed() {
        let _ = client_config(TlsConfig::secure_default());
    }
}
