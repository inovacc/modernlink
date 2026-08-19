//! The JNI boundary — 25 `Java_*` entry points that build `libmodernlink`.
//!
//! This is the only crate the Java 6 facade in `java/src/main/java/com/modernlink` talks to.
//! It exposes HTTPS execution, the messaging client, and the UUID/Base64/JSON utilities, and
//! keeps the surface provider-neutral by going through the uniform transport boundary in
//! `messaging`.
//!
//! Every function here is an unsafe boundary that must stay in sync with its Java caller, and
//! a panic on this side can terminate the host JVM (`docs/adr/0001-jni-boundary-over-sidecar.md`).
//!
//! The package is named `jni` and depends on the external `jni` crate, so cargo needs
//! `-p jni@0.1.0` to disambiguate — see `docs/ISSUES.md` I-001.

use core::{Request, Response, TlsVersion};
use jni::objects::{JByteArray, JClass, JObjectArray, JString};
use jni::sys::{jbyteArray, jint, jlong, jobjectArray};
use jni::JNIEnv;
#[cfg(feature = "kafka")]
use messaging::KafkaTransport;
#[cfg(feature = "pulsar")]
use messaging::PulsarTransport;
#[cfg(feature = "rabbitmq")]
use messaging::RabbitMqTransport;
use messaging::{
    AcknowledgementMode, DeliveryReceipt, DeliveryState, InMemoryTransport, MessageEnvelope,
    MessageTransport, MessageTransportKind, Mode, Payload, Provider, RouteConfig, RouteRule,
    TraceContext,
};
#[cfg(feature = "nats")]
use messaging::{NatsTransport, NatsTransportKind};
use std::cell::RefCell;
use std::time::Duration;

thread_local! {
    static LAST_ERROR: RefCell<String> = const { RefCell::new(String::new()) };
}

fn set_error(message: String) -> jlong {
    LAST_ERROR.with(|value| *value.borrow_mut() = message);
    0
}

#[cfg(target_os = "linux")]
#[no_mangle]
pub extern "C" fn getauxval(_kind: usize) -> usize {
    0
}

struct NativeResponse(Response);

struct NativeMessagingClient {
    transport: MessageTransportKind,
    route: RouteConfig,
}

unsafe fn messaging_client<'a>(handle: jlong) -> Option<&'a NativeMessagingClient> {
    if handle == 0 {
        None
    } else {
        Some(&*(handle as *const NativeMessagingClient))
    }
}

fn java_string(env: &mut JNIEnv, value: &JString) -> Option<String> {
    env.get_string(value)
        .ok()
        .and_then(|text| text.to_str().ok().map(str::to_owned))
}

fn messaging_error(message: String) -> jlong {
    set_error(message)
}

fn messaging_string_error(message: String) -> jni::sys::jstring {
    set_error(message);
    std::ptr::null_mut()
}

// Also required by `kafka_group`, which derives the consumer group from it.
#[cfg(any(feature = "nats", feature = "pulsar", feature = "kafka"))]
fn jetstream_name(subject: &str, suffix: &str) -> String {
    let mut name = String::from("MODERNLINK_");
    for character in subject.chars() {
        if character.is_ascii_alphanumeric() {
            name.push(character.to_ascii_uppercase());
        } else {
            name.push('_');
        }
    }
    name.push('_');
    name.push_str(suffix);
    name
}

#[cfg(feature = "kafka")]
fn kafka_group(subject: &str) -> String {
    jetstream_name(subject, "KAFKA_GROUP")
}

fn messaging_mode(value: &str) -> Result<Mode, String> {
    match value {
        "TRANSPARENT" => Ok(Mode::Transparent),
        "TRANSFORM" => Ok(Mode::Transform),
        "REDIRECT" => Ok(Mode::Redirect),
        _ => Err(format!("unsupported messaging mode: {}", value)),
    }
}

fn messaging_provider(value: &str) -> Result<Provider, String> {
    match value {
        "LEGACY_JMS" => Ok(Provider::LegacyJms),
        "KAFKA" => Ok(Provider::Kafka),
        "PULSAR" => Ok(Provider::Pulsar),
        "NATS" => Ok(Provider::Nats),
        "NATS_JETSTREAM" => Ok(Provider::NatsJetStream),
        "RABBITMQ" => Ok(Provider::RabbitMq),
        _ => Err(format!("unsupported messaging provider: {}", value)),
    }
}

fn messaging_acknowledgement(value: &str) -> Result<AcknowledgementMode, String> {
    match value {
        "AUTO" => Ok(AcknowledgementMode::Auto),
        "CLIENT" => Ok(AcknowledgementMode::Client),
        "DUPLICATE_OK" => Ok(AcknowledgementMode::DuplicateOk),
        "TRANSACTED" => Ok(AcknowledgementMode::Transacted),
        _ => Err(format!("unsupported acknowledgement mode: {}", value)),
    }
}

