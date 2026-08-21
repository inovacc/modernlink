use super::{
    AcknowledgementMode, DeliveryMode, DeliveryReceipt, DeliveryState, DomainError,
    InMemoryTransport, MessageEnvelope, MessageTransport, Mode, Payload, Provider,
    ReceiveSemantics, RouteConfig, RouteRule, Support,
};

fn message() -> MessageEnvelope {
    let mut message = MessageEnvelope::new(
        "orders.created",
        Payload::Text("body".to_string()),
        1_700_000_000_000,
    )
    .unwrap();
    message.tenant = Some("acme".to_string());
    message
        .headers
        .insert("region".to_string(), "us".to_string());
    message
}

#[cfg(any(
    feature = "nats",
    feature = "rabbitmq",
    feature = "kafka",
    feature = "pulsar"
))]
fn receipt_for(provider: Provider) -> DeliveryReceipt {
    DeliveryReceipt {
        message_id: "missing-message".to_string(),
        provider,
        state: DeliveryState::Received,
        trace_id: "trace".to_string(),
    }
}

fn poison_mutex<T>(mutex: &std::sync::Mutex<T>) {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _guard = mutex.lock().expect("test mutex should start available");
        panic!("intentionally poison provider state");
    }));
    assert!(result.is_err());
    assert!(mutex.is_poisoned());
}

#[test]
fn creates_versioned_uuidv7_envelope() {
    let message = message();
    assert_eq!(message.version, 1);
    assert_eq!(message.message_id.len(), 36);
    assert_eq!(&message.message_id[14..15], "7");
    assert_eq!(message.delivery_mode, DeliveryMode::Persistent);
    assert_eq!(message.acknowledgement_mode, AcknowledgementMode::Auto);
    assert_eq!(message.tracing.trace_id.len(), 32);
    assert_eq!(message.tracing.span_id.len(), 16);
}

#[test]
fn first_matching_rule_wins_and_preserves_rule_id() {
    let config = RouteConfig {
        default_mode: Mode::Transparent,
        default_provider: Provider::LegacyJms,
        rules: vec![RouteRule {
            id: "orders-to-kafka".to_string(),
            destination: Some("orders.created".to_string()),
            destination_prefix: None,
            tenant: Some("acme".to_string()),
            header_name: Some("region".to_string()),
            header_value: Some("us".to_string()),
            mode: Mode::Transform,
            provider: Provider::Kafka,
            allowed: true,
        }],
    };
    let decision = config.decide(&message()).unwrap();
    assert_eq!(decision.rule_id.as_deref(), Some("orders-to-kafka"));
    assert_eq!(decision.mode, Mode::Transform);
    assert_eq!(decision.provider, Provider::Kafka);
}

#[test]
fn rejects_invalid_transparent_provider_pair() {
    let config = RouteConfig {
        default_mode: Mode::Transparent,
        default_provider: Provider::Kafka,
        rules: Vec::new(),
    };
    let error = config
        .decide(&message())
        .expect_err("transparent mode must use LegacyJms");
    assert!(matches!(error, DomainError::InvalidRoute(_)));
    assert!(error.to_string().contains("LegacyJms"));
}

#[test]
fn rejects_modern_modes_with_the_legacy_provider() {
    for mode in [Mode::Transform, Mode::Redirect] {
        let config = RouteConfig {
            default_mode: mode,
            default_provider: Provider::LegacyJms,
            rules: Vec::new(),
        };
        let error = config
            .decide(&message())
            .expect_err("modern modes need a modern provider");
        assert!(matches!(error, DomainError::InvalidRoute(_)));
        assert!(error.to_string().contains("modern provider"));
    }
}

#[test]
fn non_matching_route_constraints_fall_back_to_the_default() {
    let cases = [
        ("destination", "other.destination", None, None, None, None),
        (
            "prefix",
            "orders.created",
            Some("billing."),
            None,
            None,
            None,
        ),
        ("tenant", "orders.created", None, Some("other"), None, None),
        (
            "header",
            "orders.created",
            None,
            None,
            Some("eu"),
            Some("region"),
        ),
    ];

    for (label, destination, prefix, tenant, header_value, header_name) in cases {
        let rule = RouteRule {
            id: label.to_string(),
            destination: (label == "destination").then(|| destination.to_string()),
            destination_prefix: prefix.map(str::to_string),
            tenant: tenant.map(str::to_string),
            header_name: header_name.map(str::to_string),
            header_value: header_value.map(str::to_string),
            mode: Mode::Transform,
            provider: Provider::Kafka,
            allowed: true,
        };
        let config = RouteConfig {
            default_mode: Mode::Transparent,
            default_provider: Provider::LegacyJms,
            rules: vec![rule],
        };
        let decision = config.decide(&message()).unwrap();
        assert_eq!(decision.rule_id, None, "{label} unexpectedly matched");
        assert_eq!(decision.mode, Mode::Transparent, "{label}");
        assert_eq!(decision.provider, Provider::LegacyJms, "{label}");
    }
}

#[test]
fn serializes_envelope_without_java_runtime_types() {
    let json = message().to_json().unwrap();
    assert!(json.contains("orders.created"));
    assert!(json.contains("message_id"));
}

#[test]
fn rejects_blank_destinations_before_constructing_an_envelope() {
    for destination in ["", "   ", "\t\n"] {
        let error = MessageEnvelope::new(destination, Payload::Bytes(Vec::new()), 0)
            .expect_err("blank destinations must be invalid envelopes");
        assert!(
            matches!(error, DomainError::InvalidEnvelope(_)),
            "wrong error for {destination:?}: {error:?}"
        );
        assert_eq!(error.to_string(), "destination must not be empty");
    }
}

#[test]
fn every_payload_variant_round_trips_through_envelope_json() {
    let mut map = std::collections::BTreeMap::new();
    map.insert("empty-value".to_string(), String::new());
    map.insert("unicode".to_string(), "日本語".to_string());
    let payloads = vec![
        Payload::Text(String::new()),
        Payload::Bytes(vec![0, 1, 255]),
        Payload::Map(map),
        Payload::Stream(Vec::new()),
        Payload::Object {
            content_type: "application/octet-stream".to_string(),
            bytes: vec![42],
        },
    ];

    for payload in payloads {
        let mut envelope = MessageEnvelope::new("payloads", payload.clone(), 0).unwrap();
        envelope.payload = payload.clone();
        let encoded = envelope.to_json().unwrap();
        let decoded: MessageEnvelope = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded.payload, payload);
    }
}

