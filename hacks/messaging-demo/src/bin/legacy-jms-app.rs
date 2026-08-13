use messaging::{
    InMemoryTransport, MessageEnvelope, MessageTransport, Mode, Payload, Provider, RouteConfig,
};
use serde_json::json;
use std::time::{SystemTime, UNIX_EPOCH};

fn mode(value: &str) -> Result<Mode, String> {
    match value {
        "transparent" => Ok(Mode::Transparent),
        "transform" => Ok(Mode::Transform),
        "redirect" => Ok(Mode::Redirect),
        _ => Err(format!("unsupported mode: {}", value)),
    }
}

fn provider(value: &str) -> Result<Provider, String> {
    match value {
        "legacy-jms" => Ok(Provider::LegacyJms),
        "kafka" => Ok(Provider::Kafka),
        "pulsar" => Ok(Provider::Pulsar),
        "nats" => Ok(Provider::Nats),
        "rabbitmq" => Ok(Provider::RabbitMq),
        _ => Err(format!("unsupported provider: {}", value)),
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("legacy-jms-app error: {}", error);
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let arguments: Vec<String> = std::env::args().collect();
    let mode_name = arguments
        .get(1)
        .map(String::as_str)
        .unwrap_or("transparent");
    let provider_name = arguments.get(2).map(String::as_str).unwrap_or("legacy-jms");
    let selected_mode = mode(mode_name)?;
    let selected_provider = provider(provider_name)?;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_millis() as i64;
    let mut message = MessageEnvelope::new(
        "orders.created",
        Payload::Text("order-from-jms".to_string()),
        timestamp,
    )
    .map_err(|error| error.to_string())?;
    message.source = Some("legacy-jms-app".to_string());
    message.tenant = Some("demo".to_string());
    message
        .headers
        .insert("x-demo-mode".to_string(), mode_name.to_string());
    let config = RouteConfig {
        default_mode: selected_mode,
        default_provider: selected_provider,
        rules: Vec::new(),
    };
    let transport = InMemoryTransport::new(selected_provider);
    let dispatched = config
        .dispatch(message, &transport)
        .map_err(|error| error.to_string())?;
    let routed_message = transport
        .receive()
        .map_err(|error| error.to_string())?
        .ok_or("dispatched message was not available")?
        .message;
    let frame = json!({
        "source": "jms",
        "jmx": { "mode": mode_name, "provider": provider_name, "published": 1 },
        "route": dispatched.decision,
        "receipt": dispatched.receipt,
        "message": routed_message,
    });
    println!(
        "{}",
        serde_json::to_string(&frame).map_err(|error| error.to_string())?
    );
    Ok(())
}
