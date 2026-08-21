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

mod common;
use common::{assert_send_receive_ack, endpoint, unique_destination};
use messaging::{
    DeliveryReceipt, DeliveryState, MessageTransport, NatsJetStreamTransport, NatsTransport,
    Provider, RabbitMqTransport,
};

fn missing_receipt(provider: Provider) -> DeliveryReceipt {
    DeliveryReceipt {
        message_id: "missing-message".to_string(),
        provider,
        state: DeliveryState::Received,
        trace_id: "missing-trace".to_string(),
    }
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
    let error = transport
        .acknowledge(&missing_receipt(Provider::NatsJetStream))
        .expect_err("an unknown JetStream receipt must fail closed");
    assert!(error
        .to_string()
        .contains("no pending JetStream acknowledgement"));
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
    assert!(
        transport
            .receive()
            .expect("an empty RabbitMQ queue should be readable")
            .is_none(),
        "a unique queue should be empty before the test publishes"
    );
    assert_send_receive_ack(
        &transport,
        Provider::RabbitMq,
        &destination,
        "ver02 rabbitmq payload",
    );
}
