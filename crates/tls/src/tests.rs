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
    client_config(TlsConfig::secure_default())
        .expect("the built-in version list must always be accepted");
}

#[test]
fn with_minimum_version_selects_tls_13() {
    let config = TlsConfig::with_minimum_version(TlsVersion::Tls13);

    assert_eq!(config.minimum_version(), TlsVersion::Tls13);
}

#[test]
fn tls_13_client_config_can_be_constructed() {
    client_config(TlsConfig::with_minimum_version(TlsVersion::Tls13))
        .expect("the TLS 1.3 version list must be accepted");
}