#[test]
fn uniform_transport_preserves_message_for_modern_provider_identity() {
    let transport = InMemoryTransport::new(Provider::Kafka);
    let message = message();
    let message_id = message.message_id.clone();
    let published = transport.publish(message).unwrap();
    assert_eq!(transport.provider(), Provider::Kafka);
    assert_eq!(published.state, DeliveryState::Published);
    let received = transport.receive().unwrap().unwrap();
    assert_eq!(received.message.message_id, message_id);
    let acknowledged = transport.acknowledge(&received.receipt).unwrap();
    assert_eq!(acknowledged.state, DeliveryState::Acknowledged);
}

#[test]
fn in_memory_acknowledgement_is_idempotent_and_keeps_receipt_identity() {
    let transport = InMemoryTransport::new(Provider::LegacyJms);
    let published = transport.publish(message()).unwrap();
    let first = transport.acknowledge(&published).unwrap();
    let second = transport.acknowledge(&published).unwrap();

    assert_eq!(first.state, DeliveryState::Acknowledged);
    assert_eq!(second, first);
    assert_eq!(first.message_id, published.message_id);
    assert_eq!(first.trace_id, published.trace_id);
    assert_eq!(
        *transport.acknowledged.lock().unwrap(),
        std::collections::BTreeSet::from([published.message_id])
    );
}

#[test]
fn in_memory_acknowledgement_rejects_a_receipt_from_another_provider() {
    let transport = InMemoryTransport::new(Provider::LegacyJms);
    let receipt = DeliveryReceipt {
        message_id: "foreign".to_string(),
        provider: Provider::Kafka,
        state: DeliveryState::Received,
        trace_id: "trace".to_string(),
    };
    let error = transport
        .acknowledge(&receipt)
        .expect_err("a receipt from another provider cannot be acknowledged here");
    assert!(matches!(error, DomainError::Transport(_)));
    assert_eq!(
        error.to_string(),
        "receipt provider does not match transport"
    );
}

#[test]
fn poisoned_in_memory_queue_is_reported_as_a_transport_error() {
    let transport = InMemoryTransport::new(Provider::LegacyJms);
    let queue = transport.queue.clone();
    std::thread::spawn(move || {
        let _guard = queue.lock().unwrap();
        panic!("poison queue for deterministic error-path coverage");
    })
    .join()
    .expect_err("the helper thread must poison the queue mutex");

    let publish_error = transport
        .publish(message())
        .expect_err("publishing through a poisoned queue must fail closed");
    let receive_error = transport
        .receive()
        .expect_err("receiving through a poisoned queue must fail closed");
    assert_eq!(publish_error.to_string(), "transport queue is unavailable");
    assert_eq!(receive_error.to_string(), "transport queue is unavailable");
}

#[test]
fn poisoned_in_memory_acknowledgement_registry_is_reported() {
    let transport = InMemoryTransport::new(Provider::LegacyJms);
    let acknowledged = transport.acknowledged.clone();
    std::thread::spawn(move || {
        let _guard = acknowledged.lock().unwrap();
        panic!("poison acknowledgement registry for deterministic error-path coverage");
    })
    .join()
    .expect_err("the helper thread must poison the acknowledgement registry");

    let error = transport
        .acknowledge(&DeliveryReceipt {
            message_id: "message".to_string(),
            provider: Provider::LegacyJms,
            state: DeliveryState::Received,
            trace_id: "trace".to_string(),
        })
        .expect_err("a poisoned acknowledgement registry must fail closed");
    assert_eq!(error.to_string(), "acknowledgement store is unavailable");
}

#[test]
fn legacy_jms_transport_preserves_message_and_acknowledgement_contract() {
    let transport =
        super::MessageTransportKind::LegacyJms(InMemoryTransport::new(Provider::LegacyJms));
    let message = message();
    let message_id = message.message_id.clone();
    let trace_id = message.tracing.trace_id.clone();

    let published = transport.publish(message).unwrap();
    let received = transport.receive().unwrap().unwrap();
    let acknowledged = transport.acknowledge(&received.receipt).unwrap();

    assert_eq!(published.provider, Provider::LegacyJms);
    assert_eq!(published.state, DeliveryState::Published);
    assert_eq!(received.message.message_id, message_id);
    assert_eq!(received.message.tracing.trace_id, trace_id);
    assert_eq!(acknowledged.state, DeliveryState::Acknowledged);
}

#[test]
fn dispatch_applies_transform_metadata_and_returns_receipt() {
    let config = RouteConfig {
        default_mode: Mode::Transform,
        default_provider: Provider::Kafka,
        rules: Vec::new(),
    };
    let transport = InMemoryTransport::new(Provider::Kafka);
    let result = config.dispatch(message(), &transport).unwrap();
    assert_eq!(result.decision.mode, Mode::Transform);
    assert_eq!(result.receipt.state, DeliveryState::Published);
    let received = transport.receive().unwrap().unwrap();
    assert_eq!(
        received.message.properties.get("modernlink.mode"),
        Some(&"Transform".to_string())
    );
}

#[test]
fn transparent_dispatch_preserves_properties_without_modern_metadata() {
    let config = RouteConfig {
        default_mode: Mode::Transparent,
        default_provider: Provider::LegacyJms,
        rules: Vec::new(),
    };
    let transport = InMemoryTransport::new(Provider::LegacyJms);
    let result = config.dispatch(message(), &transport).unwrap();
    assert_eq!(result.decision.mode, Mode::Transparent);
    let received = transport.receive().unwrap().unwrap();
    assert_eq!(received.message.properties.get("modernlink.mode"), None);
    assert_eq!(received.message.properties.get("modernlink.provider"), None);
}

