use runtime_observer::{
    Capability, ConnectorKind, GenericHttpConnector, HttpResponse, HttpTransport, ObservationDepth,
    ObservationSnapshot, RuntimeConnector, RuntimeEvidence, RuntimeObservation, RuntimeProfile,
};
use std::collections::BTreeMap;

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

#[test]
fn profile_rejects_credential_bearing_endpoint_query_keys() {
    for key in [
        "token",
        "access_token",
        "apiKey",
        "password",
        "client-secret",
    ] {
        let profile = format!(
            r#"{{"schema_version":"modernlink.runtime-profile/v1","name":"x","kind":"generic-http","endpoint":"http://127.0.0.1/metadata?{key}=secret","http_method":"GET"}}"#
        );
        assert!(RuntimeProfile::from_json(&profile).is_err(), "{key}");
    }
}

#[test]
fn profile_rejects_credential_key_variants_without_rejecting_ordinary_words() {
    for key in [
        "auth_token",
        "bearer_token",
        "refresh_token",
        "client_secret",
        "api_key",
        "access-key",
        "password",
        "passwd",
        "pwd",
        "credential",
        "credentials",
        "secret",
        "token",
    ] {
        let profile = format!(
            r#"{{"schema_version":"modernlink.runtime-profile/v1","name":"x","kind":"generic-http","endpoint":"http://127.0.0.1/metadata?{key}=secret","http_method":"GET"}}"#
        );
        assert!(RuntimeProfile::from_json(&profile).is_err(), "{key}");
    }
    for key in [
        "tokenizer",
        "secretary",
        "passwordless",
        "tokenCount",
        "secretariat",
    ] {
        let profile = format!(
            r#"{{"schema_version":"modernlink.runtime-profile/v1","name":"x","kind":"generic-http","endpoint":"http://127.0.0.1/metadata?{key}=value","http_method":"GET"}}"#
        );
        assert!(RuntimeProfile::from_json(&profile).is_ok(), "{key}");
    }
}

#[test]
fn sensitive_key_matrix_rejects_profiles_and_redacts_payload_digests() {
    let keys = [
        "apiKey",
        "api-key",
        "apikey",
        "authToken",
        "authtoken",
        "accessKey",
        "accesskey",
        "privateKey",
        "privatekey",
        "bearerToken",
        "refreshToken",
        "clientSecret",
        "password",
        "passwd",
        "pwd",
        "credential",
        "credentials",
        "secret",
        "token",
    ];
    for key in keys {
        let unsafe_profile = format!(
            r#"{{"schema_version":"modernlink.runtime-profile/v1","name":"x","kind":"generic-http","endpoint":"http://127.0.0.1/metadata?{key}=first","http_method":"GET"}}"#
        );
        assert!(RuntimeProfile::from_json(&unsafe_profile).is_err(), "{key}");

        let first = snapshot_with_payload_key(key, "first");
        let second = snapshot_with_payload_key(key, "second");
        let json = first.canonical_json().expect("JSON");
        assert!(!json.contains("first"), "{key}");
        assert_eq!(first.evidence[0].payload[key], "[REDACTED]", "{key}");
        assert_eq!(first.content_digest, second.content_digest, "{key}");
    }
}

#[test]
fn sensitive_suffixes_reject_profiles_and_redact_payloads_without_overmatching() {
    for key in [
        "sessionToken",
        "csrfToken",
        "jwtToken",
        "awsAccessKey",
        "token2",
    ] {
        let unsafe_profile = format!(
            r#"{{"schema_version":"modernlink.runtime-profile/v1","name":"x","kind":"generic-http","endpoint":"http://127.0.0.1/metadata?{key}=secret","http_method":"GET"}}"#
        );
        assert!(RuntimeProfile::from_json(&unsafe_profile).is_err(), "{key}");
        assert_eq!(
            snapshot_with_payload_key(key, "secret").evidence[0].payload[key],
            "[REDACTED]",
            "{key}"
        );
    }
    for key in ["tokenCount", "secretariat"] {
        let profile = format!(
            r#"{{"schema_version":"modernlink.runtime-profile/v1","name":"x","kind":"generic-http","endpoint":"http://127.0.0.1/metadata?{key}=value","http_method":"GET"}}"#
        );
        assert!(RuntimeProfile::from_json(&profile).is_ok(), "{key}");
        assert_eq!(
            snapshot_with_payload_key(key, "value").evidence[0].payload[key],
            "value"
        );
    }
}