fn messaging_mode_name(mode: Mode) -> &'static str {
    match mode {
        Mode::Transparent => "TRANSPARENT",
        Mode::Transform => "TRANSFORM",
        Mode::Redirect => "REDIRECT",
    }
}

/// Decode one pipe-delimited route rule supplied by the Java facade.
///
/// Field order matches `ModernRouteRule.encode()`:
/// `id|destination|destinationPrefix|tenant|headerName|headerValue|mode|provider|allowed`.
/// An empty string means "not constrained"; `allowed` is `1` or `0`.
///
/// This is strict on purpose. A malformed rule is rejected rather than skipped —
/// silently dropping a routing rule would change delivery behaviour without saying so,
/// which is exactly what the fail-closed rule forbids.
fn parse_route_rule(encoded: &str) -> Result<RouteRule, String> {
    let fields: Vec<&str> = encoded.split('|').collect();
    if fields.len() != 9 {
        return Err(format!(
            "route rule must have 9 pipe-delimited fields, found {}",
            fields.len()
        ));
    }
    if fields[0].is_empty() {
        return Err("route rule id must not be empty".to_string());
    }
    let optional = |value: &str| {
        if value.is_empty() {
            None
        } else {
            Some(value.to_string())
        }
    };
    if fields[4].is_empty() != fields[5].is_empty() {
        return Err(format!(
            "route rule {} must set both header name and header value, or neither",
            fields[0]
        ));
    }
    let allowed = match fields[8] {
        "1" => true,
        "0" => false,
        other => {
            return Err(format!(
                "route rule {} has an invalid allowed flag: {}",
                fields[0], other
            ))
        }
    };
    Ok(RouteRule {
        id: fields[0].to_string(),
        destination: optional(fields[1]),
        destination_prefix: optional(fields[2]),
        tenant: optional(fields[3]),
        header_name: optional(fields[4]),
        header_value: optional(fields[5]),
        mode: messaging_mode(fields[6])?,
        provider: messaging_provider(fields[7])?,
        allowed,
    })
}

/// Connect the transport for a provider. Shared by `nativeOpen` and `nativeOpenRouted`.
fn build_transport(
    provider: Provider,
    url: &str,
    subject: &str,
) -> Result<MessageTransportKind, String> {
    // SC-07: with every provider compiled out, no arm below reads these. The binding
    // keeps the signature stable across feature sets instead of renaming the parameters.
    let _ = (url, subject);
    Ok(match provider {
        Provider::LegacyJms => {
            MessageTransportKind::LegacyJms(InMemoryTransport::new(Provider::LegacyJms))
        }
        #[cfg(feature = "nats")]
        Provider::Nats => MessageTransportKind::Nats(NatsTransportKind::Core(Box::new(
            NatsTransport::connect(url, subject).map_err(|error| error.to_string())?,
        ))),
        #[cfg(feature = "nats")]
        Provider::NatsJetStream => {
            MessageTransportKind::Nats(NatsTransportKind::JetStream(Box::new(
                messaging::NatsJetStreamTransport::connect(
                    url,
                    subject,
                    &jetstream_name(subject, "STREAM"),
                    &jetstream_name(subject, "CONSUMER"),
                )
                .map_err(|error| error.to_string())?,
            )))
        }
        #[cfg(feature = "kafka")]
        Provider::Kafka => MessageTransportKind::Kafka(
            KafkaTransport::connect(url, subject, &kafka_group(subject))
                .map_err(|error| error.to_string())?,
        ),
        #[cfg(feature = "rabbitmq")]
        Provider::RabbitMq => MessageTransportKind::RabbitMq(Box::new(
            RabbitMqTransport::connect(url, subject).map_err(|error| error.to_string())?,
        )),
        #[cfg(feature = "pulsar")]
        Provider::Pulsar => MessageTransportKind::Pulsar(
            PulsarTransport::connect(
                url,
                subject,
                &jetstream_name(subject, "PULSAR_SUBSCRIPTION"),
            )
            .map_err(|error| error.to_string())?,
        ),
        // SC-07. The arms above are cfg-gated, so with a provider compiled out the match
        // is no longer exhaustive and these arms take over. They FAIL CLOSED: the caller
        // is told the capability is absent and which build flag restores it. It is never
        // downgraded to another transport, because silently moving a legacy application's
        // traffic to a different broker is exactly the misleading contract AGENTS.md
        // forbids. `provider_disabled` names the cargo feature so the message is
        // actionable rather than merely negative.
        #[cfg(not(feature = "nats"))]
        Provider::Nats | Provider::NatsJetStream => {
            return Err(provider_disabled(provider, "nats"))
        }
        #[cfg(not(feature = "kafka"))]
        Provider::Kafka => return Err(provider_disabled(provider, "kafka")),
        #[cfg(not(feature = "rabbitmq"))]
        Provider::RabbitMq => return Err(provider_disabled(provider, "rabbitmq")),
        #[cfg(not(feature = "pulsar"))]
        Provider::Pulsar => return Err(provider_disabled(provider, "pulsar")),
        // Still no `_` arm on purpose: a newly added Provider variant must be a compile
        // error here, not a runtime string.
    })
}

