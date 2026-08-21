//! The TLS policy boundary for ModernLink, backed by rustls with webpki roots.
//!
//! TLS terminates and is verified here rather than in Java, which is why the Java facade
//! rejects custom `HostnameVerifier` and `SSLSocketFactory` instances instead of silently
//! ignoring them (`docs/ISSUES.md` I-008). The floor is TLS 1.2; callers may select 1.2 or
//! 1.3, and unsupported values are rejected before a request starts.

use modernlink_core::Error;
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

/// Build the client TLS configuration.
///
/// H-10: returns `Result` rather than expecting. The version list is a static constant, so
/// the old `.expect("ModernLink TLS versions are supported")` was practically unreachable -
/// but it asserted a *dependency's* behaviour, not this crate's own invariant, and it sat in
/// a library reachable from an FFI entry point where a panic used to be undefined behaviour.
/// An unreachable panic is still a panic path.
pub fn client_config(config: TlsConfig) -> Result<Arc<ClientConfig>, Error> {
    let roots = RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let versions: &[&'static rustls::SupportedProtocolVersion] = match config.minimum_version {
        TlsVersion::Tls12 => &[&rustls::version::TLS13, &rustls::version::TLS12],
        TlsVersion::Tls13 => &[&rustls::version::TLS13],
    };
    let builder = ClientConfig::builder_with_provider(provider)
        .with_protocol_versions(versions)
        .map_err(|error| {
            Error::InvalidRequest(format!("TLS protocol versions were rejected: {error}"))
        })?;
    Ok(Arc::new(
        builder.with_root_certificates(roots).with_no_client_auth(),
    ))
}

#[cfg(test)]
mod tests;