#[test]
fn dispatch_rejects_provider_mismatch_and_denied_routes() {
    let mismatch = RouteConfig {
        default_mode: Mode::Redirect,
        default_provider: Provider::Nats,
        rules: Vec::new(),
    };
    assert!(mismatch
        .dispatch(message(), &InMemoryTransport::new(Provider::Kafka))
        .is_err());

    let denied = RouteConfig {
        default_mode: Mode::Redirect,
        default_provider: Provider::Nats,
        rules: vec![RouteRule {
            id: "deny-orders".to_string(),
            destination: Some("orders.created".to_string()),
            destination_prefix: None,
            tenant: None,
            header_name: None,
            header_value: None,
            mode: Mode::Redirect,
            provider: Provider::Nats,
            allowed: false,
        }],
    };
    assert!(denied
        .dispatch(message(), &InMemoryTransport::new(Provider::Nats))
        .is_err());
}

#[test]
fn dry_run_returns_denied_decision_without_publishing() {
    let config = RouteConfig {
        default_mode: Mode::Transparent,
        default_provider: Provider::LegacyJms,
        rules: vec![RouteRule {
            id: "hold-orders".to_string(),
            destination: Some("orders.created".to_string()),
            destination_prefix: None,
            tenant: None,
            header_name: None,
            header_value: None,
            mode: Mode::Redirect,
            provider: Provider::Nats,
            allowed: false,
        }],
    };

    let decision = config.dry_run(&message()).unwrap();
    assert_eq!(decision.rule_id.as_deref(), Some("hold-orders"));
    assert_eq!(decision.mode, Mode::Redirect);
    assert_eq!(decision.provider, Provider::Nats);
    assert!(!decision.allowed);
}

#[test]
fn every_domain_error_variant_displays_its_diagnostic() {
    let errors = [
        (
            DomainError::InvalidEnvelope("envelope".to_string()),
            "envelope",
        ),
        (DomainError::InvalidRoute("route".to_string()), "route"),
        (
            DomainError::Serialization("serialization".to_string()),
            "serialization",
        ),
        (DomainError::Transport("transport".to_string()), "transport"),
        (
            DomainError::Unsupported("unsupported".to_string()),
            "unsupported",
        ),
    ];
    for (error, expected) in errors {
        assert_eq!(error.to_string(), expected);
    }
}

// ---- MSG-04: the provider guarantee table ----

const ALL_PROVIDERS: [Provider; 6] = [
    Provider::LegacyJms,
    Provider::Nats,
    Provider::NatsJetStream,
    Provider::Kafka,
    Provider::Pulsar,
    Provider::RabbitMq,
];

#[test]
fn every_provider_declares_a_guarantee_table() {
    for provider in ALL_PROVIDERS {
        let guarantees = provider.guarantees();
        assert_eq!(
            guarantees.provider, provider,
            "the table must describe the provider it was asked about"
        );
    }
}

/// Core NATS is fire-and-forget: there is no broker-side state at all. A caller that
/// asks for persistence must be told no, because the alternative -- accepting and
/// delivering non-persistently -- is the silent downgrade AGENTS.md forbids.
#[test]
fn nats_core_refuses_persistent_delivery_rather_than_downgrading_it() {
    let guarantees = Provider::Nats.guarantees();
    let error = guarantees
        .require_delivery_mode(DeliveryMode::Persistent)
        .expect_err("core NATS cannot persist and must refuse");
    assert!(matches!(error, DomainError::Unsupported(_)));
    assert!(
        error.to_string().contains("refused rather than downgraded"),
        "the refusal must say no downgrade happened: {}",
        error
    );
    guarantees
        .require_delivery_mode(DeliveryMode::NonPersistent)
        .expect("non-persistent is what core NATS actually offers");
}

#[test]
fn nats_core_refuses_client_acknowledgement() {
    let error = Provider::Nats
        .guarantees()
        .require_acknowledgement_mode(AcknowledgementMode::Client)
        .expect_err("core NATS has no server-side ack state");
    assert!(matches!(error, DomainError::Unsupported(_)));
}

#[test]
fn automatic_and_duplicate_acknowledgement_are_always_accepted() {
    for provider in ALL_PROVIDERS {
        let guarantees = provider.guarantees();
        guarantees
            .require_acknowledgement_mode(AcknowledgementMode::Auto)
            .expect("AUTO is the baseline mode for every provider");
        guarantees
            .require_acknowledgement_mode(AcknowledgementMode::DuplicateOk)
            .expect("DUPLICATE_OK has no extra broker capability requirement");
    }
}

#[test]
fn declared_persistence_is_accepted_without_downgrade() {
    for provider in [Provider::NatsJetStream, Provider::Kafka, Provider::Pulsar] {
        provider
            .guarantees()
            .require_delivery_mode(DeliveryMode::Persistent)
            .expect("declared persistence must not be rejected or downgraded");
    }
}

#[test]
fn support_levels_render_their_wire_values() {
    assert_eq!(Support::Verified.as_str(), "VERIFIED");
    assert_eq!(Support::Declared.as_str(), "DECLARED");
    assert_eq!(Support::Unsupported.as_str(), "UNSUPPORTED");
}

#[test]
fn jetstream_accepts_client_acknowledgement() {
    Provider::NatsJetStream
        .guarantees()
        .require_acknowledgement_mode(AcknowledgementMode::Client)
        .expect("JetStream uses AckPolicy::Explicit, so CLIENT ack is real");
}

/// No transport in this crate implements transactions. Until one does, every provider
/// must refuse TRANSACTED -- accepting it and behaving as if AUTO is exactly how a
/// rollback silently becomes a commit.
#[test]
fn no_provider_accepts_transacted_acknowledgement_today() {
    for provider in ALL_PROVIDERS {
        let error = provider
            .guarantees()
            .require_acknowledgement_mode(AcknowledgementMode::Transacted)
            .expect_err("no transport implements transactions yet");
        assert!(
            matches!(error, DomainError::Unsupported(_)),
            "{:?}",
            provider
        );
    }
}

/// The three-level scale only earns its complexity if Declared is not treated as
/// proof. This pins that: a claim must not read as evidence.
#[test]
fn declared_is_not_proven() {
    assert!(Support::Verified.is_proven());
    assert!(!Support::Declared.is_proven());
    assert!(!Support::Unsupported.is_proven());
}

