//! VER-02 — broker-backed send/receive/ack for Pulsar.
//!
//! A separate target from `broker_backed.rs` for the same reason as Kafka: keeping it
//! there would drag the pulsar client (and protoc) into jobs that need neither.
//!
//! `#[ignore]`d so `cargo test --workspace` cannot pass by skipping it. Run it
//! explicitly:
//!
//! ```text
//! docker run -d --name ml-pulsar -p 6650:6650 apachepulsar/pulsar:3.3.0 \
//!   bin/pulsar standalone
//! cargo test -p messaging --test broker_backed_pulsar --features pulsar -- --ignored
//! ```
//!
//! Recorded command reach is listed in `docs/VERIFICATION.md`; it does not establish
//! reconnect, durability, redelivery, or vendor-host semantics.

mod common;
use common::{assert_send_receive_ack, endpoint, unique_destination};
use messaging::{DeliveryReceipt, DeliveryState, MessageTransport, Provider, PulsarTransport};

fn missing_receipt(provider: Provider) -> DeliveryReceipt {
    DeliveryReceipt {
        message_id: "missing-message".to_string(),
        provider,
        state: DeliveryState::Received,
        trace_id: "missing-trace".to_string(),
    }
}

#[test]
#[ignore = "requires a live Pulsar broker; see the module docs"]
fn pulsar_send_receive_ack() {
    let service_url = endpoint("MODERNLINK_PULSAR_URL", "pulsar://127.0.0.1:6650");
    // Pulsar topics are namespaced; an unqualified name is not addressable.
    let destination = format!(
        "persistent://public/default/{}",
        unique_destination("modernlink.ver02.pulsar")
    );
    let subscription = unique_destination("modernlink_ver02_pulsar_sub");
    let transport = PulsarTransport::connect(&service_url, &destination, &subscription)
        .unwrap_or_else(|error| panic!("could not reach Pulsar at {service_url}: {error}"));
    let wrong_provider = transport
        .acknowledge(&missing_receipt(Provider::Kafka))
        .expect_err("a receipt from another provider must fail closed");
    assert!(wrong_provider.to_string().contains("does not match Pulsar"));
    let missing = transport
        .acknowledge(&missing_receipt(Provider::Pulsar))
        .expect_err("an unknown Pulsar receipt must fail closed");
    assert!(missing
        .to_string()
        .contains("no pending Pulsar acknowledgement"));
    assert_send_receive_ack(
        &transport,
        Provider::Pulsar,
        &destination,
        "ver02 pulsar payload",
    );
}
