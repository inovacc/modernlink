use super::*;

#[cfg(test)]
mod feature_gating_tests {
    use super::*;

    /// SC-07 fail-closed contract.
    ///
    /// AGENTS.md: "Fail closed on unsupported guarantees. A capability gap must be
    /// reported explicitly - never silently degraded." Feature-gating the providers
    /// created exactly such a gap, so this asserts the gap is *reported*: asking for a
    /// provider that was compiled out must return an error that names the provider and
    /// the cargo feature which restores it.
    ///
    /// The dangerous failure this guards against is not an error - it is a *success*
    /// on some other transport. A legacy application that asks for Kafka and silently
    /// gets an in-memory queue would report every publish as delivered.
    #[cfg(not(feature = "kafka"))]
    #[test]
    fn kafka_is_refused_when_it_was_not_compiled_in() {
        // `expect_err` needs `T: Debug` and MessageTransportKind holds broker handles
        // that are not Debug, so the refusal is matched explicitly.
        let error = match build_transport(Provider::Kafka, "localhost:9092", "orders") {
            Ok(_) => panic!("Kafka was compiled out but build_transport returned a transport"),
            Err(error) => error,
        };
        assert!(error.contains("KAFKA"), "must name the provider: {}", error);
        assert!(
            error.contains("`kafka` cargo feature"),
            "must name the feature that restores it: {}",
            error
        );
        assert!(
            error.contains("refused rather than routed to a different provider"),
            "must state that no fallback happened: {}",
            error
        );
    }

    #[cfg(not(feature = "nats"))]
    #[test]
    fn both_nats_variants_are_refused_when_nats_was_not_compiled_in() {
        for provider in [Provider::Nats, Provider::NatsJetStream] {
            let error = match build_transport(provider, "127.0.0.1:4222", "orders") {
                Ok(_) => panic!("NATS was compiled out but build_transport returned a transport"),
                Err(error) => error,
            };
            assert!(error.contains("`nats` cargo feature"), "{}", error);
        }
    }

    /// LEGACY_JMS is in-process and depends on no provider crate, so it must keep
    /// working in the broker-free default build - otherwise gating would have broken
    /// the transparent-mode path the Java 6 fixtures rely on.
    #[test]
    fn legacy_jms_is_available_in_every_feature_configuration() {
        let transport = match build_transport(Provider::LegacyJms, "", "orders") {
            Ok(transport) => transport,
            Err(error) => panic!("LEGACY_JMS must always build, got: {}", error),
        };
        assert_eq!(transport.provider(), Provider::LegacyJms);
    }
}

#[cfg(test)]
mod payload_category_tests {
    use super::*;
    use std::collections::BTreeMap;

    fn round_trip(payload: Payload) -> Payload {
        let kind = messaging_payload_kind_name(&payload);
        let bytes = messaging_payload_bytes(&payload);
        messaging_build_payload(kind, bytes).expect("a supported category must round-trip")
    }

    #[test]
    fn text_round_trips() {
        assert_eq!(
            round_trip(Payload::Text("hello".to_string())),
            Payload::Text("hello".to_string())
        );
    }

    /// The reason BYTES needed the category field at all: these bytes are not valid
    /// UTF-8, so a text-only boundary would have had to mangle or reject them.
    #[test]
    fn arbitrary_bytes_survive_intact() {
        let raw = vec![0x00, 0xff, 0xfe, 0x41, 0x0a, 0x80];
        assert_eq!(round_trip(Payload::Bytes(raw.clone())), Payload::Bytes(raw));
    }

    #[test]
    fn map_round_trips() {
        let mut entries = BTreeMap::new();
        entries.insert("alpha".to_string(), "one".to_string());
        entries.insert("beta".to_string(), "two".to_string());
        assert_eq!(
            round_trip(Payload::Map(entries.clone())),
            Payload::Map(entries)
        );
    }

