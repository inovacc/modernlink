//! Entry point for the cross-application contract fixtures.
//!
//! This binary does nothing itself; the fixtures are the seven `src/bin/*.rs` targets.
//! Each provider fixture is gated behind the cargo feature that supplies its transport
//! (SC-07), so naming one without its feature is a cargo error rather than a mystery.

fn main() {
    eprintln!("modernlink contract fixtures - pick one:");
    eprintln!();
    eprintln!("  cargo run --bin legacy-jms-app                              (no feature needed)");
    eprintln!("  cargo run --bin modern-provider-app                         (no feature needed)");
    eprintln!("  cargo run --bin nats-app       --features nats");
    eprintln!("  cargo run --bin jetstream-app  --features nats");
    eprintln!("  cargo run --bin kafka-app      --features kafka");
    eprintln!("  cargo run --bin pulsar-app     --features pulsar");
    eprintln!("  cargo run --bin rabbitmq-app   --features rabbitmq");
    eprintln!();
    eprintln!("The provider fixtures need a live broker; see docs/providers.md.");
}
