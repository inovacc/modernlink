use messaging::{MessageEnvelope, MessageTransport, Payload, PulsarTransport};
use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    if let Err(error) = run() {
        eprintln!("pulsar-app error: {}", error);
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let service_url =
        std::env::var("PULSAR_URL").unwrap_or_else(|_| "pulsar://127.0.0.1:6650".to_string());
    let topic = std::env::var("PULSAR_TOPIC")
        .unwrap_or_else(|_| "persistent://public/default/modernlink.orders".to_string());
    let subscription =
        std::env::var("PULSAR_SUBSCRIPTION").unwrap_or_else(|_| "modernlink-fixture".to_string());
    let transport = PulsarTransport::connect(&service_url, &topic, &subscription)
        .map_err(|error| error.to_string())?;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_millis() as i64;
    let mut message = MessageEnvelope::new(
        &topic,
        Payload::Text("order-from-pulsar-fixture".to_string()),
        timestamp,
    )
    .map_err(|error| error.to_string())?;
    message.source = Some("pulsar-app".to_string());
    let published = transport
        .publish(message)
        .map_err(|error| error.to_string())?;
    let received = transport
        .receive()
        .map_err(|error| error.to_string())?
        .ok_or("Pulsar message was not received")?;
    let acknowledged = transport
        .acknowledge(&received.receipt)
        .map_err(|error| error.to_string())?;
    if received.receipt.message_id != published.message_id
        || acknowledged.message_id != published.message_id
    {
        return Err("Pulsar receipt identity did not remain stable".to_string());
    }
    println!(
        "provider=Pulsar service-url={} topic={} subscription={} message-id={} trace-id={} published={:?} received={:?} acknowledged={:?}",
        service_url,
        topic,
        subscription,
        received.message.message_id,
        received.message.tracing.trace_id,
        published.state,
        received.receipt.state,
        acknowledged.state
    );
    Ok(())
}