    /// Both halves of every pair are base64 precisely so a key or value containing the
    /// delimiters cannot forge a pair boundary. Without that, this map would decode into
    /// a different map -- silent corruption rather than an error.
    #[test]
    fn map_keys_and_values_containing_delimiters_round_trip() {
        let mut entries = BTreeMap::new();
        entries.insert(
            "key=with,delimiters".to_string(),
            "value,with=both".to_string(),
        );
        entries.insert("plain".to_string(), String::new());
        assert_eq!(
            round_trip(Payload::Map(entries.clone())),
            Payload::Map(entries)
        );
    }

    #[test]
    fn empty_map_round_trips() {
        assert_eq!(
            round_trip(Payload::Map(BTreeMap::new())),
            Payload::Map(BTreeMap::new())
        );
    }

    /// STREAM is refused rather than delivered as opaque bytes, which would drop the
    /// typed field structure a StreamMessage exists to carry.
    #[test]
    fn stream_is_refused_with_a_reason() {
        let error = messaging_build_payload("STREAM", vec![1, 2, 3])
            .expect_err("STREAM must be refused, not degraded to BYTES");
        assert!(error.contains("typed field ordering"), "{}", error);
    }

    /// The security-relevant one. Reconstructing an ObjectMessage means deserializing
    /// broker-supplied bytes into Java objects, which is a remote-code-execution surface.
    /// It must refuse, and the refusal must say why so nobody "helpfully" enables it.
    #[test]
    fn object_is_refused_and_says_why() {
        let error = messaging_build_payload("OBJECT", vec![0xac, 0xed])
            .expect_err("OBJECT must be refused");
        assert!(
            error.contains("remote-code-execution"),
            "the refusal must name the risk: {}",
            error
        );
    }

    #[test]
    fn an_unknown_category_is_refused_rather_than_guessed() {
        let error = messaging_build_payload("SOMETHING_NEW", vec![1])
            .expect_err("an unknown category must not be guessed at");
        assert!(error.contains("unknown payload category"), "{}", error);
    }

    #[test]
    fn invalid_utf8_is_refused_for_text() {
        let error = messaging_build_payload("TEXT", vec![0xff, 0xfe])
            .expect_err("invalid UTF-8 must not be silently replaced");
        assert!(error.contains("not valid UTF-8"), "{}", error);
    }

    #[test]
    fn every_payload_category_has_a_stable_wire_name_and_byte_encoding() {
        let mut entries = BTreeMap::new();
        entries.insert("key".to_string(), "value".to_string());
        let cases = [
            (Payload::Text("text".to_string()), "TEXT", b"text".to_vec()),
            (Payload::Bytes(vec![0, 255]), "BYTES", vec![0, 255]),
            (
                Payload::Map(entries.clone()),
                "MAP",
                messaging_encode_map(&entries),
            ),
            (Payload::Stream(vec![1, 2]), "STREAM", vec![1, 2]),
            (
                Payload::Object {
                    content_type: "application/example".to_string(),
                    bytes: vec![3, 4],
                },
                "OBJECT",
                vec![3, 4],
            ),
        ];

        for (payload, kind, bytes) in cases {
            assert_eq!(messaging_payload_kind_name(&payload), kind);
            assert_eq!(messaging_payload_bytes(&payload), bytes);
        }
    }

    #[test]
    fn malformed_map_frames_are_rejected_without_guessing() {
        let cases = [
            (b"YQ==".to_vec(), "no value"),
            (b"not-base64:Yg==".to_vec(), "Invalid symbol"),
            (vec![0xff], "not valid UTF-8"),
            (
                format!("{}:Yg==", modernlink_core::base64_encode(&[0xff])).into_bytes(),
                "map key is not valid UTF-8",
            ),
            (
                format!("YQ==:{}", modernlink_core::base64_encode(&[0xff])).into_bytes(),
                "map value is not valid UTF-8",
            ),
        ];

        for (bytes, reason) in cases {
            let error = messaging_decode_map(&bytes).expect_err("malformed map must fail");
            assert!(error.contains(reason), "expected {reason:?}, got {error:?}");
        }
    }
}

#[cfg(test)]
mod messaging_helper_tests {
    use super::*;

    #[test]
    fn supported_messaging_modes_parse_from_wire_names() {
        let cases = [
            ("TRANSPARENT", Mode::Transparent),
            ("TRANSFORM", Mode::Transform),
            ("REDIRECT", Mode::Redirect),
        ];
        for (name, expected) in cases {
            assert_eq!(messaging_mode(name), Ok(expected));
        }
    }

