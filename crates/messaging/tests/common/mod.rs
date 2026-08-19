//! Shared harness for the broker-backed tests (VER-02).
//!
//! Extracted from `broker_backed.rs` when Kafka and Pulsar gained their own test
//! targets. Each provider group is a separate target because `required-features` is
//! all-or-nothing: keeping them in one file would force a CI job that only stands up
//! NATS and RabbitMQ to also compile librdkafka and pulsar.
//!
//! The assertions are unchanged from the versions that passed against live NATS,
//! JetStream and RabbitMQ on 2026-08-14.

#![allow(dead_code)] // each test target uses a subset of these helpers

use messaging::{DeliveryState, MessageEnvelope, MessageTransport, Payload, Provider};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn endpoint(variable: &str, fallback: &str) -> String {
    std::env::var(variable).unwrap_or_else(|_| fallback.to_string())
}

/// A destination unique to this run, so a leftover message from an earlier run
/// cannot make a broken transport look like it works.
pub fn unique_destination(prefix: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is before the epoch")
        .as_nanos();
    format!("{}_{}", prefix, nanos)
}

pub fn text_envelope(destination: &str, body: &str) -> MessageEnvelope {
    MessageEnvelope::new(destination, Payload::Text(body.to_string()), 0)
        .expect("envelope construction must succeed for a non-empty destination")
}

/// Poll `receive` for up to ~10s. Brokers deliver asynchronously; a single immediate
/// poll would be flaky in a way that hides real breakage behind a retry-less failure.
pub fn receive_with_deadline<T: MessageTransport>(transport: &T) -> messaging::ReceivedMessage {
    for _ in 0..100 {
        match transport.receive() {
            Ok(Some(received)) => return received,
            Ok(None) => std::thread::sleep(std::time::Duration::from_millis(100)),
            Err(error) => panic!("receive failed against a live broker: {error}"),
        }
    }
    panic!("no message arrived within 10s");
}

/// The shared contract every provider must honour: what goes in comes out intact,
/// and the acknowledgement round trip reports the right states.
pub fn assert_send_receive_ack<T: MessageTransport>(
    transport: &T,
    provider: Provider,
    destination: &str,
    body: &str,
) {
    assert_eq!(
        transport.provider(),
        provider,
        "transport reports the wrong provider"
    );

    let sent = text_envelope(destination, body);
    let expected_id = sent.message_id.clone();
    let expected_trace = sent.tracing.trace_id.clone();
    let expected_destination = sent.destination.clone();

    let publish_receipt = transport
        .publish(sent)
        .expect("publish to a live broker failed");
    assert_eq!(publish_receipt.state, DeliveryState::Published);
    assert_eq!(publish_receipt.provider, provider);
    assert_eq!(publish_receipt.message_id, expected_id);
    assert_eq!(
        publish_receipt.trace_id, expected_trace,
        "trace id was not preserved through publish"
    );

    let received = receive_with_deadline(transport);
    assert_eq!(
        received.message.message_id, expected_id,
        "message id changed in flight"
    );
    assert_eq!(
        received.message.destination, expected_destination,
        "destination changed in flight"
    );
    assert_eq!(
        received.message.payload,
        Payload::Text(body.to_string()),
        "payload changed in flight"
    );
    assert_eq!(
        received.message.tracing.trace_id, expected_trace,
        "trace context was not preserved across the broker"
    );
    assert_eq!(received.receipt.state, DeliveryState::Received);

    let acknowledged = transport
        .acknowledge(&received.receipt)
        .expect("acknowledge against a live broker failed");
    assert_eq!(acknowledged.state, DeliveryState::Acknowledged);
    assert_eq!(acknowledged.message_id, expected_id);
}
