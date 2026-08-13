use messaging::{MessageEnvelope, MessageTransport, NatsJetStreamTransport, Payload};
use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    if let Err(error) = run() {
        eprintln!("jetstream-app error: {}", error);
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let url = std::env::var("NATS_URL").unwrap_or_else(|_| "nats://127.0.0.1:4222".to_string());
    let subject =
        std::env::var("NATS_SUBJECT").unwrap_or_else(|_| "modernlink.jetstream.orders".to_string());
    let stream = std::env::var("NATS_STREAM").unwrap_or_else(|_| "MODERNLINK_ORDERS".to_string());
    let consumer =
        std::env::var("NATS_CONSUMER").unwrap_or_else(|_| "MODERNLINK_ORDERS_CONSUMER".to_string());
    let transport = NatsJetStreamTransport::connect(&url, &subject, &stream, &consumer)
        .map_err(|error| error.to_string())?;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_millis() as i64;
    let mut message = MessageEnvelope::new(
        &subject,
        Payload::Text("order-from-jetstream-fixture".to_string()),
        timestamp,
    )
    .map_err(|error| error.to_string())?;
    message.source = Some("jetstream-app".to_string());
    let published = transport
        .publish(message)
        .map_err(|error| error.to_string())?;
    let received = transport
        .receive()
        .map_err(|error| error.to_string())?
        .ok_or("JetStream message was not received")?;
    let acknowledged = transport
        .acknowledge(&received.receipt)
        .map_err(|error| error.to_string())?;
    if received.receipt.message_id != published.message_id
        || acknowledged.message_id != published.message_id
    {
        return Err("JetStream receipt identity did not remain stable".to_string());
    }
    println!(
        "provider=NatsJetStream stream={} consumer={} subject={} message-id={} trace-id={} published={:?} received={:?} acknowledged={:?}",
        stream,
        consumer,
        subject,
        received.message.message_id,
        received.message.tracing.trace_id,
        published.state,
        received.receipt.state,
        acknowledged.state
    );
    Ok(())
}