    #[test]
    fn unsupported_messaging_mode_is_rejected_exactly() {
        assert_eq!(
            messaging_mode("transparent"),
            Err("unsupported messaging mode: transparent".to_string())
        );
        assert_eq!(
            messaging_mode(""),
            Err("unsupported messaging mode: ".to_string())
        );
    }

    #[test]
    fn supported_providers_and_acknowledgements_parse_from_wire_names() {
        let providers = [
            ("LEGACY_JMS", Provider::LegacyJms),
            ("KAFKA", Provider::Kafka),
            ("PULSAR", Provider::Pulsar),
            ("NATS", Provider::Nats),
            ("NATS_JETSTREAM", Provider::NatsJetStream),
            ("RABBITMQ", Provider::RabbitMq),
        ];
        for (name, expected) in providers {
            assert_eq!(messaging_provider(name), Ok(expected));
        }

        let acknowledgements = [
            ("AUTO", AcknowledgementMode::Auto),
            ("CLIENT", AcknowledgementMode::Client),
            ("DUPLICATE_OK", AcknowledgementMode::DuplicateOk),
            ("TRANSACTED", AcknowledgementMode::Transacted),
        ];
        for (name, expected) in acknowledgements {
            assert_eq!(messaging_acknowledgement(name), Ok(expected));
        }
    }

    #[test]
    fn unsupported_provider_and_acknowledgement_are_rejected() {
        assert_eq!(
            messaging_provider("AMQP"),
            Err("unsupported messaging provider: AMQP".to_string())
        );
        assert_eq!(
            messaging_acknowledgement("MANUAL"),
            Err("unsupported acknowledgement mode: MANUAL".to_string())
        );
    }

    #[test]
    fn messaging_names_cover_every_provider_and_delivery_state() {
        let modes = [
            (Mode::Transparent, "TRANSPARENT"),
            (Mode::Transform, "TRANSFORM"),
            (Mode::Redirect, "REDIRECT"),
        ];
        for (mode, name) in modes {
            assert_eq!(messaging_mode_name(mode), name);
        }

        let providers = [
            (Provider::LegacyJms, "LEGACY_JMS"),
            (Provider::Kafka, "KAFKA"),
            (Provider::Pulsar, "PULSAR"),
            (Provider::Nats, "NATS"),
            (Provider::NatsJetStream, "NATS_JETSTREAM"),
            (Provider::RabbitMq, "RABBITMQ"),
        ];
        for (provider, name) in providers {
            assert_eq!(messaging_provider_name(provider), name);
        }

        let states = [
            (DeliveryState::Published, "PUBLISHED"),
            (DeliveryState::Received, "RECEIVED"),
            (DeliveryState::Acknowledged, "ACKNOWLEDGED"),
            (DeliveryState::Rejected, "REJECTED"),
            (DeliveryState::Retried, "RETRIED"),
            (DeliveryState::DeadLettered, "DEAD_LETTERED"),
        ];
        for (state, name) in states {
            assert_eq!(messaging_state_name(state), name);
        }
    }

    #[test]
    fn route_rule_encoding_accepts_all_optional_fields() {
        let rule = parse_route_rule("rule-1|orders|ord|tenant-a|x-request-id|abc|REDIRECT|KAFKA|0")
            .expect("a complete route rule should parse");
        assert_eq!(
            rule,
            RouteRule {
                id: "rule-1".to_string(),
                destination: Some("orders".to_string()),
                destination_prefix: Some("ord".to_string()),
                tenant: Some("tenant-a".to_string()),
                header_name: Some("x-request-id".to_string()),
                header_value: Some("abc".to_string()),
                mode: Mode::Redirect,
                provider: Provider::Kafka,
                allowed: false,
            }
        );
    }

