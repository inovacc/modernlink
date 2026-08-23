use runtime_observer::{
    Capability, ConnectorKind, GenericHttpConnector, HttpResponse, HttpTransport, ObservationDepth,
    RuntimeConnector, RuntimeProfile,
};

struct FixtureTransport;

impl HttpTransport for FixtureTransport {
    fn get(&self, endpoint: &str) -> Result<HttpResponse, runtime_observer::ObservationError> {
        assert_eq!(endpoint, "http://127.0.0.1:18080/metadata");
        Ok(HttpResponse {
            status: 200,
            content_type: Some("application/json".to_owned()),
            body: br#"{"service":"orders","state":"running","token":"must-not-escape"}"#.to_vec(),
        })
    }
}

fn profile() -> RuntimeProfile {
    RuntimeProfile {
        schema_version: "modernlink.runtime-profile/v1".to_owned(),
        name: "orders-loopback".to_owned(),
        kind: ConnectorKind::GenericHttp,
        endpoint: "http://127.0.0.1:18080/metadata".to_owned(),
        scope: None,
        authorization: None,
        credential_ref: None,
        tls: None,
        tunnel_bind: None,
        http_method: "GET".to_owned(),
    }
}

#[test]
fn generic_http_observation_redacts_before_digest_and_ignores_capture_time() {
    let connector = GenericHttpConnector::new(FixtureTransport);
    let first = connector
        .observe_metadata(
            &profile(),
            ObservationDepth::Metadata,
            "2026-08-23T00:00:00Z",
        )
        .expect("first metadata observation");
    let second = connector
        .observe_metadata(
            &profile(),
            ObservationDepth::Metadata,
            "2026-08-23T01:00:00Z",
        )
        .expect("second metadata observation");

    assert_eq!(
        first.schema_version,
        "modernlink.runtime-observation/v1alpha1"
    );
    assert_eq!(first.content_digest, second.content_digest);
    assert_eq!(first.evidence[0].payload["token"], "[REDACTED]");
    assert!(
        !first
            .canonical_json()
            .expect("JSON")
            .contains("must-not-escape")
    );
}

#[test]
fn generic_http_probe_reports_json_capability() {
    let connector = GenericHttpConnector::new(FixtureTransport);
    let report = connector.probe(&profile()).expect("probe");

    assert_eq!(report.connector, "generic-http");
    assert_eq!(
        report.capabilities,
        vec![Capability::supported("http-json-metadata")]
    );
}

#[test]
fn profile_rejects_inline_credentials_tunnel_bind_and_state_changing_method() {
    for profile_json in [
        r#"{"schema_version":"modernlink.runtime-profile/v1","name":"x","kind":"generic-http","endpoint":"http://127.0.0.1","credential_ref":{"kind":"inline","value":"secret"},"http_method":"GET"}"#,
        r#"{"schema_version":"modernlink.runtime-profile/v1","name":"x","kind":"generic-http","endpoint":"http://127.0.0.1","tunnel_bind":"0.0.0.0","http_method":"GET"}"#,
        r#"{"schema_version":"modernlink.runtime-profile/v1","name":"x","kind":"generic-http","endpoint":"http://127.0.0.1","http_method":"POST"}"#,
        r#"{"schema_version":"modernlink.runtime-profile/v1","name":"x","kind":"generic-http","endpoint":"http://127.0.0.1","http_method":"GET","deep_categories":["unknown"]}"#,
    ] {
        assert!(
            RuntimeProfile::from_json(profile_json).is_err(),
            "{profile_json}"
        );
    }
}