fn snapshot_with_payload_key(key: &str, value: &str) -> ObservationSnapshot {
    ObservationSnapshot::from_observation(
        &profile(),
        "test",
        "2026-08-23T00:00:00Z",
        ObservationDepth::Metadata,
        RuntimeObservation {
            capabilities: vec![],
            evidence: vec![RuntimeEvidence {
                id: "caller-provided".to_owned(),
                collector: "test".to_owned(),
                target: "http://127.0.0.1/metadata".to_owned(),
                resource_key: "http://127.0.0.1/metadata".to_owned(),
                source_operation: "GET".to_owned(),
                payload_digest: "caller-provided".to_owned(),
                payload: serde_json::json!({key: value}),
            }],
            redaction_counters: BTreeMap::new(),
        },
    )
    .expect("snapshot")
}

#[test]
fn profile_rejects_unknown_nested_credential_material() {
    let profile = r#"{
        "schema_version":"modernlink.runtime-profile/v1",
        "name":"x",
        "kind":"generic-http",
        "endpoint":"http://127.0.0.1/metadata",
        "http_method":"GET",
        "credential_ref":{"kind":"environment","variable":"RUNTIME_TOKEN","api_key":"secret"}
    }"#;
    assert!(RuntimeProfile::from_json(profile).is_err());
}

#[test]
fn snapshot_constructor_redacts_untrusted_payload_and_resource_query_before_digesting() {
    let snapshot = ObservationSnapshot::from_observation(
        &profile(),
        "test",
        "2026-08-23T00:00:00Z",
        ObservationDepth::Metadata,
        RuntimeObservation {
            capabilities: vec![],
            evidence: vec![RuntimeEvidence {
                id: "caller-provided".to_owned(),
                collector: "test".to_owned(),
                target: "http://127.0.0.1/metadata?token=secret".to_owned(),
                resource_key: "http://127.0.0.1/metadata?token=secret".to_owned(),
                source_operation: "GET".to_owned(),
                payload_digest: "caller-provided".to_owned(),
                payload: serde_json::json!({"password": "secret"}),
            }],
            redaction_counters: BTreeMap::from([("connector-rule".to_owned(), 2)]),
        },
    )
    .expect("snapshot");

    let json = snapshot.canonical_json().expect("JSON");
    assert!(!json.contains("secret"));
    assert_eq!(snapshot.evidence[0].payload["password"], "[REDACTED]");
    assert!(snapshot.evidence[0].resource_key.contains("%5BREDACTED%5D"));
    assert_ne!(snapshot.evidence[0].payload_digest, "caller-provided");
    assert_eq!(snapshot.redaction_counters["connector-rule"], 2);
    assert_eq!(snapshot.redaction_counters["sensitive-field"], 1);
}

#[test]
fn generic_http_accepts_case_insensitive_json_content_type() {
    struct MixedCaseTransport;
    impl HttpTransport for MixedCaseTransport {
        fn get(&self, _: &str) -> Result<HttpResponse, runtime_observer::ObservationError> {
            Ok(HttpResponse {
                status: 200,
                content_type: Some("Application/JSON; charset=UTF-8".to_owned()),
                body: br#"{"service":"orders"}"#.to_vec(),
            })
        }
    }

    assert!(
        GenericHttpConnector::new(MixedCaseTransport)
            .probe(&profile())
            .is_ok()
    );
}
