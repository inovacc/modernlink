//! VER-02 — broker-backed send/receive/ack, per provider.
//!
//! Everything in `crates/messaging` beyond `InMemoryTransport` was a source-level
//! claim: the transports existed and compiled, but no test had ever contacted a real
//! broker, so a defect in any of them would not have been detected by anything
//! (`docs/ISSUES.md` I-010).
//!
//! These tests are `#[ignore]`d, so `cargo test --workspace` does not run them and
//! does not silently pass when no broker is present. That matters: a test that skips
//! itself when its dependency is missing is a hollow gate — it reports success for
//! having done nothing. Run them explicitly against real brokers:
//!
//! ```text
//! pwsh .scripts/16-D_start_brokers.ps1
//! cargo test -p messaging --test broker_backed -- --ignored --test-threads=1
//! ```
//!
//! Endpoints come from the environment so CI service containers can override them;
//! when a broker is unreachable the test FAILS rather than skipping.

use messaging::{
    DeliveryState, MessageEnvelope, MessageTransport, NatsJetStreamTransport, NatsTransport,
    Payload, Provider, RabbitMqTransport,
};
use std::time::{SystemTime, UNIX_EPOCH};

fn endpoint(variable: &str, fallback: &str) -> String {
    std::env::var(variable).unwrap_or_else(|_| fallback.to_string())
}

/// A destination unique to this run, so a leftover message from an earlier run
/// cannot make a broken transport look like it works.
fn unique_destination(prefix: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is before the epoch")
        .as_nanos();
    format!("{}_{}", prefix, nanos)
}

fn text_envelope(destination: &str, body: &str) -> MessageEnvelope {
    MessageEnvelope::new(destination, Payload::Text(body.to_string()), 0)
        .expect("envelope construction must succeed for a non-empty destination")
}

/// Poll `receive` for up to ~10s. Brokers deliver asynchronously; a single immediate
/// poll would be flaky in a way that hides real breakage behind a retry-less failure.
fn receive_with_deadline<T: MessageTransport>(transport: &T) -> messaging::ReceivedMessage {
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
fn assert_send_receive_ack<T: MessageTransport>(
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

#[test]
#[ignore = "requires a live NATS broker; see the module docs"]
fn nats_core_send_receive_ack() {
    let url = endpoint("MODERNLINK_NATS_URL", "nats://127.0.0.1:4222");
    let destination = unique_destination("modernlink.ver02.core");
    let transport = NatsTransport::connect(&url, &destination)
        .unwrap_or_else(|error| panic!("could not reach NATS at {url}: {error}"));
    assert_send_receive_ack(
        &transport,
        Provider::Nats,
        &destination,
        "ver02 nats core payload",
    );
}

#[test]
#[ignore = "requires a live NATS broker with JetStream enabled; see the module docs"]
fn nats_jetstream_send_receive_ack() {
    let url = endpoint("MODERNLINK_NATS_URL", "nats://127.0.0.1:4222");
    let destination = unique_destination("modernlink_ver02_js");
    let stream = format!("{}_STREAM", destination.to_uppercase());
    let consumer = format!("{}_CONSUMER", destination.to_uppercase());
    let transport = NatsJetStreamTransport::connect(&url, &destination, &stream, &consumer)
        .unwrap_or_else(|error| panic!("could not reach NATS JetStream at {url}: {error}"));
    assert_send_receive_ack(
        &transport,
        Provider::NatsJetStream,
        &destination,
        "ver02 jetstream payload",
    );
}

#[test]
#[ignore = "requires a live RabbitMQ broker; see the module docs"]
fn rabbitmq_send_receive_ack() {
    let uri = endpoint(
        "MODERNLINK_RABBITMQ_URL",
        "amqp://guest:guest@127.0.0.1:5672/%2f",
    );
    let destination = unique_destination("modernlink.ver02.rabbit");
    let transport = RabbitMqTransport::connect(&uri, &destination)
        .unwrap_or_else(|error| panic!("could not reach RabbitMQ: {error}"));
    assert_send_receive_ack(
        &transport,
        Provider::RabbitMq,
        &destination,
        "ver02 rabbitmq payload",
    );
}