/// SC-07 fail-closed message for a provider whose transport was not compiled into this
/// build of `libmodernlink`.
#[cfg(not(all(
    feature = "nats",
    feature = "kafka",
    feature = "pulsar",
    feature = "rabbitmq"
)))]
fn provider_disabled(provider: Provider, feature: &str) -> String {
    format!(
        "provider {} is not available in this build of libmodernlink:          crates/jni was compiled without the `{}` cargo feature. Rebuild with          `--features {}` (or `--features all-providers`) to enable it.          The request was refused rather than routed to a different provider.",
        messaging_provider_name(provider),
        feature,
        feature
    )
}

fn messaging_provider_name(provider: Provider) -> &'static str {
    match provider {
        Provider::LegacyJms => "LEGACY_JMS",
        Provider::Kafka => "KAFKA",
        Provider::Pulsar => "PULSAR",
        Provider::Nats => "NATS",
        Provider::NatsJetStream => "NATS_JETSTREAM",
        Provider::RabbitMq => "RABBITMQ",
    }
}

fn messaging_state_name(state: DeliveryState) -> &'static str {
    match state {
        DeliveryState::Published => "PUBLISHED",
        DeliveryState::Received => "RECEIVED",
        DeliveryState::Acknowledged => "ACKNOWLEDGED",
        DeliveryState::Rejected => "REJECTED",
        DeliveryState::Retried => "RETRIED",
        DeliveryState::DeadLettered => "DEAD_LETTERED",
    }
}

fn messaging_receipt_frame(receipt: &DeliveryReceipt) -> String {
    format!(
        "{}#{}#{}#{}",
        receipt.message_id,
        messaging_provider_name(receipt.provider),
        messaging_state_name(receipt.state),
        receipt.trace_id
    )
}

fn messaging_message_frame(
    message: &MessageEnvelope,
    receipt: &DeliveryReceipt,
) -> Result<String, String> {
    let payload = match &message.payload {
        Payload::Text(value) => value,
        _ => {
            return Err("Java 6 messaging facade currently supports text payloads only".to_string())
        }
    };
    let acknowledgement = match message.acknowledgement_mode {
        AcknowledgementMode::Auto => "AUTO",
        AcknowledgementMode::Client => "CLIENT",
        AcknowledgementMode::DuplicateOk => "DUPLICATE_OK",
        AcknowledgementMode::Transacted => "TRANSACTED",
    };
    let message_frame = [
        message.message_id.clone(),
        message.destination.clone(),
        core::base64_encode(payload.as_bytes()),
        message.tracing.trace_id.clone(),
        message.tracing.span_id.clone(),
        message.tracing.parent_span_id.clone().unwrap_or_default(),
        message.tracing.trace_state.clone().unwrap_or_default(),
        acknowledgement.to_string(),
        if message.tracing.sampled {
            "1".to_string()
        } else {
            "0".to_string()
        },
    ]
    .join("|");
    Ok(format!(
        "{}\n{}",
        message_frame,
        messaging_receipt_frame(receipt)
    ))
}

#[no_mangle]
pub extern "system" fn Java_com_modernlink_messaging_ModernMessagingClient_nativeOpen(
    mut env: JNIEnv,
    _class: JClass,
    url: JString,
    subject: JString,
    mode: JString,
    provider: JString,
) -> jlong {
    let values = [&url, &subject, &mode, &provider]
        .iter()
        .map(|value| java_string(&mut env, value))
        .collect::<Option<Vec<_>>>();
    let values = match values {
        Some(values) => values,
        None => return messaging_error("invalid messaging connection string".to_string()),
    };
    let selected_mode = match messaging_mode(&values[2]) {
        Ok(value) => value,
        Err(error) => return messaging_error(error),
    };
    let selected_provider = match messaging_provider(&values[3]) {
        Ok(value) => value,
        Err(error) => return messaging_error(error),
    };
    let route = RouteConfig {
        default_mode: selected_mode,
        default_provider: selected_provider,
        rules: Vec::new(),
    };
    let route_probe = match MessageEnvelope::new(&values[1], Payload::Text(String::new()), 0) {
        Ok(value) => value,
        Err(error) => return messaging_error(error.to_string()),
    };
    if let Err(error) = route.decide(&route_probe) {
        return messaging_error(error.to_string());
    }
    let transport = match build_transport(selected_provider, &values[0], &values[1]) {
        Ok(value) => value,
        Err(error) => return messaging_error(error),
    };
    Box::into_raw(Box::new(NativeMessagingClient { transport, route })) as jlong
}

