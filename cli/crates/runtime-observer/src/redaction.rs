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

pub(crate) fn redact_reference(reference: &str, counters: &mut BTreeMap<String, usize>) -> String {
    let Ok(mut url) = reqwest::Url::parse(reference) else {
        return redact_raw_query(reference, counters);
    };
    if !url.username().is_empty() || url.password().is_some() {
        let _ = url.set_username("");
        let _ = url.set_password(None);
        *counters.entry("url-userinfo".to_owned()).or_default() += 1;
    }
    let pairs = url
        .query_pairs()
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect::<Vec<_>>();
    if pairs.iter().any(|(key, _)| is_sensitive_query_key(key)) {
        let mut query = url.query_pairs_mut();
        query.clear();
        for (key, value) in pairs {
            let value = if is_sensitive_query_key(&key) {
                *counters.entry("sensitive-query".to_owned()).or_default() += 1;
                "[REDACTED]".to_owned()
            } else {
                value
            };
            query.append_pair(&key, &value);
        }
    }
    url.into()
}

pub(crate) fn is_sensitive_query_key(key: &str) -> bool {
    if sensitive_normalized_key(&normalize_key(key)) {
        return true;
    }
    key.split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|segment| !segment.is_empty())
        .map(normalize_key)
        .any(|segment| sensitive_normalized_key(&segment))
}

fn normalize_key(key: &str) -> String {
    key.chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|character| character.to_ascii_lowercase())
        .collect()
}

fn sensitive_normalized_key(key: &str) -> bool {
    matches!(
        key,
        "token"
            | "secret"
            | "password"
            | "passwd"
            | "pwd"
            | "credential"
            | "credentials"
            | "authorization"
            | "cookie"
            | "apikey"
            | "authtoken"
            | "bearertoken"
            | "refreshtoken"
            | "idtoken"
            | "accesstoken"
            | "clientsecret"
            | "accesskey"
            | "privatekey"
    )
}

fn redact_raw_query(reference: &str, counters: &mut BTreeMap<String, usize>) -> String {
    let Some((prefix, query_and_fragment)) = reference.split_once('?') else {
        return reference.to_owned();
    };
    let (query, fragment) = query_and_fragment
        .split_once('#')
        .map_or((query_and_fragment, ""), |(query, fragment)| {
            (query, fragment)
        });
    let query = query
        .split('&')
        .map(|part| {
            let (key, value) = part.split_once('=').map_or((part, ""), |pair| pair);
            if is_sensitive_query_key(key) {
                *counters.entry("sensitive-query".to_owned()).or_default() += 1;
                format!("{key}=%5BREDACTED%5D")
            } else {
                format!("{key}={value}")
            }
        })
        .collect::<Vec<_>>()
        .join("&");
    if fragment.is_empty() {
        format!("{prefix}?{query}")
    } else {
        format!("{prefix}?{query}#{fragment}")
    }
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
    is_sensitive_query_key(key)
}