/// B-003. RabbitMQ declares its queue durable but publishes with
/// BasicProperties::default(), i.e. AMQP delivery_mode 1 (transient), so messages do
/// not survive a broker restart. The table must record the behaviour, not the intent.
/// If someone fixes the publisher, this test should fail and be updated deliberately.
#[test]
fn rabbitmq_persistence_records_the_behaviour_not_the_intent() {
    assert_eq!(
        Provider::RabbitMq.guarantees().persistence,
        Support::Unsupported,
        "see docs/BUGS.md B-003: the publisher never sets delivery_mode 2"
    );
}

#[cfg(feature = "nats")]
#[test]
fn nats_connect_rejects_an_empty_subject_before_network_io() {
    let error = super::NatsTransport::connect("nats://127.0.0.1:4222", "")
        .err()
        .expect("an empty NATS subject is an invalid envelope");
    assert!(matches!(error, DomainError::InvalidEnvelope(_)));
    assert_eq!(error.to_string(), "NATS subject must not be empty");
}

#[cfg(feature = "nats")]
#[test]
fn jetstream_connect_rejects_missing_names_before_network_io() {
    let error =
        super::NatsJetStreamTransport::connect("nats://127.0.0.1:4222", "orders", "", "consumer")
            .err()
            .expect("a JetStream stream name is required");
    assert!(matches!(error, DomainError::InvalidEnvelope(_)));
    assert!(error.to_string().contains("subject, stream, and consumer"));
}

#[cfg(feature = "kafka")]
#[test]
fn kafka_connect_rejects_missing_connection_fields_before_network_io() {
    let error = super::KafkaTransport::connect("", "orders", "group")
        .err()
        .expect("Kafka brokers are required");
    assert!(matches!(error, DomainError::InvalidEnvelope(_)));
    assert!(error.to_string().contains("brokers, topic, and group ID"));
}

#[cfg(feature = "rabbitmq")]
#[test]
fn rabbitmq_connect_rejects_missing_connection_fields_before_network_io() {
    let error = super::RabbitMqTransport::connect("", "orders")
        .err()
        .expect("RabbitMQ URI is required");
    assert!(matches!(error, DomainError::InvalidEnvelope(_)));
    assert!(error.to_string().contains("URI and queue are required"));
}

#[cfg(feature = "pulsar")]
#[test]
fn pulsar_connect_rejects_missing_connection_fields_before_network_io() {
    let error = super::PulsarTransport::connect("", "orders", "consumer")
        .err()
        .expect("Pulsar service URL is required");
    assert!(matches!(error, DomainError::InvalidEnvelope(_)));
    assert!(error
        .to_string()
        .contains("service URL, topic, and subscription"));
}

// ---- H-02 / BACKLOG P1: broker operations must be bounded ----

fn test_runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Runtime::new().expect("a tokio runtime is required for these tests")
}

/// The whole point: an operation that never completes must return, not hang.
///
/// `pending()` is used rather than a real unreachable endpoint so the test is
/// deterministic and needs no network — a black-holed address would make this test
/// depend on the firewall of whatever machine runs it.
#[test]
fn an_operation_that_never_completes_is_refused_at_the_deadline() {
    let runtime = test_runtime();
    let started = std::time::Instant::now();
    let result: Result<(), DomainError> = super::block_on_with_timeout(
        &runtime,
        "test operation",
        std::time::Duration::from_millis(150),
        std::future::pending::<()>(),
    );
    let error = result.expect_err("a future that never completes must be refused");
    assert!(
        matches!(error, DomainError::Transport(_)),
        "expiry is a transport failure: {:?}",
        error
    );
    assert!(
        error
            .to_string()
            .contains("refused rather than left to block"),
        "the message must say it was refused, not that it failed: {}",
        error
    );
    assert!(
        started.elapsed() < std::time::Duration::from_secs(5),
        "it must return at the deadline, not hang"
    );
}

#[test]
fn an_operation_that_completes_passes_its_value_through() {
    let runtime = test_runtime();
    let result = super::block_on_with_timeout(
        &runtime,
        "test operation",
        std::time::Duration::from_secs(30),
        async { 7u32 },
    );
    assert_eq!(result.expect("a completed future must pass through"), 7);
}

/// B-006: the timeout message must not become a new place for an endpoint - and its
/// credentials - to leak into a Java exception the host application logs.
#[test]
fn the_timeout_message_carries_no_endpoint() {
    let runtime = test_runtime();
    let result: Result<(), DomainError> = super::block_on_with_timeout(
        &runtime,
        "RabbitMQ connect",
        std::time::Duration::from_millis(50),
        std::future::pending::<()>(),
    );
    let message = result.expect_err("must time out").to_string();
    assert!(
        !message.contains("://"),
        "no URL scheme may appear: {}",
        message
    );
    assert!(
        !message.contains('@'),
        "no userinfo may appear: {}",
        message
    );
}

/// H-02 structural guard: no `connect` may await a broker without a deadline.
///
/// Counting call sites is crude, and it is the only check that fails when a sixth
/// transport is added with a bare `runtime.block_on(...)` in its connect — which is
/// exactly how this defect would come back. Scans the source rather than the behaviour
/// because the behaviour needs a broker that stalls, which no test can rely on.
#[test]
fn no_connect_awaits_a_broker_without_a_deadline() {
    let source = include_str!("lib.rs");
    let lines: Vec<&str> = source.lines().collect();
    let mut offenders: Vec<String> = Vec::new();
    let mut connects = 0;
    let mut index = 0;
    while index < lines.len() {
        if lines[index].trim_start().starts_with("pub fn connect") {
            connects += 1;
            let mut end = index;
            while end < lines.len() && lines[end] != "    }" {
                end += 1;
            }
            for line in &lines[index..end] {
                if line.contains("block_on") && !line.contains("block_on_with_timeout") {
                    offenders.push((*line).trim().to_string());
                }
            }
            index = end;
        }
        index += 1;
    }
    assert!(
        connects >= 5,
        "expected at least 5 connect fns, found {connects}"
    );
    assert!(
        offenders.is_empty(),
        "every broker connect must be bounded by block_on_with_timeout; unbounded: {:?}. \
             An unbounded connect hangs the legacy application's thread through a JNI call it \
             cannot cancel - see docs/BACKLOG.md H-02.",
        offenders
    );
}