    #[test]
    fn route_rule_encoding_accepts_empty_optional_fields() {
        let rule = parse_route_rule("rule-2||||||TRANSFORM|PULSAR|1")
            .expect("empty optional fields should mean unconstrained");
        assert_eq!(rule.id, "rule-2");
        assert_eq!(rule.destination, None);
        assert_eq!(rule.destination_prefix, None);
        assert_eq!(rule.tenant, None);
        assert_eq!(rule.header_name, None);
        assert_eq!(rule.header_value, None);
        assert_eq!(rule.mode, Mode::Transform);
        assert_eq!(rule.provider, Provider::Pulsar);
    }

    #[test]
    fn malformed_route_rules_are_rejected_fail_closed() {
        let cases = [
            ("||||||||", "id must not be empty"),
            (
                "only|eight|fields|are|not|enough|TRANSFORM|KAFKA",
                "9 pipe-delimited",
            ),
            (
                "r||||name||TRANSFORM|KAFKA|1",
                "both header name and header value",
            ),
            ("r||||||TRANSFORM|KAFKA|maybe", "invalid allowed flag"),
            ("r||||||NOPE|KAFKA|1", "unsupported messaging mode"),
            ("r||||||TRANSFORM|NOPE|1", "unsupported messaging provider"),
        ];
        for (encoded, reason) in cases {
            let error = parse_route_rule(encoded).expect_err("malformed route rule must fail");
            assert!(error.contains(reason), "expected {reason:?}, got {error:?}");
        }
    }

    #[test]
    fn error_helpers_set_last_error_and_return_their_sentinels() {
        assert_eq!(messaging_error("numeric failure".to_string()), 0);
        assert_eq!(
            LAST_ERROR.with(|value| value.borrow().clone()),
            "numeric failure"
        );

        assert!(messaging_string_error("string failure".to_string()).is_null());
        assert_eq!(
            LAST_ERROR.with(|value| value.borrow().clone()),
            "string failure"
        );
    }

    #[cfg(any(
        feature = "nats",
        feature = "kafka",
        feature = "pulsar",
        feature = "rabbitmq"
    ))]
    #[test]
    fn broker_subject_names_are_uppercase_and_delimiter_safe() {
        assert_eq!(
            jetstream_name("orders.foo-1", "STREAM"),
            "MODERNLINK_ORDERS_FOO_1_STREAM"
        );
    }

    #[cfg(feature = "kafka")]
    #[test]
    fn kafka_group_name_uses_the_same_subject_normalization() {
        assert_eq!(
            kafka_group("orders.foo-1"),
            "MODERNLINK_ORDERS_FOO_1_KAFKA_GROUP"
        );
    }

    #[cfg(not(all(
        feature = "nats",
        feature = "kafka",
        feature = "pulsar",
        feature = "rabbitmq"
    )))]
    #[test]
    fn disabled_provider_errors_name_the_missing_feature() {
        #[cfg(not(feature = "nats"))]
        assert!(provider_disabled(Provider::Nats, "nats").contains("`nats` cargo feature"));
        #[cfg(not(feature = "kafka"))]
        assert!(provider_disabled(Provider::Kafka, "kafka").contains("`kafka` cargo feature"));
        #[cfg(not(feature = "pulsar"))]
        assert!(provider_disabled(Provider::Pulsar, "pulsar").contains("`pulsar` cargo feature"));
        #[cfg(not(feature = "rabbitmq"))]
        assert!(
            provider_disabled(Provider::RabbitMq, "rabbitmq").contains("`rabbitmq` cargo feature")
        );
    }
}

#[cfg(test)]
mod messaging_frame_tests {
    use super::*;

    fn message(payload: Payload, acknowledgement_mode: AcknowledgementMode) -> MessageEnvelope {
        let mut message =
            MessageEnvelope::new("orders", payload, 123).expect("test destination is valid");
        message.message_id = "message-1".to_string();
        message.acknowledgement_mode = acknowledgement_mode;
        message.tracing = TraceContext {
            trace_id: "trace-1".to_string(),
            span_id: "span-1".to_string(),
            parent_span_id: Some("parent-1".to_string()),
            trace_state: Some("state-1".to_string()),
            sampled: true,
        };
        message
    }

    fn receipt(state: DeliveryState) -> DeliveryReceipt {
        DeliveryReceipt {
            message_id: "message-1".to_string(),
            provider: Provider::Nats,
            state,
            trace_id: "trace-1".to_string(),
        }
    }

