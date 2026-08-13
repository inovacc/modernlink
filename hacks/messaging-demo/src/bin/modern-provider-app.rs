use messaging::{MessageEnvelope, Payload, Provider, RouteDecision};
use serde_json::Value;
use std::io::{self, Read};

fn main() {
    if let Err(error) = run() {
        eprintln!("modern-provider-app error: {}", error);
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .map_err(|error| error.to_string())?;
    let frame: Value = serde_json::from_str(&input).map_err(|error| error.to_string())?;
    let route: RouteDecision =
        serde_json::from_value(frame.get("route").cloned().ok_or("route is missing")?)
            .map_err(|error| error.to_string())?;
    let message: MessageEnvelope =
        serde_json::from_value(frame.get("message").cloned().ok_or("message is missing")?)
            .map_err(|error| error.to_string())?;
    if route.provider == Provider::LegacyJms {
        return Err("modern provider application received a LegacyJms route".to_string());
    }
    let payload_kind = match message.payload {
        Payload::Text(_) => "text",
        Payload::Bytes(_) => "bytes",
        Payload::Map(_) => "map",
        Payload::Stream(_) => "stream",
        Payload::Object { .. } => "object",
    };
    println!(
        "provider={:?} mode={:?} destination={} message-id={} payload={}",
        route.provider, route.mode, message.destination, message.message_id, payload_kind
    );
    Ok(())
}
