//! Illustrative application-side preflight. The JNI layer sees the same provider-neutral shape
//! regardless of whether the selected transport is Kafka, RabbitMQ, NATS, or Pulsar.
//!
//! B-003: the current publish path does not call `require_delivery_mode`; this example shows
//! the check a caller/boundary must make, not behavior the current JNI path already guarantees.

use messaging::{AcknowledgementMode, DeliveryMode, Provider, ProviderGuarantees};

fn refuse_if_contract_is_not_available(provider: Provider) -> Result<ProviderGuarantees, String> {
    let guarantees = provider.guarantees();

    // Required preflight before publishing. Until B-003 wires this into the boundary, callers
    // cannot assume the publish path performs it automatically.
    guarantees
        .require_delivery_mode(DeliveryMode::Persistent)
        .map_err(|error| error.to_string())?;
    guarantees
        .require_acknowledgement_mode(AcknowledgementMode::Client)
        .map_err(|error| error.to_string())?;

    Ok(guarantees)
}

fn choose_provider_from_java_value(value: &str) -> Result<Provider, String> {
    match value {
        "KAFKA" => Ok(Provider::Kafka),
        "RABBITMQ" => Ok(Provider::RabbitMq),
        "NATS" => Ok(Provider::Nats),
        "NATS_JETSTREAM" => Ok(Provider::NatsJetStream),
        "PULSAR" => Ok(Provider::Pulsar),
        "LEGACY_JMS" => Ok(Provider::LegacyJms),
        other => Err(format!("unsupported ModernLink provider: {}", other)),
    }
}