    #[test]
    fn message_frame_preserves_payload_category_trace_and_receipt() {
        let frame = messaging_message_frame(
            &message(
                Payload::Text("hello".to_string()),
                AcknowledgementMode::Client,
            ),
            &receipt(DeliveryState::Received),
        )
        .expect("message framing is deterministic");
        assert_eq!(
            frame,
            "message-1|orders|aGVsbG8=|trace-1|span-1|parent-1|state-1|CLIENT|1|TEXT\nmessage-1#NATS#RECEIVED#trace-1"
        );
    }

    #[test]
    fn message_frame_names_every_acknowledgement_mode() {
        let cases = [
            (AcknowledgementMode::Auto, "AUTO"),
            (AcknowledgementMode::Client, "CLIENT"),
            (AcknowledgementMode::DuplicateOk, "DUPLICATE_OK"),
            (AcknowledgementMode::Transacted, "TRANSACTED"),
        ];
        for (acknowledgement, name) in cases {
            let frame = messaging_message_frame(
                &message(Payload::Bytes(vec![0, 255]), acknowledgement),
                &receipt(DeliveryState::Published),
            )
            .expect("message framing should succeed");
            assert!(frame.contains(&format!("|{name}|1|BYTES\n")), "{frame}");
        }
    }

    #[test]
    fn receipt_frame_names_every_delivery_state() {
        let states = [
            DeliveryState::Published,
            DeliveryState::Received,
            DeliveryState::Acknowledged,
            DeliveryState::Rejected,
            DeliveryState::Retried,
            DeliveryState::DeadLettered,
        ];
        for state in states {
            let frame = messaging_receipt_frame(&receipt(state));
            assert!(frame.starts_with("message-1#NATS#"), "{frame}");
            assert!(frame.ends_with("#trace-1"), "{frame}");
        }
    }
}

#[cfg(test)]
mod panic_containment_tests {
    use super::*;

    fn last_error() -> String {
        LAST_ERROR.with(|value| value.borrow().clone())
    }

    #[test]
    fn a_body_that_succeeds_is_passed_through_untouched() {
        assert_eq!(jni_guard(0i64, || 42i64), 42);
    }

    /// The core of B-004: a panic must become a reported error and a sentinel, not an
    /// unwind into JVM frames.
    #[test]
    fn a_panicking_body_returns_the_sentinel_instead_of_unwinding() {
        LAST_ERROR.with(|value| value.borrow_mut().clear());
        let result = jni_guard(0i64, || panic!("boom from inside an entry point"));
        assert_eq!(result, 0, "the caller must receive the error sentinel");
        let error = last_error();
        assert!(
            error.contains("boom from inside an entry point"),
            "the panic detail must survive into the error channel: {}",
            error
        );
        assert!(
            error.contains("did not reach the JVM"),
            "the message must say the panic was contained: {}",
            error
        );
    }

    /// Null is the sentinel for every object return, and it is what the Java side already
    /// treats as "ask nativeLastError why".
    #[test]
    fn a_null_sentinel_is_returned_for_object_returns() {
        let result = jni_guard(std::ptr::null_mut::<u8>(), || panic!("boom"));
        assert!(result.is_null());
    }

    /// A formatted panic produces `String`, an unformatted one `&str`. Both must render;
    /// this caught a real gap where only one arm was handled.
    #[test]
    fn both_string_and_str_panic_payloads_render() {
        LAST_ERROR.with(|value| value.borrow_mut().clear());
        jni_guard(0i64, || panic!("plain str payload"));
        assert!(last_error().contains("plain str payload"));

        LAST_ERROR.with(|value| value.borrow_mut().clear());
        let n = 7;
        jni_guard(0i64, || panic!("formatted {} payload", n));
        assert!(last_error().contains("formatted 7 payload"));
    }

    /// Handling a panic must never itself panic - that aborts the process, which is worse
    /// than the undefined behaviour this guard exists to prevent.
    #[test]
    fn an_unknown_payload_type_does_not_panic_the_handler() {
        LAST_ERROR.with(|value| value.borrow_mut().clear());
        let result = jni_guard(0i64, || std::panic::panic_any(42u32));
        assert_eq!(result, 0);
        assert!(
            last_error().contains("non-string panic payload"),
            "{}",
            last_error()
        );
    }