/// The 10s/30s defaults are a starting point, not a policy - Codex flagged that a
/// hard-coded bound can reject a slow-but-healthy deployment. These pin the override.
///
/// Serialised into one test because they mutate process-wide environment state.
#[cfg(any(
    feature = "nats",
    feature = "kafka",
    feature = "pulsar",
    feature = "rabbitmq"
))]
#[test]
fn the_broker_deadline_is_overridable_and_rejects_nonsense() {
    let key = "MODERNLINK_BROKER_TIMEOUT_SECS";
    let restore = std::env::var(key).ok();

    std::env::remove_var(key);
    assert_eq!(
        super::broker_timeout(10).as_secs(),
        10,
        "default applies when unset"
    );

    std::env::set_var(key, "45");
    assert_eq!(super::broker_timeout(10).as_secs(), 45, "override applies");

    std::env::set_var(key, "  45  ");
    assert_eq!(
        super::broker_timeout(10).as_secs(),
        45,
        "surrounding space is tolerated"
    );

    // Zero would mean "time out immediately", which looks exactly like a broker outage.
    std::env::set_var(key, "0");
    assert_eq!(
        super::broker_timeout(10).as_secs(),
        10,
        "zero must not be honoured"
    );

    std::env::set_var(key, "not-a-number");
    assert_eq!(
        super::broker_timeout(10).as_secs(),
        10,
        "garbage falls back"
    );

    std::env::set_var(key, "-5");
    assert_eq!(
        super::broker_timeout(10).as_secs(),
        10,
        "negative falls back"
    );

    match restore {
        Some(value) => std::env::set_var(key, value),
        None => std::env::remove_var(key),
    }
}

// ---- H-03 / B-006: credentials must never reach an error message ----

#[test]
fn userinfo_is_stripped_from_a_broker_url() {
    let message = super::redact_credentials(
        "connection refused to amqp://guest:hunter2@rabbit.internal:5672/%2f",
    );
    assert!(!message.contains("hunter2"), "password survived: {message}");
    assert!(!message.contains("guest:"), "username survived: {message}");
    assert!(
        message.contains("***@rabbit.internal:5672"),
        "host must survive: {message}"
    );
    assert!(message.contains("/%2f"), "path must survive: {message}");
}

#[test]
fn a_url_without_credentials_is_left_alone() {
    let original = "could not reach nats://127.0.0.1:4222";
    assert_eq!(super::redact_credentials(original), original);
}

/// An error can name more than one endpoint - a failover list, or a redirect.
#[test]
fn every_url_in_the_message_is_scrubbed() {
    let message = super::redact_credentials("tried amqp://a:b@one:5672 then amqp://c:d@two:5672");
    assert!(!message.contains("a:b@"), "{message}");
    assert!(!message.contains("c:d@"), "{message}");
    assert!(message.contains("***@one:5672"), "{message}");
    assert!(message.contains("***@two:5672"), "{message}");
}

/// An `@` in a path or query is not a credential. Blanking it would corrupt a useful
/// diagnostic, which is its own kind of harm.
#[test]
fn an_at_sign_outside_the_authority_is_not_treated_as_a_credential() {
    let original = "pulsar://host:6650/tenant/ns/topic@version";
    assert_eq!(super::redact_credentials(original), original);
    let query = "http://host/path?user=a@b.com";
    assert_eq!(super::redact_credentials(query), query);
}

#[test]
fn text_with_no_url_is_unchanged() {
    let original = "NATS subscription is unavailable";
    assert_eq!(super::redact_credentials(original), original);
}

/// The end-to-end contract: whatever a provider crate says, what leaves this crate as a
/// DomainError carries no password.
#[test]
fn transport_error_redacts_whatever_the_provider_said() {
    let error =
        super::transport_error("io error connecting to amqp://admin:s3cr3t@broker:5672/vhost");
    let text = error.to_string();
    assert!(
        !text.contains("s3cr3t"),
        "password reached DomainError: {text}"
    );
    assert!(matches!(error, DomainError::Transport(_)));
}

#[test]
fn transport_error_scrubs_each_url_without_corrupting_path_at_signs() {
    let error = super::transport_error(
        "tried amqp://one:first@broker-a:5672/vhost@blue, then nats://two:second@broker-b:4222",
    );
    let text = error.to_string();
    assert!(!text.contains("one:first"));
    assert!(!text.contains("two:second"));
    assert!(text.contains("***@broker-a:5672/vhost@blue"));
    assert!(text.contains("***@broker-b:4222"));
    assert!(matches!(error, DomainError::Transport(_)));
}

/// Structural guard: no site may build a transport error from a raw provider string
/// again. This is the check that fails when a sixth transport is added the old way.
#[test]
fn no_transport_error_is_built_from_an_unredacted_provider_string() {
    let source = include_str!("lib.rs");
    // Built in two pieces so this needle does not match its own source line - the
    // first run of this test failed on exactly that self-reference.
    let needle = concat!("DomainError::Transport(error.", "to_string())");
    let bare = source.matches(needle).count();
    assert_eq!(
        bare, 0,
        "{bare} site(s) still build DomainError::Transport from a raw provider error. \
             Provider errors embed the endpoint, and endpoints carry credentials - use \
             transport_error(). See docs/BUGS.md B-006."
    );
}

/// Edge cases for the scrubber, written from the questions put to the independent
/// reviewer. Self-verified (same-model) - weaker than an outside check, and better than
/// asserting the cases were considered.
#[test]
fn scrubber_edge_cases() {
    let r = super::redact_credentials;

    // Userinfo with no password still identifies an account.
    assert_eq!(r("amqp://user@host:5672"), "amqp://***@host:5672");

    // TLS schemes must be treated identically - this is the case that matters most,
    // because it is the one a security-conscious deployment actually uses.
    assert!(!r("amqps://u:p@host/v").contains("u:p"));
    assert!(!r("nats+tls://u:p@host").contains("u:p"));

    // Scheme case is not normalised by every provider crate.
    assert!(!r("AMQP://u:p@host").contains("u:p"));

    // A URL ending the string with no trailing delimiter is the common shape in an
    // error message, and an off-by-one here would leave the password intact.
    assert_eq!(
        r("failed: amqp://u:p@host:5672"),
        "failed: amqp://***@host:5672"
    );

    // A percent-encoded password contains its own '@'; rfind must take the LAST one.
    assert!(!r("amqp://user:p%40ss@host").contains("p%40ss"));

    // Punctuation around the URL must not defeat it.
    assert!(!r("connect to \"amqp://u:p@host\" failed").contains("u:p"));
    assert!(!r("(amqp://u:p@host)").contains("u:p"));

    // Multi-byte UTF-8 adjacent to the delimiters must not panic on a slice boundary.
    // A panic here is contained by jni_guard but still breaks the call.
    let _ = r("naïve://üser:pä@høst/päth");
    let _ = r("日本://語:密@ホスト/パス");
    let _ = r("amqp://u:p@host/路径@版本");
    let _ = r("://");
    let _ = r("://@");
    let _ = r("");
}

