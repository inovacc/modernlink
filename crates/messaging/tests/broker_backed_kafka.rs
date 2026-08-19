//! VER-02 — broker-backed send/receive/ack for Kafka.
//!
//! A separate target from `broker_backed.rs` because `required-features` is
//! all-or-nothing: folding Kafka into that file would force every job that stands up
//! only NATS and RabbitMQ to compile librdkafka from source as well.
//!
//! `#[ignore]`d for the same reason as the others — `cargo test --workspace` must not
//! silently pass by skipping a test whose broker is absent. Run it explicitly:
//!
//! ```text
//! docker run -d --name ml-kafka -p 9092:9092 apache/kafka:3.8.0
//! cargo test -p messaging --test broker_backed_kafka --features kafka -- --ignored
//! ```
//!
//! **This test has never been executed.** It is written against the same contract the
//! three passing providers satisfy, but no run against a live Kafka has been recorded
//! (see `docs/BUGS.md`, "Verification reach").

mod common;
use common::{assert_send_receive_ack, endpoint, unique_destination};
use messaging::{KafkaTransport, Provider};

#[test]
#[ignore = "requires a live Kafka broker; see the module docs"]
fn kafka_send_receive_ack() {
    let brokers = endpoint("MODERNLINK_KAFKA_BROKERS", "127.0.0.1:9092");
    let destination = unique_destination("modernlink.ver02.kafka");
    // A consumer group unique to this run, so an offset committed by an earlier run
    // cannot make a broken consumer look like it received nothing legitimately.
    let group = format!("{}_group", destination);
    let transport = KafkaTransport::connect(&brokers, &destination, &group)
        .unwrap_or_else(|error| panic!("could not reach Kafka at {brokers}: {error}"));
    assert_send_receive_ack(
        &transport,
        Provider::Kafka,
        &destination,
        "ver02 kafka payload",
    );
}