/// Open a messaging client WITH a routing policy.
///
/// `nativeOpen` builds a `RouteConfig` with no rules, so the policy engine in
/// `messaging` was unreachable from Java (BUGS B-002). This is the routed variant:
/// `rules` carries pipe-delimited rules in `ModernRouteRule.encode()` form, evaluated
/// in order, first match wins. It is additive — `nativeOpen` keeps its exact behaviour
/// for callers that do not route.
#[no_mangle]
pub extern "system" fn Java_com_modernlink_messaging_ModernMessagingClient_nativeOpenRouted(
    mut env: JNIEnv,
    _class: JClass,
    url: JString,
    subject: JString,
    mode: JString,
    provider: JString,
    rules: JObjectArray,
) -> jlong {
    let values = [&url, &subject, &mode, &provider]
        .iter()
        .map(|value| java_string(&mut env, value))
        .collect::<Option<Vec<_>>>();
    let values = match values {
        Some(values) => values,
        None => return messaging_error("invalid messaging connection string".to_string()),
    };
    let selected_mode = match messaging_mode(&values[2]) {
        Ok(value) => value,
        Err(error) => return messaging_error(error),
    };
    let selected_provider = match messaging_provider(&values[3]) {
        Ok(value) => value,
        Err(error) => return messaging_error(error),
    };
    let encoded_rules = match java_string_array(&mut env, &rules) {
        Some(value) => value,
        None => return messaging_error("invalid routing rule array".to_string()),
    };
    let mut parsed = Vec::with_capacity(encoded_rules.len());
    for encoded in &encoded_rules {
        match parse_route_rule(encoded) {
            Ok(rule) => parsed.push(rule),
            Err(error) => return messaging_error(error),
        }
    }
    let route = RouteConfig {
        default_mode: selected_mode,
        default_provider: selected_provider,
        rules: parsed,
    };
    let route_probe = match MessageEnvelope::new(&values[1], Payload::Text(String::new()), 0) {
        Ok(value) => value,
        Err(error) => return messaging_error(error.to_string()),
    };
    if let Err(error) = route.decide(&route_probe) {
        return messaging_error(error.to_string());
    }
    let transport = match build_transport(selected_provider, &values[0], &values[1]) {
        Ok(value) => value,
        Err(error) => return messaging_error(error),
    };
    Box::into_raw(Box::new(NativeMessagingClient { transport, route })) as jlong
}