// ---- H-15 / B-007: a contained panic must not permanently break a transport ----

/// The defect, reproduced directly: take a value out, panic before putting it back, and
/// the slot must still hold it. Against the old take/await/replace code this fails -
/// the slot stays None forever and every later receive reports "unavailable" on a
/// client that still looks open.
#[test]
fn a_panic_between_take_and_replace_still_restores_the_value() {
    let slot: std::sync::Mutex<Option<String>> =
        std::sync::Mutex::new(Some("subscription".to_string()));

    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut guard =
            super::RestoreOnDrop::take_from(&slot, "missing").expect("the slot is populated");
        assert_eq!(guard.value_mut().map(|v| v.as_str()), Some("subscription"));
        panic!("the broker await blew up");
    }));

    assert!(outcome.is_err(), "the panic must actually have happened");
    let restored = slot.lock().expect("slot lock").clone();
    assert_eq!(
        restored,
        Some("subscription".to_string()),
        "the value must be back in the slot after an unwind, or the client is broken \
             for good - see docs/BUGS.md B-007"
    );
}

#[test]
fn restore_on_drop_does_not_panic_when_the_slot_is_poisoned() {
    let slot = std::sync::Mutex::new(Some(1u8));
    poison_mutex(&slot);
    let guard = super::RestoreOnDrop {
        slot: &slot,
        value: Some(2u8),
    };

    drop(guard);
    assert!(slot.is_poisoned());
}

#[test]
fn the_happy_path_restores_through_the_same_route() {
    let slot: std::sync::Mutex<Option<u32>> = std::sync::Mutex::new(Some(7));
    {
        let mut guard = super::RestoreOnDrop::take_from(&slot, "missing").expect("populated");
        assert_eq!(guard.value_mut().copied(), Some(7));
        assert!(
            slot.lock().expect("slot lock").is_none(),
            "while borrowed the slot is empty - that is the window B-007 is about"
        );
    }
    assert_eq!(*slot.lock().expect("slot lock"), Some(7));
}

#[test]
fn an_empty_slot_is_reported_not_panicked() {
    let slot: std::sync::Mutex<Option<u32>> = std::sync::Mutex::new(None);
    let error = super::RestoreOnDrop::take_from(&slot, "stream is unavailable")
        .err()
        .expect("an empty slot must be an error");
    assert!(
        error.to_string().contains("stream is unavailable"),
        "{error}"
    );
}

/// Structural guard: no receive path may go back to take/await/manual-replace. This is
/// what fails if a sixth transport copies the old shape.
#[test]
fn no_receive_path_replaces_a_borrowed_value_by_hand() {
    let source = include_str!("lib.rs");
    let needle = concat!(".replace(sub", "scription);");
    assert_eq!(
        source.matches(needle).count(),
        0,
        "a receive path restores its subscription by hand instead of via RestoreOnDrop; \
             an unwind before that line leaves the client broken for good - B-007."
    );
}

// ---- H-07: a TLS request that cannot be honoured must be refused, not downgraded ----

#[test]
fn tls_schemes_are_recognised_across_providers() {
    for endpoint in [
        "amqps://host:5671",
        "AMQPS://host:5671",
        "  amqps://host:5671  ",
        "nats+tls://host:4222",
        "tls://host:4222",
        "ssl://host:9093",
        "sasl_ssl://host:9093",
        "pulsar+ssl://host:6651",
        "https://host",
    ] {
        assert!(
            super::endpoint_requests_tls(endpoint),
            "must be recognised as a TLS request: {endpoint}"
        );
    }
}

#[test]
fn plaintext_schemes_are_not_mistaken_for_tls() {
    for endpoint in [
        "amqp://host:5672",
        "nats://host:4222",
        "pulsar://host:6650",
        "host:9092",
        "http://host",
        "",
    ] {
        assert!(
            !super::endpoint_requests_tls(endpoint),
            "must NOT be read as a TLS request: {endpoint}"
        );
    }
}

/// The defect this closes: a deployment that writes an encrypted endpoint and gets a
/// plaintext connection believes its broker traffic and credentials are protected.
/// Silence is the whole danger, so the refusal must be explicit and must say why.
#[cfg(feature = "kafka")]
#[test]
fn kafka_refuses_a_tls_endpoint_rather_than_connecting_in_plaintext() {
    let error = super::KafkaTransport::connect("ssl://broker:9093", "orders", "group")
        .err()
        .expect("a TLS endpoint must be refused, not silently downgraded");
    assert!(
        matches!(error, DomainError::Unsupported(_)),
        "a capability gap is Unsupported, not a Transport failure: {error:?}"
    );
    let text = error.to_string();
    assert!(
        text.contains("ssl"),
        "must name the missing feature: {text}"
    );
    assert!(
        text.contains("refused rather than made in plaintext"),
        "must state that no plaintext fallback happened: {text}"
    );
}

// ---- H-16 / B-010: receive() blocking semantics must be queryable ----

/// The finding, pinned. The signature promises `Option`, i.e. "there may be no message".
/// That promise holds for two of six providers; for the other four `Ok(None)` is
/// unreachable and the call never returns. A caller must be able to find that out
/// without discovering it as a hang in production.
#[test]
fn receive_semantics_are_declared_for_every_provider() {
    use ReceiveSemantics::{BlocksIndefinitely, NonBlocking};
    let expected = [
        (Provider::LegacyJms, NonBlocking),
        (Provider::RabbitMq, NonBlocking),
        (Provider::Nats, BlocksIndefinitely),
        (Provider::NatsJetStream, BlocksIndefinitely),
        (Provider::Kafka, BlocksIndefinitely),
        (Provider::Pulsar, BlocksIndefinitely),
    ];
    for (provider, semantics) in expected {
        assert_eq!(
            provider.guarantees().receive_semantics,
            semantics,
            "{provider:?} declares the wrong receive semantics - see docs/BUGS.md B-010"
        );
    }
}

