use messaging::{KafkaTransport, MessageEnvelope, MessageTransport, Payload};
use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    if let Err(error) = run() {
        eprintln!("kafka-app error: {}", error);
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let brokers = std::env::var("KAFKA_BROKERS").unwrap_or_else(|_| "127.0.0.1:19092".to_string());
    let topic = std::env::var("KAFKA_TOPIC").unwrap_or_else(|_| "modernlink.orders".to_string());
    let group = std::env::var("KAFKA_GROUP").unwrap_or_else(|_| "modernlink-fixture".to_string());
    let transport =
        KafkaTransport::connect(&brokers, &topic, &group).map_err(|error| error.to_string())?;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_millis() as i64;
    let mut message = MessageEnvelope::new(
        &topic,
        Payload::Text("order-from-kafka-fixture".to_string()),
        timestamp,
    )
    .map_err(|error| error.to_string())?;
    message.source = Some("kafka-app".to_string());
    let published = transport
        .publish(message)
        .map_err(|error| error.to_string())?;
    let received = transport
        .receive()
        .map_err(|error| error.to_string())?
        .ok_or("Kafka message was not received")?;
    let acknowledged = transport
        .acknowledge(&received.receipt)
        .map_err(|error| error.to_string())?;
    if received.receipt.message_id != published.message_id
        || acknowledged.message_id != published.message_id
    {
        return Err("Kafka receipt identity did not remain stable".to_string());
    }
    println!(
        "provider=Kafka brokers={} topic={} group={} message-id={} trace-id={} published={:?} received={:?} acknowledged={:?}",
        brokers,
        topic,
        group,
        received.message.message_id,
        received.message.tracing.trace_id,
        published.state,
        received.receipt.state,
        acknowledged.state
    );
    Ok(())
}