/// Evaluate the routing policy for a hypothetical message WITHOUT publishing it.
///
/// Returns `mode#provider#ruleId#allowed`, with an empty `ruleId` when the default
/// route applied. This is the dry-run path (`RouteConfig::dry_run`) surfaced to Java —
/// a denied route comes back as a decision so the caller can explain *why*, rather
/// than as an error.
#[no_mangle]
pub extern "system" fn Java_com_modernlink_messaging_ModernMessagingClient_nativeDryRun(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    destination: JString,
    tenant: JString,
    header_name: JString,
    header_value: JString,
) -> jni::sys::jstring {
    let client = match unsafe { messaging_client(handle) } {
        Some(client) => client,
        None => return messaging_string_error("messaging client is closed".to_string()),
    };
    let values = [&destination, &tenant, &header_name, &header_value]
        .iter()
        .map(|value| java_string(&mut env, value))
        .collect::<Option<Vec<_>>>();
    let values = match values {
        Some(values) => values,
        None => return messaging_string_error("invalid dry-run arguments".to_string()),
    };
    let mut probe = match MessageEnvelope::new(&values[0], Payload::Text(String::new()), 0) {
        Ok(value) => value,
        Err(error) => return messaging_string_error(error.to_string()),
    };
    if !values[1].is_empty() {
        probe.tenant = Some(values[1].clone());
    }
    if !values[2].is_empty() {
        probe.headers.insert(values[2].clone(), values[3].clone());
    }
    let decision = match client.route.dry_run(&probe) {
        Ok(value) => value,
        Err(error) => return messaging_string_error(error.to_string()),
    };
    let frame = format!(
        "{}#{}#{}#{}",
        messaging_mode_name(decision.mode),
        messaging_provider_name(decision.provider),
        decision.rule_id.unwrap_or_default(),
        if decision.allowed { "1" } else { "0" }
    );
    match env.new_string(frame) {
        Ok(value) => value.into_raw(),
        Err(error) => messaging_string_error(error.to_string()),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_modernlink_messaging_ModernMessagingClient_nativePublish(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    message_id: JString,
    destination: JString,
    payload: JString,
    trace_id: JString,
    span_id: JString,
    parent_span_id: JString,
    trace_state: JString,
    sampled: jni::sys::jboolean,
    acknowledgement_mode: JString,
) -> jni::sys::jstring {
    let client = match unsafe { messaging_client(handle) } {
        Some(value) => value,
        None => return messaging_string_error("messaging client handle is invalid".to_string()),
    };
    let values = [
        &message_id,
        &destination,
        &payload,
        &trace_id,
        &span_id,
        &parent_span_id,
        &trace_state,
        &acknowledgement_mode,
    ]
    .iter()
    .map(|value| java_string(&mut env, value))
    .collect::<Option<Vec<_>>>();
    let values = match values {
        Some(values) => values,
        None => return messaging_string_error("invalid messaging message string".to_string()),
    };
    let acknowledgement = match messaging_acknowledgement(&values[7]) {
        Ok(value) => value,
        Err(error) => return messaging_string_error(error),
    };
    let mut message = match MessageEnvelope::new(&values[1], Payload::Text(values[2].clone()), 0) {
        Ok(value) => value,
        Err(error) => return messaging_string_error(error.to_string()),
    };
    message.message_id = values[0].clone();
    message.acknowledgement_mode = acknowledgement;
    message.tracing = TraceContext {
        trace_id: values[3].clone(),
        span_id: values[4].clone(),
        parent_span_id: if values[5].is_empty() {
            None
        } else {
            Some(values[5].clone())
        },
        trace_state: if values[6].is_empty() {
            None
        } else {
            Some(values[6].clone())
        },
        sampled: sampled != 0,
    };
    let receipt = match client.route.dispatch(message, &client.transport) {
        Ok(value) => value.receipt,
        Err(error) => return messaging_string_error(error.to_string()),
    };
    match env.new_string(messaging_receipt_frame(&receipt)) {
        Ok(value) => value.into_raw(),
        Err(error) => messaging_string_error(error.to_string()),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_modernlink_messaging_ModernMessagingClient_nativeReceive(
    env: JNIEnv,
    _class: JClass,
    handle: jlong,
) -> jni::sys::jstring {
    let client = match unsafe { messaging_client(handle) } {
        Some(value) => value,
        None => return messaging_string_error("messaging client handle is invalid".to_string()),
    };
    let received = match client.transport.receive() {
        Ok(Some(value)) => value,
        Ok(None) => return messaging_string_error("no messaging message available".to_string()),
        Err(error) => return messaging_string_error(error.to_string()),
    };
    let frame = match messaging_message_frame(&received.message, &received.receipt) {
        Ok(value) => value,
        Err(error) => return messaging_string_error(error),
    };
    match env.new_string(frame) {
        Ok(value) => value.into_raw(),
        Err(error) => messaging_string_error(error.to_string()),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_modernlink_messaging_ModernMessagingClient_nativeAcknowledge(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    message_id: JString,
    provider: JString,
    state: JString,
    trace_id: JString,
) -> jni::sys::jstring {
    let client = match unsafe { messaging_client(handle) } {
        Some(value) => value,
        None => return messaging_string_error("messaging client handle is invalid".to_string()),
    };
    let values = [&message_id, &provider, &state, &trace_id]
        .iter()
        .map(|value| java_string(&mut env, value))
        .collect::<Option<Vec<_>>>();
    let values = match values {
        Some(values) => values,
        None => return messaging_string_error("invalid delivery receipt string".to_string()),
    };
    let provider = match messaging_provider(&values[1]) {
        Ok(value) => value,
        Err(error) => return messaging_string_error(error),
    };
    let state = match values[2].as_str() {
        "PUBLISHED" => DeliveryState::Published,
        "RECEIVED" => DeliveryState::Received,
        "ACKNOWLEDGED" => DeliveryState::Acknowledged,
        "REJECTED" => DeliveryState::Rejected,
        "RETRIED" => DeliveryState::Retried,
        "DEAD_LETTERED" => DeliveryState::DeadLettered,
        _ => return messaging_string_error("unsupported delivery state".to_string()),
    };
    let receipt = DeliveryReceipt {
        message_id: values[0].clone(),
        provider,
        state,
        trace_id: values[3].clone(),
    };
    let acknowledged = match client.transport.acknowledge(&receipt) {
        Ok(value) => value,
        Err(error) => return messaging_string_error(error.to_string()),
    };
    match env.new_string(messaging_receipt_frame(&acknowledged)) {
        Ok(value) => value.into_raw(),
        Err(error) => messaging_string_error(error.to_string()),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_modernlink_messaging_ModernMessagingClient_nativeClose(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) {
    if handle != 0 {
        unsafe {
            drop(Box::from_raw(handle as *mut NativeMessagingClient));
        }
    }
}

#[no_mangle]
pub extern "system" fn Java_com_modernlink_messaging_ModernMessagingClient_nativeLastError(
    env: JNIEnv,
    _class: JClass,
) -> jni::sys::jstring {
    let message = LAST_ERROR.with(|value| value.borrow().clone());
    match env.new_string(message) {
        Ok(value) => value.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

unsafe fn response<'a>(handle: jlong) -> Option<&'a NativeResponse> {
    if handle == 0 {
        None
    } else {
        Some(&*(handle as *const NativeResponse))
    }
}

#[no_mangle]
pub extern "system" fn Java_com_modernlink_LegacyHttpClient_nativeExecute(
    mut env: JNIEnv,
    _class: JClass,
    url: JString,
    method: JString,
    headers: JObjectArray,
    body: JByteArray,
    connect_timeout_millis: jlong,
    read_timeout_millis: jlong,
    follow_redirects: jni::sys::jboolean,
    max_redirects: jint,
    minimum_tls_version: jint,
) -> jlong {
    let url = match env
        .get_string(&url)
        .ok()
        .and_then(|value| value.to_str().ok().map(str::to_owned))
    {
        Some(value) => value,
        None => return set_error("invalid URL string".to_string()),
    };
    let method = match env
        .get_string(&method)
        .ok()
        .and_then(|value| value.to_str().ok().map(str::to_owned))
    {
        Some(value) => value,
        None => return set_error("invalid method string".to_string()),
    };
    let mut request = match Request::new(&url) {
        Ok(value) => value,
        Err(error) => return set_error(error.to_string()),
    };
    request.method = method;
    if env.get_array_length(&headers).ok().unwrap_or(1) % 2 != 0 {
        return set_error("headers must contain name/value pairs".to_string());
    }
    let header_count = env.get_array_length(&headers).ok().unwrap_or(0);
    for index in (0..header_count).step_by(2) {
        let name = match env
            .get_object_array_element(&headers, index)
            .ok()
            .and_then(|value| {
                env.get_string(&JString::from(value))
                    .ok()
                    .and_then(|text| text.to_str().ok().map(str::to_owned))
            }) {
            Some(value) => value,
            None => return set_error("invalid header name".to_string()),
        };
        let value = match env
            .get_object_array_element(&headers, index + 1)
            .ok()
            .and_then(|value| {
                env.get_string(&JString::from(value))
                    .ok()
                    .and_then(|text| text.to_str().ok().map(str::to_owned))
            }) {
            Some(value) => value,
            None => return set_error("invalid header value".to_string()),
        };
        request.headers.insert(name, value);
    }
    request.body = match env.convert_byte_array(&body) {
        Ok(value) => value,
        Err(error) => return set_error(error.to_string()),
    };
    if connect_timeout_millis > 0 {
        request.connect_timeout = Some(Duration::from_millis(connect_timeout_millis as u64));
    }
    if read_timeout_millis > 0 {
        request.read_timeout = Some(Duration::from_millis(read_timeout_millis as u64));
    }
    request.follow_redirects = follow_redirects != 0;
    if max_redirects < 0 {
        return set_error("maximum redirects must not be negative".to_string());
    }
    request.max_redirects = max_redirects as u32;
    request.minimum_tls_version = match minimum_tls_version {
        12 => TlsVersion::Tls12,
        13 => TlsVersion::Tls13,
        _ => return set_error("unsupported minimum TLS version".to_string()),
    };
    let result = match http::execute(&request) {
        Ok(value) => value,
        Err(error) => return set_error(error.to_string()),
    };
    Box::into_raw(Box::new(NativeResponse(result))) as jlong
}

#[no_mangle]
pub extern "system" fn Java_com_modernlink_LegacyHttpClient_nativeLastError(
    env: JNIEnv,
    _class: JClass,
) -> jni::sys::jstring {
    let message = LAST_ERROR.with(|value| value.borrow().clone());
    match env.new_string(message) {
        Ok(value) => value.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_modernlink_LegacyHttpClient_nativeCapabilities(
    _env: JNIEnv,
    _class: JClass,
) -> jlong {
    31
}

#[no_mangle]
pub extern "system" fn Java_com_modernlink_ModernUuid_nativeV7(
    env: JNIEnv,
    _class: JClass,
) -> jni::sys::jstring {
    match env.new_string(core::uuid_v7()) {
        Ok(value) => value.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_modernlink_ModernUuid_nativeV4(
    env: JNIEnv,
    _class: JClass,
) -> jni::sys::jstring {
    match env.new_string(core::uuid_v4()) {
        Ok(value) => value.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_modernlink_ModernBase64_nativeEncode(
    env: JNIEnv,
    _class: JClass,
    value: JByteArray,
) -> jni::sys::jstring {
    let value = match env.convert_byte_array(&value) {
        Ok(value) => value,
        Err(_) => return std::ptr::null_mut(),
    };
    match env.new_string(core::base64_encode(&value)) {
        Ok(value) => value.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_modernlink_ModernBase64_nativeDecode(
    mut env: JNIEnv,
    _class: JClass,
    value: JString,
) -> jni::sys::jbyteArray {
    let value = match env
        .get_string(&value)
        .ok()
        .and_then(|text| text.to_str().ok().map(str::to_owned))
    {
        Some(value) => value,
        None => return std::ptr::null_mut(),
    };
    match core::base64_decode(&value)
        .ok()
        .and_then(|bytes| env.byte_array_from_slice(&bytes).ok())
    {
        Some(bytes) => bytes.into_raw(),
        None => std::ptr::null_mut(),
    }
}

fn java_string_array(env: &mut JNIEnv, values: &JObjectArray) -> Option<Vec<String>> {
    let length = env.get_array_length(values).ok()?;
    let mut result = Vec::with_capacity(length as usize);
    for index in 0..length {
        let value = env.get_object_array_element(values, index).ok()?;
        let value = env
            .get_string(&JString::from(value))
            .ok()?
            .to_str()
            .ok()?
            .to_owned();
        result.push(value);
    }
    Some(result)
}

#[no_mangle]
pub extern "system" fn Java_com_modernlink_ModernJson_nativeObject(
    mut env: JNIEnv,
    _class: JClass,
    fields: JObjectArray,
) -> jni::sys::jstring {
    let fields = match java_string_array(&mut env, &fields) {
        Some(value) => value,
        None => return std::ptr::null_mut(),
    };
    match core::json_object(&fields)
        .ok()
        .and_then(|value| env.new_string(value).ok())
    {
        Some(value) => value.into_raw(),
        None => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_modernlink_ModernJson_nativeArray(
    mut env: JNIEnv,
    _class: JClass,
    values: JObjectArray,
) -> jni::sys::jstring {
    let values = match java_string_array(&mut env, &values) {
        Some(value) => value,
        None => return std::ptr::null_mut(),
    };
    match core::json_array(&values)
        .ok()
        .and_then(|value| env.new_string(value).ok())
    {
        Some(value) => value.into_raw(),
        None => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_modernlink_ModernJson_nativeDecode(
    mut env: JNIEnv,
    _class: JClass,
    json: JString,
) -> jni::sys::jstring {
    let json = match env
        .get_string(&json)
        .ok()
        .and_then(|text| text.to_str().ok().map(str::to_owned))
    {
        Some(value) => value,
        None => return std::ptr::null_mut(),
    };
    match core::json_decode(&json)
        .ok()
        .and_then(|value| env.new_string(value).ok())
    {
        Some(value) => value.into_raw(),
        None => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_modernlink_LegacyHttpClient_nativeStatus(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) -> jint {
    unsafe {
        response(handle)
            .map(|value| value.0.status as jint)
            .unwrap_or(0)
    }
}

#[no_mangle]
pub extern "system" fn Java_com_modernlink_LegacyHttpClient_nativeStatusMessage(
    env: JNIEnv,
    _class: JClass,
    handle: jlong,
) -> jni::sys::jstring {
    match unsafe { response(handle).map(|value| value.0.status_message.clone()) }
        .and_then(|value| env.new_string(value).ok())
    {
        Some(value) => value.into_raw(),
        None => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_modernlink_LegacyHttpClient_nativeHeaders(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
) -> jobjectArray {
    let headers = match unsafe { response(handle) } {
        Some(value) => &value.0.headers,
        None => return std::ptr::null_mut(),
    };
    let string_class = match env.find_class("java/lang/String") {
        Ok(value) => value,
        Err(_) => return std::ptr::null_mut(),
    };
    let array =
        match env.new_object_array((headers.len() * 2) as i32, string_class, JString::default()) {
            Ok(value) => value,
            Err(_) => return std::ptr::null_mut(),
        };
    for (index, (name, value)) in headers.iter().enumerate() {
        let name = match env.new_string(name) {
            Ok(value) => value,
            Err(_) => return std::ptr::null_mut(),
        };
        let value = match env.new_string(value) {
            Ok(value) => value,
            Err(_) => return std::ptr::null_mut(),
        };
        if env
            .set_object_array_element(&array, (index * 2) as i32, name)
            .is_err()
            || env
                .set_object_array_element(&array, (index * 2 + 1) as i32, value)
                .is_err()
        {
            return std::ptr::null_mut();
        }
    }
    array.into_raw()
}

#[no_mangle]
pub extern "system" fn Java_com_modernlink_LegacyHttpClient_nativeBody(
    env: JNIEnv,
    _class: JClass,
    handle: jlong,
) -> jbyteArray {
    let body = match unsafe { response(handle) } {
        Some(value) => &value.0.body,
        None => return std::ptr::null_mut(),
    };
    match env.byte_array_from_slice(body) {
        Ok(value) => value.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_modernlink_LegacyHttpClient_nativeTlsCertificates(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
) -> jobjectArray {
    let certificates = match unsafe { response(handle) } {
        Some(value) => match &value.0.tls {
            Some(info) => &info.peer_certificates_der,
            None => return std::ptr::null_mut(),
        },
        None => return std::ptr::null_mut(),
    };
    let byte_array_class = match env.find_class("[B") {
        Ok(value) => value,
        Err(_) => return std::ptr::null_mut(),
    };
    let array = match env.new_object_array(
        certificates.len() as i32,
        byte_array_class,
        JByteArray::default(),
    ) {
        Ok(value) => value,
        Err(_) => return std::ptr::null_mut(),
    };
    for (index, certificate) in certificates.iter().enumerate() {
        let value = match env.byte_array_from_slice(certificate) {
            Ok(value) => value,
            Err(_) => return std::ptr::null_mut(),
        };
        if env
            .set_object_array_element(&array, index as i32, value)
            .is_err()
        {
            return std::ptr::null_mut();
        }
    }
    array.into_raw()
}

fn tls_string(handle: jlong, protocol: bool) -> Option<String> {
    unsafe {
        response(handle)
            .and_then(|value| value.0.tls.as_ref())
            .and_then(|info| {
                if protocol {
                    info.protocol.clone()
                } else {
                    info.cipher_suite.clone()
                }
            })
    }
}

#[no_mangle]
pub extern "system" fn Java_com_modernlink_LegacyHttpClient_nativeFinalUrl(
    env: JNIEnv,
    _class: JClass,
    handle: jlong,
) -> jni::sys::jstring {
    match unsafe { response(handle).map(|value| value.0.final_url.clone()) }
        .and_then(|value| env.new_string(value).ok())
    {
        Some(value) => value.into_raw(),
        None => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_modernlink_LegacyHttpClient_nativeTlsProtocol(
    env: JNIEnv,
    _class: JClass,
    handle: jlong,
) -> jni::sys::jstring {
    match tls_string(handle, true).and_then(|value| env.new_string(value).ok()) {
        Some(value) => value.into_raw(),
        None => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_modernlink_LegacyHttpClient_nativeTlsCipherSuite(
    env: JNIEnv,
    _class: JClass,
    handle: jlong,
) -> jni::sys::jstring {
    match tls_string(handle, false).and_then(|value| env.new_string(value).ok()) {
        Some(value) => value.into_raw(),
        None => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_modernlink_LegacyHttpClient_nativeRelease(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) {
    if handle != 0 {
        unsafe {
            drop(Box::from_raw(handle as *mut NativeResponse));
        }
    }
}

#[cfg(test)]
mod feature_gating_tests {
    use super::*;

    /// SC-07 fail-closed contract.
    ///
    /// AGENTS.md: "Fail closed on unsupported guarantees. A capability gap must be
    /// reported explicitly - never silently degraded." Feature-gating the providers
    /// created exactly such a gap, so this asserts the gap is *reported*: asking for a
    /// provider that was compiled out must return an error that names the provider and
    /// the cargo feature which restores it.
    ///
    /// The dangerous failure this guards against is not an error - it is a *success*
    /// on some other transport. A legacy application that asks for Kafka and silently
    /// gets an in-memory queue would report every publish as delivered.
    #[cfg(not(feature = "kafka"))]
    #[test]
    fn kafka_is_refused_when_it_was_not_compiled_in() {
        // `expect_err` needs `T: Debug` and MessageTransportKind holds broker handles
        // that are not Debug, so the refusal is matched explicitly.
        let error = match build_transport(Provider::Kafka, "localhost:9092", "orders") {
            Ok(_) => panic!("Kafka was compiled out but build_transport returned a transport"),
            Err(error) => error,
        };
        assert!(error.contains("KAFKA"), "must name the provider: {}", error);
        assert!(
            error.contains("`kafka` cargo feature"),
            "must name the feature that restores it: {}",
            error
        );
        assert!(
            error.contains("refused rather than routed to a different provider"),
            "must state that no fallback happened: {}",
            error
        );
    }

    #[cfg(not(feature = "nats"))]
    #[test]
    fn both_nats_variants_are_refused_when_nats_was_not_compiled_in() {
        for provider in [Provider::Nats, Provider::NatsJetStream] {
            let error = match build_transport(provider, "127.0.0.1:4222", "orders") {
                Ok(_) => panic!("NATS was compiled out but build_transport returned a transport"),
                Err(error) => error,
            };
            assert!(error.contains("`nats` cargo feature"), "{}", error);
        }
    }

    /// LEGACY_JMS is in-process and depends on no provider crate, so it must keep
    /// working in the broker-free default build - otherwise gating would have broken
    /// the transparent-mode path the Java 6 fixtures rely on.
    #[test]
    fn legacy_jms_is_available_in_every_feature_configuration() {
        let transport = match build_transport(Provider::LegacyJms, "", "orders") {
            Ok(transport) => transport,
            Err(error) => panic!("LEGACY_JMS must always build, got: {}", error),
        };
        assert_eq!(transport.provider(), Provider::LegacyJms);
    }
}
