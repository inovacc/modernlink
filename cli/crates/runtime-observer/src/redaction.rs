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
    let segments = key_segments(key);
    if segments.iter().any(|segment| sensitive_segment(segment)) {
        return true;
    }
    for width in 2..=segments.len().min(3) {
        if segments
            .windows(width)
            .map(|window| window.concat())
            .any(|phrase| sensitive_phrase(&phrase))
        {
            return true;
        }
    }
    false
}

fn key_segments(key: &str) -> Vec<String> {
    let mut segments = Vec::new();
    let mut current = String::new();
    let mut previous_lowercase = false;
    for character in key.chars() {
        if !character.is_ascii_alphanumeric() {
            push_segment(&mut segments, &mut current);
            previous_lowercase = false;
            continue;
        }
        if character.is_ascii_uppercase() && previous_lowercase {
            push_segment(&mut segments, &mut current);
        }
        current.push(character.to_ascii_lowercase());
        previous_lowercase = character.is_ascii_lowercase();
    }
    push_segment(&mut segments, &mut current);
    segments
}

fn push_segment(segments: &mut Vec<String>, current: &mut String) {
    if !current.is_empty() {
        segments.push(std::mem::take(current));
    }
}

fn sensitive_segment(segment: &str) -> bool {
    let base = segment.trim_end_matches(|character: char| character.is_ascii_digit());
    matches!(
        base,
        "token"
            | "secret"
            | "password"
            | "passwd"
            | "pwd"
            | "credential"
            | "credentials"
            | "authorization"
            | "cookie"
    )
}

fn sensitive_phrase(phrase: &str) -> bool {
    matches!(
        phrase,
        "authtoken"
            | "bearertoken"
            | "refreshtoken"
            | "idtoken"
            | "accesstoken"
            | "clientsecret"
            | "apikey"
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