    /// The structural guarantee, not just the mechanism: EVERY exported entry point must be
    /// wrapped. Counting them in the source is crude, and it is the only check that fails
    /// when someone adds a 29th entry point and forgets the guard - which is exactly how
    /// this defect would come back.
    #[test]
    fn every_exported_entry_point_is_wrapped_in_the_guard() {
        let source = include_str!("lib.rs");
        let exported = source.matches("pub extern \"system\" fn Java_").count();
        let guarded = source.matches("jni_guard(").count();
        // Unit bodies live in tests.rs, and the generic `fn jni_guard<F, T>` definition
        // does not contain the literal `jni_guard(`. Every remaining match in lib.rs must
        // therefore be an entry-point call site.
        assert!(
            exported > 0,
            "no entry points found - the check itself is broken"
        );
        assert_eq!(
            guarded, exported,
            "every `Java_*` entry point must call jni_guard: {} exported, {} guarded call \
             sites. An unguarded entry point can unwind \
             a panic into the JVM - see docs/BUGS.md B-004.",
            exported, guarded
        );
    }
}

#[cfg(test)]
mod handle_registry_tests {
    use super::*;

    fn a_client() -> NativeMessagingClient {
        NativeMessagingClient {
            transport: MessageTransportKind::LegacyJms(InMemoryTransport::new(Provider::LegacyJms)),
            route: RouteConfig {
                default_mode: Mode::Transparent,
                default_provider: Provider::LegacyJms,
                rules: Vec::new(),
            },
        }
    }

    /// The defect B-005 named: a handle the registry never issued must be a miss, not a
    /// dereference. Against the old code this address would have been cast to a pointer and
    /// read.
    #[test]
    fn a_fabricated_handle_is_refused_rather_than_dereferenced() {
        assert!(client_for(0).is_none(), "zero is the invalid sentinel");
        assert!(
            client_for(-1).is_none(),
            "a negative handle is not an address"
        );
        assert!(
            client_for(0x7fff_dead_beef).is_none(),
            "an arbitrary value must miss, not be dereferenced"
        );
    }

    #[test]
    fn a_registered_client_is_found_and_stays_found_until_closed() {
        let handle = register_client(a_client());
        assert!(handle > 0, "a live client gets a non-zero handle");
        assert!(client_for(handle).is_some());
        assert!(
            client_for(handle).is_some(),
            "lookup does not consume the entry"
        );
        unregister_client(handle);
        assert!(client_for(handle).is_none(), "a closed handle must miss");
    }

    /// Java's close() is synchronized and zeroes its field, but nothing stops a second
    /// caller holding a copy of the long. Under the old code that was a double free.
    #[test]
    fn closing_twice_is_a_no_op_not_a_double_free() {
        let handle = register_client(a_client());
        unregister_client(handle);
        unregister_client(handle);
        unregister_client(handle);
        assert!(client_for(handle).is_none());
    }

    /// Ids are never reused, so a stale handle cannot silently address a different client -
    /// which would be worse than a miss, because the caller would get plausible results
    /// from the wrong connection.
    #[test]
    fn a_closed_handle_is_never_reissued_to_a_later_client() {
        let first = register_client(a_client());
        unregister_client(first);
        let second = register_client(a_client());
        assert_ne!(first, second, "handles must not be recycled");
        assert!(client_for(first).is_none(), "the stale handle stays dead");
        unregister_client(second);
    }

    /// An in-flight call holds an Arc, so a concurrent close cannot pull the memory out
    /// from under it - it frees when the last user finishes.
    #[test]
    fn an_in_flight_borrow_survives_a_concurrent_close() {
        let handle = register_client(a_client());
        let in_flight = client_for(handle).expect("registered");
        unregister_client(handle);
        assert!(client_for(handle).is_none(), "new lookups miss immediately");
        assert_eq!(
            in_flight.transport.provider(),
            Provider::LegacyJms,
            "the in-flight borrow is still usable"
        );
    }

