use messaging::{MessageEnvelope, MessageTransport, Payload, RabbitMqTransport};
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let uri = env::var("RABBITMQ_URI")
        .unwrap_or_else(|_| "amqp://guest:guest@127.0.0.1:5672/%2f".to_string());
    let queue = env::var("RABBITMQ_QUEUE").unwrap_or_else(|_| "modernlink.orders".to_string());
    let transport = RabbitMqTransport::connect(&uri, &queue)?;
    let message = MessageEnvelope::new(&queue, Payload::Text("rabbitmq-fixture".to_string()), 0)?;
    let published = transport.publish(message.clone())?;
    let received = transport
        .receive()?
        .ok_or("RabbitMQ queue returned no message")?;
    let acknowledged = transport.acknowledge(&received.receipt)?;
    if received.message.message_id != message.message_id {
        return Err("RabbitMQ message identity changed".into());
    }
    println!(
        "provider=RabbitMq queue={} message-id={} trace-id={} published={:?} received={:?} acknowledged={:?}",
        queue,
        message.message_id,
        message.tracing.trace_id,
        published.state,
        received.receipt.state,
        acknowledged.state
    );
    Ok(())
}