/// LEGACY_JMS is the transparent-mode fixture the Java 6 tests drive, and its
/// non-blocking behaviour is observable here rather than merely declared.
#[test]
fn the_in_process_transport_really_does_return_none_when_empty() {
    let transport = InMemoryTransport::new(Provider::LegacyJms);
    assert!(
        transport
            .receive()
            .expect("receive must not fail")
            .is_none(),
        "LEGACY_JMS must return Ok(None) on an empty queue, as its guarantee declares"
    );
    assert_eq!(
        Provider::LegacyJms.guarantees().receive_semantics,
        ReceiveSemantics::NonBlocking,
        "the declaration must match the behaviour just observed"
    );
}

#[test]
fn blocking_and_non_blocking_render_distinctly_for_the_java_boundary() {
    assert_eq!(ReceiveSemantics::NonBlocking.as_str(), "NON_BLOCKING");
    assert_eq!(
        ReceiveSemantics::BlocksIndefinitely.as_str(),
        "BLOCKS_INDEFINITELY"
    );
}

// These states cannot be produced through a public constructor: they model a broker
// resource that disappeared after connect (or a stale receipt from another provider).
// Each operation must fail closed before dereferencing the absent provider handle.
#[cfg(feature = "nats")]
#[test]
fn nats_refuses_operations_when_runtime_client_or_subscription_is_absent() {
    let missing_runtime = super::NatsTransport {
        client: None,
        subject: "orders".to_string(),
        subscription: std::sync::Mutex::new(None),
        acknowledged: std::sync::Mutex::new(std::collections::BTreeSet::new()),
        runtime: None,
    };
    assert!(missing_runtime
        .publish(message())
        .unwrap_err()
        .to_string()
        .contains("runtime"));
    assert!(missing_runtime
        .receive()
        .unwrap_err()
        .to_string()
        .contains("subscription"));
    assert!(missing_runtime
        .acknowledge(&receipt_for(Provider::Kafka))
        .unwrap_err()
        .to_string()
        .contains("does not match NATS"));
    poison_mutex(&missing_runtime.acknowledged);
    assert!(missing_runtime
        .acknowledge(&receipt_for(Provider::Nats))
        .unwrap_err()
        .to_string()
        .contains("acknowledgement state is unavailable"));

    let missing_client = super::NatsTransport {
        client: None,
        subject: "orders".to_string(),
        subscription: std::sync::Mutex::new(None),
        acknowledged: std::sync::Mutex::new(std::collections::BTreeSet::new()),
        runtime: Some(test_runtime()),
    };
    assert!(missing_client
        .publish(message())
        .unwrap_err()
        .to_string()
        .contains("client"));
}

#[cfg(feature = "nats")]
#[test]
fn jetstream_refuses_operations_when_runtime_context_stream_or_receipt_is_absent() {
    let missing_runtime = super::NatsJetStreamTransport {
        client: None,
        context: None,
        stream: std::sync::Mutex::new(None),
        pending_acknowledgements: std::sync::Mutex::new(std::collections::BTreeMap::new()),
        subject: "orders".to_string(),
        runtime: None,
    };
    assert!(missing_runtime
        .publish(message())
        .unwrap_err()
        .to_string()
        .contains("runtime"));
    assert!(missing_runtime
        .receive()
        .unwrap_err()
        .to_string()
        .contains("stream"));
    assert!(missing_runtime
        .acknowledge(&receipt_for(Provider::Nats))
        .unwrap_err()
        .to_string()
        .contains("does not match NATS JetStream"));
    assert!(missing_runtime
        .acknowledge(&receipt_for(Provider::NatsJetStream))
        .unwrap_err()
        .to_string()
        .contains("no pending JetStream acknowledgement"));
    poison_mutex(&missing_runtime.pending_acknowledgements);
    assert!(missing_runtime
        .acknowledge(&receipt_for(Provider::NatsJetStream))
        .unwrap_err()
        .to_string()
        .contains("acknowledgement store is unavailable"));

    let missing_context = super::NatsJetStreamTransport {
        client: None,
        context: None,
        stream: std::sync::Mutex::new(None),
        pending_acknowledgements: std::sync::Mutex::new(std::collections::BTreeMap::new()),
        subject: "orders".to_string(),
        runtime: Some(test_runtime()),
    };
    assert!(missing_context
        .publish(message())
        .unwrap_err()
        .to_string()
        .contains("context"));
}

#[cfg(feature = "rabbitmq")]
#[test]
fn rabbitmq_refuses_operations_when_runtime_channel_or_receipt_is_absent() {
    let missing_runtime = super::RabbitMqTransport {
        connection: None,
        channel: None,
        runtime: None,
        queue: "orders".to_string(),
        pending_acknowledgements: std::sync::Mutex::new(std::collections::BTreeMap::new()),
    };
    assert!(missing_runtime
        .publish(message())
        .unwrap_err()
        .to_string()
        .contains("runtime"));
    assert!(missing_runtime
        .receive()
        .unwrap_err()
        .to_string()
        .contains("runtime"));
    assert!(missing_runtime
        .acknowledge(&receipt_for(Provider::Kafka))
        .unwrap_err()
        .to_string()
        .contains("does not match RabbitMQ"));
    assert!(missing_runtime
        .acknowledge(&receipt_for(Provider::RabbitMq))
        .unwrap_err()
        .to_string()
        .contains("no pending RabbitMQ acknowledgement"));
    poison_mutex(&missing_runtime.pending_acknowledgements);
    assert!(missing_runtime
        .acknowledge(&receipt_for(Provider::RabbitMq))
        .unwrap_err()
        .to_string()
        .contains("acknowledgement store is unavailable"));

    let missing_channel = super::RabbitMqTransport {
        connection: None,
        channel: None,
        runtime: Some(test_runtime()),
        queue: "orders".to_string(),
        pending_acknowledgements: std::sync::Mutex::new(std::collections::BTreeMap::new()),
    };
    assert!(missing_channel
        .publish(message())
        .unwrap_err()
        .to_string()
        .contains("channel"));
    assert!(missing_channel
        .receive()
        .unwrap_err()
        .to_string()
        .contains("channel"));
}

