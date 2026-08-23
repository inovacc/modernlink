use serde_json::Value;
use std::collections::BTreeMap;
pub(crate) struct RedactedProjection {
    pub value: Value,
    pub counters: BTreeMap<String, usize>,
}
pub(crate) fn redact_projection(value: Value) -> RedactedProjection {
    let mut counters = BTreeMap::new();
    let value = redact_value(value, &mut counters);
    RedactedProjection { value, counters }
}
fn redact_value(value: Value, counters: &mut BTreeMap<String, usize>) -> Value {
    match value {
        Value::Object(object) => Value::Object(
            object
                .into_iter()
                .map(|(key, value)| {
                    if secret_key(&key) {
                        *counters.entry("sensitive-field".to_owned()).or_default() += 1;
                        (key, Value::String("[REDACTED]".to_owned()))
                    } else {
                        (key, redact_value(value, counters))
                    }
                })
                .collect(),
        ),
        Value::Array(values) => Value::Array(
            values
                .into_iter()
                .map(|value| redact_value(value, counters))
                .collect(),
        ),
        value => value,
    }
}
fn secret_key(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    [
        "password",
        "secret",
        "token",
        "authorization",
        "cookie",
        "credential",
        "api_key",
    ]
    .iter()
    .any(|word| key.contains(word))
}