    fn a_response() -> NativeResponse {
        NativeResponse(modernlink_core::Response {
            final_url: "https://example.com/".to_string(),
            status: 200,
            status_message: "OK".to_string(),
            headers: std::collections::BTreeMap::new(),
            body: Vec::new(),
            tls: None,
        })
    }

    fn a_response_with_tls() -> NativeResponse {
        NativeResponse(modernlink_core::Response {
            final_url: "https://example.com/".to_string(),
            status: 200,
            status_message: "OK".to_string(),
            headers: std::collections::BTreeMap::new(),
            body: Vec::new(),
            tls: Some(modernlink_core::TlsInfo {
                protocol: Some("TLSv1.3".to_string()),
                cipher_suite: Some("TLS_AES_128_GCM_SHA256".to_string()),
                peer_certificates_der: vec![vec![1, 2, 3]],
            }),
        })
    }

    #[test]
    fn a_fabricated_response_handle_is_refused_rather_than_dereferenced() {
        assert!(response_for(0).is_none());
        assert!(response_for(-1).is_none());
        assert!(response_for(0x7fff_dead_beef).is_none());
    }

    /// Response and client handles share one counter, so mixing them up misses in both maps
    /// instead of finding the wrong object in one - which would hand back a plausible answer
    /// from an unrelated request.
    #[test]
    fn response_and_client_handles_never_collide() {
        let client = register_client(a_client());
        let response = register_response(a_response());
        assert_ne!(
            client, response,
            "the two registries must not issue the same id"
        );
        assert!(
            response_for(client).is_none(),
            "a client handle must miss in the response map"
        );
        assert!(
            client_for(response).is_none(),
            "a response handle must miss in the client map"
        );
        unregister_client(client);
        unregister_response(response);
    }

    #[test]
    fn releasing_a_response_twice_is_a_no_op() {
        let handle = register_response(a_response());
        assert!(response_for(handle).is_some());
        unregister_response(handle);
        unregister_response(handle);
        assert!(response_for(handle).is_none());
    }

    #[test]
    fn response_tls_metadata_is_available_until_release() {
        let without_tls = register_response(a_response());
        assert_eq!(tls_string(without_tls, true), None);
        assert_eq!(tls_string(without_tls, false), None);
        unregister_response(without_tls);

        let handle = register_response(a_response_with_tls());
        assert_eq!(tls_string(handle, true), Some("TLSv1.3".to_string()));
        assert_eq!(
            tls_string(handle, false),
            Some("TLS_AES_128_GCM_SHA256".to_string())
        );
        unregister_response(handle);
        assert_eq!(tls_string(handle, true), None);
    }

    #[test]
    fn invalid_response_handles_are_ignored_by_lookup_metadata_and_release() {
        for handle in [0, -1, 0x7fff_dead_beef] {
            assert!(response_for(handle).is_none());
            assert_eq!(tls_string(handle, true), None);
            assert_eq!(tls_string(handle, false), None);
            unregister_response(handle);
        }
    }

    /// Structural guard: the handle must never go back to being a raw pointer.
    #[test]
    fn no_handle_is_cast_to_a_messaging_client_pointer() {
        let source = include_str!("lib.rs");
        for (kind, bug) in [
            ("NativeMessagingClient", "B-005"),
            ("NativeResponse", "B-008"),
        ] {
            for form in ["const", "mut"] {
                let needle = format!("handle as *{form} {kind}");
                assert_eq!(
                    source.matches(needle.as_str()).count(),
                    0,
                    "a handle is cast to a raw {kind} pointer - {bug}"
                );
            }
        }
        // Both handle types go through a registry now, so the crate needs no `unsafe` block
        // at all. This fails the moment one comes back, which is the only way the
        // use-after-free returns.
        // Needles built in pieces so they do not match their own source lines - the same
        // self-reference that broke the first version of this check.
        let block = concat!("unsa", "fe {");
        let function = concat!("unsa", "fe fn ");
        let unsafe_blocks = source.matches(block).count() + source.matches(function).count();
        assert_eq!(
            unsafe_blocks, 0,
            "crates/jni should contain no unsafe blocks; found {unsafe_blocks} - B-005/B-008"
        );
    }
}