#[cfg(feature = "kafka")]
#[test]
fn kafka_refuses_operations_when_runtime_clients_or_receipt_is_absent() {
    let missing_runtime = super::KafkaTransport {
        producer: None,
        consumer: None,
        runtime: None,
        topic: "orders".to_string(),
        pending_acknowledgements: std::sync::Mutex::new(std::collections::BTreeMap::new()),
    };
    assert!(missing_runtime
        .publish(message())
        .unwrap_err()
        .to_string()
        .contains("runtime"));
    assert!(missing_runtime
        .receive()
        .unwrap_err()
        .to_string()
        .contains("runtime"));
    assert!(missing_runtime
        .acknowledge(&receipt_for(Provider::Nats))
        .unwrap_err()
        .to_string()
        .contains("does not match Kafka"));
    assert!(missing_runtime
        .acknowledge(&receipt_for(Provider::Kafka))
        .unwrap_err()
        .to_string()
        .contains("no pending Kafka acknowledgement"));
    poison_mutex(&missing_runtime.pending_acknowledgements);
    assert!(missing_runtime
        .acknowledge(&receipt_for(Provider::Kafka))
        .unwrap_err()
        .to_string()
        .contains("acknowledgement store is unavailable"));

    let missing_clients = super::KafkaTransport {
        producer: None,
        consumer: None,
        runtime: Some(test_runtime()),
        topic: "orders".to_string(),
        pending_acknowledgements: std::sync::Mutex::new(std::collections::BTreeMap::new()),
    };
    assert!(missing_clients
        .publish(message())
        .unwrap_err()
        .to_string()
        .contains("producer"));
    assert!(missing_clients
        .receive()
        .unwrap_err()
        .to_string()
        .contains("consumer"));
}

#[cfg(any(
    feature = "nats",
    feature = "rabbitmq",
    feature = "kafka",
    feature = "pulsar"
))]
fn live_name(prefix: &str) -> String {
    format!("{}_{}", prefix, uuid::Uuid::new_v4().simple())
}

#[cfg(feature = "nats")]
#[test]
#[ignore = "coverage fault injection requires a live NATS broker"]
fn live_coverage_nats_resource_loss_fails_closed() {
    let url = std::env::var("MODERNLINK_NATS_URL")
        .unwrap_or_else(|_| "nats://127.0.0.1:4222".to_string());
    let mut transport = super::NatsTransport::connect(&url, &live_name("modernlink_cov_nats"))
        .expect("coverage NATS connection");
    transport.runtime.take();

    assert!(transport
        .publish(message())
        .unwrap_err()
        .to_string()
        .contains("runtime"));
    assert!(transport
        .receive()
        .unwrap_err()
        .to_string()
        .contains("runtime"));
}

#[cfg(feature = "nats")]
#[test]
#[ignore = "coverage fault injection requires live NATS JetStream"]
fn live_coverage_jetstream_poisoned_ack_store_fails_closed() {
    let url = std::env::var("MODERNLINK_NATS_URL")
        .unwrap_or_else(|_| "nats://127.0.0.1:4222".to_string());
    let subject = live_name("modernlink_cov_js");
    let stream = format!("{}_STREAM", subject.to_uppercase());
    let consumer = format!("{}_CONSUMER", subject.to_uppercase());
    let transport = super::NatsJetStreamTransport::connect(&url, &subject, &stream, &consumer)
        .expect("coverage JetStream connection");
    transport
        .publish(message())
        .expect("coverage JetStream publish");
    poison_mutex(&transport.pending_acknowledgements);

    assert!(transport
        .receive()
        .unwrap_err()
        .to_string()
        .contains("acknowledgement store is unavailable"));
}

#[cfg(feature = "rabbitmq")]
#[test]
#[ignore = "coverage fault injection requires a live RabbitMQ broker"]
fn live_coverage_rabbitmq_poisoned_ack_store_fails_closed() {
    let uri = std::env::var("MODERNLINK_RABBITMQ_URL")
        .unwrap_or_else(|_| "amqp://guest:guest@127.0.0.1:5672/%2f".to_string());
    let transport = super::RabbitMqTransport::connect(&uri, &live_name("modernlink.cov.rabbit"))
        .expect("coverage RabbitMQ connection");
    transport
        .publish(message())
        .expect("coverage RabbitMQ publish");
    poison_mutex(&transport.pending_acknowledgements);

    assert!(transport
        .receive()
        .unwrap_err()
        .to_string()
        .contains("acknowledgement store is unavailable"));
}

#[cfg(feature = "kafka")]
#[test]
#[ignore = "coverage fault injection requires a live Kafka broker"]
fn live_coverage_kafka_poisoned_ack_store_fails_closed() {
    let brokers =
        std::env::var("MODERNLINK_KAFKA_BROKERS").unwrap_or_else(|_| "127.0.0.1:9092".to_string());
    let topic = live_name("modernlink_cov_kafka");
    let transport = super::KafkaTransport::connect(&brokers, &topic, &live_name("cov_group"))
        .expect("coverage Kafka connection");
    transport
        .publish(message())
        .expect("coverage Kafka publish");
    poison_mutex(&transport.pending_acknowledgements);

    assert!(transport
        .receive()
        .unwrap_err()
        .to_string()
        .contains("acknowledgement store is unavailable"));
}

#[cfg(feature = "pulsar")]
#[test]
#[ignore = "coverage fault injection requires a live Pulsar broker"]
fn live_coverage_pulsar_poisoned_ack_store_fails_closed() {
    let url = std::env::var("MODERNLINK_PULSAR_URL")
        .unwrap_or_else(|_| "pulsar://127.0.0.1:6650".to_string());
    let topic = format!(
        "persistent://public/default/{}",
        live_name("modernlink_cov_pulsar")
    );
    let transport = super::PulsarTransport::connect(&url, &topic, &live_name("cov_subscription"))
        .expect("coverage Pulsar connection");
    transport
        .publish(message())
        .expect("coverage Pulsar publish");
    poison_mutex(&transport.pending_acknowledgements);

    assert!(transport
        .receive()
        .unwrap_err()
        .to_string()
        .contains("acknowledgement store is unavailable"));
}
