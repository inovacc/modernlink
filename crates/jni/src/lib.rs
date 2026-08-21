//! The JNI boundary — 28 `Java_*` entry points that build `libmodernlink`.
//!
//! This is the only crate the Java 6 facade in `java/src/main/java/com/modernlink` talks to.
//! It exposes HTTPS execution, the messaging client, and the UUID/Base64/JSON utilities, and
//! keeps the surface provider-neutral by going through the uniform transport boundary in
//! `messaging`.
//!
//! Every function here is an unsafe boundary that must stay in sync with its Java caller, and
//! a panic on this side can terminate the host JVM (`docs/adr/0001-jni-boundary-over-sidecar.md`).
//!
//! The package is `jni-bridge` (the folder is still `crates/jni`, and `[lib] name` is still
//! `modernlink`). It was named `jni`, which shadowed the external `jni` crate it depends on and
//! forced `-p jni@0.1.0` on every invocation; SC-05 renamed it, so `-p jni-bridge` is the
//! spelling and `-p jni` is unambiguous again — see `docs/ISSUES.md` I-001.

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
use modernlink_core::{Request, Response, TlsVersion};
use std::cell::RefCell;
use std::time::Duration;

thread_local! {
    static LAST_ERROR: RefCell<String> = const { RefCell::new(String::new()) };
}

/// B-004 — contain a panic before it unwinds into the JVM.
///
/// Every `Java_*` entry point runs its body inside this. A Rust panic crossing an
/// `extern "system"` boundary into JVM frames is undefined behaviour, and this library is
/// loaded *into* the process of a vendor-locked Java 6 application that must not be
/// destabilised (`docs/adr/0001-jni-boundary-over-sidecar.md`).
///
/// On a panic the payload is recorded through the same `LAST_ERROR` channel every other
/// failure uses, so `nativeLastError` reports it and the Java side raises
/// `LegacyHttpException` exactly as it would for an ordinary error. The caller gets the
/// type's error sentinel — `0` for `jlong`/`jint`, null for every object return.
///
/// `AssertUnwindSafe` is required because `JNIEnv` is not `UnwindSafe`.
///
/// **Read the limit of that assertion carefully — it is narrower than it looks.** The
/// handle a caught panic leaves behind IS reused: a `NativeMessagingClient` outlives the
/// call, and the Java caller will make another one. `LAST_ERROR` is safe (overwritten
/// wholesale, never mutated in place), but transport state is not automatically so.
/// `NatsTransport::receive` takes its subscription out of a `Mutex`, awaits, and only then
/// puts it back (`crates/messaging/src/lib.rs:620-642`); a panic in the middle drops the
/// subscription and leaves the `Mutex` holding `None` for good, so every later `receive`
/// reports "NATS subscription is unavailable" on a client that still looks open. The
/// `Mutex` is not poisoned by this, because the guard is released before the await.
///
/// So this guard converts **undefined behaviour into a reported error**, which is the
/// difference between a corrupted host process and a diagnosable failure — and that is all
/// it does. It does not make the affected client usable again. A caller that sees a
/// contained panic should close and reopen. Making the client fail closed on reuse after a
/// contained panic is tracked as `docs/BUGS.md` B-007; it needs the handle registry from
/// B-005 to do properly.
///
/// This does NOT make a panic acceptable.
fn jni_guard<T>(sentinel: T, body: impl FnOnce() -> T) -> T {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(body)) {
        Ok(value) => value,
        Err(payload) => {
            set_error(describe_panic(payload.as_ref()));
            // Do not drop the payload. A custom `panic_any` value whose `Drop` panics would
            // panic while a panic is being handled, which aborts the process - the exact
            // outcome this function exists to prevent. Leaking a payload on an already
            // catastrophic path is the cheaper of the two failures.
            std::mem::forget(payload);
            sentinel
        }
    }
}

/// Render a panic payload without re-panicking.
///
/// `panic!("...")` yields `&str` or `String` depending on whether it was formatted; anything
/// else (a custom `panic_any`) is unknowable here. Never unwrap a downcast — a panic while
/// handling a panic aborts the process, which is the outcome this whole function exists to
/// avoid.
fn describe_panic(payload: &(dyn std::any::Any + Send)) -> String {
    let detail = if let Some(text) = payload.downcast_ref::<&str>() {
        (*text).to_string()
    } else if let Some(text) = payload.downcast_ref::<String>() {
        text.clone()
    } else {
        "non-string panic payload".to_string()
    };
    format!(
        "internal error: a panic was contained at the JNI boundary and did not reach the JVM \
         ({}). This is a defect in the native library - please report it with this message.",
        detail
    )
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

/// Live messaging clients, keyed by an opaque handle (B-005, H-05).
///
/// The handle used to be the address of a leaked `Box`, dereferenced after a null check and
/// nothing else. A stale, copied or fabricated `jlong` therefore reached
/// `&*(handle as *const _)` unchecked — a use-after-free inside the host JVM, which is the
/// one thing this product must never cause.
///
/// Handles are now ids into this map, so an unknown id is a lookup miss and an ordinary
/// error rather than undefined behaviour. Ids are never reused (`NEXT_HANDLE` only
/// increments), so a handle from a closed client cannot collide with a later one.
///
/// Lookup clones the `Arc` and releases the lock immediately. Holding it across a blocking
/// broker receive would serialise every client in the process behind one mutex — a real
/// throughput regression in the name of safety. The `Arc` also means `close()` racing with
/// an in-flight call frees only after that call finishes, instead of pulling memory out from
/// under it.
static CLIENTS: std::sync::LazyLock<
    std::sync::Mutex<std::collections::HashMap<u64, std::sync::Arc<NativeMessagingClient>>>,
> = std::sync::LazyLock::new(|| std::sync::Mutex::new(std::collections::HashMap::new()));

/// Starts at 1 so 0 stays the "invalid handle" sentinel the Java side already checks.
static NEXT_HANDLE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

fn register_client(client: NativeMessagingClient) -> jlong {
    let handle = NEXT_HANDLE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    match CLIENTS.lock() {
        Ok(mut clients) => {
            clients.insert(handle, std::sync::Arc::new(client));
            handle as jlong
        }
        // A poisoned registry means a thread panicked holding the lock. Refusing to hand
        // out a handle is the fail-closed answer: the alternative is a client the caller
        // believes is open and that nothing can ever look up.
        Err(_) => set_error("messaging client registry is unavailable".to_string()),
    }
}

fn client_for(handle: jlong) -> Option<std::sync::Arc<NativeMessagingClient>> {
    if handle <= 0 {
        return None;
    }
    CLIENTS.lock().ok()?.get(&(handle as u64)).cloned()
}

/// Remove a client. Idempotent: closing twice, or closing an unknown handle, is a no-op
/// rather than a double free.
fn unregister_client(handle: jlong) {
    if handle <= 0 {
        return;
    }
    if let Ok(mut clients) = CLIENTS.lock() {
        clients.remove(&(handle as u64));
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

/// MSG-05 — the wire name for a payload category.
///
/// The frame carries the category alongside the bytes because base64 alone is ambiguous:
/// the receiver cannot tell a UTF-8 string from an opaque blob, and guessing would make
/// a BytesMessage silently arrive as text.
fn messaging_payload_kind_name(payload: &Payload) -> &'static str {
    match payload {
        Payload::Text(_) => "TEXT",
        Payload::Bytes(_) => "BYTES",
        Payload::Map(_) => "MAP",
        Payload::Stream(_) => "STREAM",
        Payload::Object { .. } => "OBJECT",
    }
}

/// A map encoded so no key or value can collide with a delimiter.
///
/// `base64(key):base64(value)`, pairs joined by `,`. Both halves are base64 precisely so
/// a key containing a delimiter cannot forge a pair boundary. `BTreeMap` iteration is
/// ordered, so the encoding is deterministic and two equal maps encode identically.
///
/// The separators are `:` and `,` because neither is in the base64 alphabet
/// (`A-Z a-z 0-9 + / =`). `=` was the obvious choice and is wrong: it is base64 *padding*,
/// so `base64("alpha")` ends in `=` and splitting on the first one cut the key in half.
/// The delimiter round-trip test caught exactly that.
fn messaging_encode_map(entries: &std::collections::BTreeMap<String, String>) -> Vec<u8> {
    let mut parts: Vec<String> = Vec::new();
    for (key, value) in entries {
        parts.push(format!(
            "{}:{}",
            modernlink_core::base64_encode(key.as_bytes()),
            modernlink_core::base64_encode(value.as_bytes())
        ));
    }
    parts.join(",").into_bytes()
}

fn messaging_decode_map(
    bytes: &[u8],
) -> Result<std::collections::BTreeMap<String, String>, String> {
    let text = String::from_utf8(bytes.to_vec())
        .map_err(|_| "map payload is not valid UTF-8".to_string())?;
    let mut entries = std::collections::BTreeMap::new();
    if text.is_empty() {
        return Ok(entries);
    }
    for pair in text.split(',') {
        let mut halves = pair.splitn(2, ':');
        let key = halves
            .next()
            .ok_or_else(|| "map payload entry has no key".to_string())?;
        let value = halves
            .next()
            .ok_or_else(|| "map payload entry has no value".to_string())?;
        let key = modernlink_core::base64_decode(key).map_err(|error| error.to_string())?;
        let value = modernlink_core::base64_decode(value).map_err(|error| error.to_string())?;
        entries.insert(
            String::from_utf8(key).map_err(|_| "map key is not valid UTF-8".to_string())?,
            String::from_utf8(value).map_err(|_| "map value is not valid UTF-8".to_string())?,
        );
    }
    Ok(entries)
}

/// The canonical bytes for a payload, as carried in the frame.
fn messaging_payload_bytes(payload: &Payload) -> Vec<u8> {
    match payload {
        Payload::Text(value) => value.as_bytes().to_vec(),
        Payload::Bytes(value) | Payload::Stream(value) => value.clone(),
        Payload::Map(entries) => messaging_encode_map(entries),
        Payload::Object { bytes, .. } => bytes.clone(),
    }
}

/// Rebuild a payload from the category and its bytes.
///
/// STREAM and OBJECT are refused rather than accepted. STREAM needs typed field ordering
/// this encoding does not carry, and OBJECT would mean handing broker-supplied bytes to
/// Java deserialization -- a well-known remote-code-execution surface that must not be
/// opened by default in a compatibility layer fronting a legacy application. Both refuse
/// explicitly, naming why, instead of degrading to BYTES.
fn messaging_build_payload(kind: &str, bytes: Vec<u8>) -> Result<Payload, String> {
    match kind {
        "TEXT" => String::from_utf8(bytes)
            .map(Payload::Text)
            .map_err(|_| "text payload is not valid UTF-8".to_string()),
        "BYTES" => Ok(Payload::Bytes(bytes)),
        "MAP" => messaging_decode_map(&bytes).map(Payload::Map),
        "STREAM" => Err(
            "STREAM payloads are not carried across the Java 6 boundary: the \
                         frame does not encode the typed field ordering a StreamMessage \
                         requires, and delivering it as opaque bytes would lose that \
                         structure silently"
                .to_string(),
        ),
        "OBJECT" => Err(
            "OBJECT payloads are deliberately not carried across the Java 6 \
                         boundary: reconstructing one means deserializing broker-supplied \
                         bytes into Java objects, which is a remote-code-execution surface. \
                         Use BYTES and deserialize explicitly if the application accepts \
                         that risk"
                .to_string(),
        ),
        other => Err(format!("unknown payload category: {}", other)),
    }
}

fn messaging_message_frame(
    message: &MessageEnvelope,
    receipt: &DeliveryReceipt,
) -> Result<String, String> {
    // MSG-05: every category is carried now, not just text. The category travels with
    // the bytes so the receiver never has to guess whether base64 holds a UTF-8 string.
    let payload_bytes = messaging_payload_bytes(&message.payload);
    let payload_kind = messaging_payload_kind_name(&message.payload);
    let acknowledgement = match message.acknowledgement_mode {
        AcknowledgementMode::Auto => "AUTO",
        AcknowledgementMode::Client => "CLIENT",
        AcknowledgementMode::DuplicateOk => "DUPLICATE_OK",
        AcknowledgementMode::Transacted => "TRANSACTED",
    };
    let message_frame = [
        message.message_id.clone(),
        message.destination.clone(),
        modernlink_core::base64_encode(&payload_bytes),
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
        payload_kind.to_string(),
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
    jni_guard(0, move || {
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
        register_client(NativeMessagingClient { transport, route })
    })
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
    jni_guard(0, move || {
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
        register_client(NativeMessagingClient { transport, route })
    })
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
    jni_guard(std::ptr::null_mut(), move || {
        let client = match client_for(handle) {
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
    })
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
    payload_kind: JString,
) -> jni::sys::jstring {
    jni_guard(std::ptr::null_mut(), move || {
        let client = match client_for(handle) {
            Some(value) => value,
            None => {
                return messaging_string_error("messaging client handle is invalid".to_string())
            }
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
            &payload_kind,
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
        // MSG-05: the payload arrives base64-encoded for every category, so a BytesMessage
        // is not mangled by a UTF-8 round trip on the way in. The category decides how those
        // bytes are interpreted; an unsupported one is refused here, before publishing.
        let payload_bytes = match modernlink_core::base64_decode(&values[2]) {
            Ok(value) => value,
            Err(error) => return messaging_string_error(error.to_string()),
        };
        let payload = match messaging_build_payload(&values[8], payload_bytes) {
            Ok(value) => value,
            Err(error) => return messaging_string_error(error),
        };
        let mut message = match MessageEnvelope::new(&values[1], payload, 0) {
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
    })
}

#[no_mangle]
pub extern "system" fn Java_com_modernlink_messaging_ModernMessagingClient_nativeReceive(
    env: JNIEnv,
    _class: JClass,
    handle: jlong,
) -> jni::sys::jstring {
    jni_guard(std::ptr::null_mut(), move || {
        let client = match client_for(handle) {
            Some(value) => value,
            None => {
                return messaging_string_error("messaging client handle is invalid".to_string())
            }
        };
        let received = match client.transport.receive() {
            Ok(Some(value)) => value,
            Ok(None) => {
                return messaging_string_error("no messaging message available".to_string())
            }
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
    })
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
    jni_guard(std::ptr::null_mut(), move || {
        let client = match client_for(handle) {
            Some(value) => value,
            None => {
                return messaging_string_error("messaging client handle is invalid".to_string())
            }
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
    })
}

#[no_mangle]
pub extern "system" fn Java_com_modernlink_messaging_ModernMessagingClient_nativeClose(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) {
    jni_guard((), move || {
        unregister_client(handle);
    })
}

/// MSG-04 — the provider guarantee table, readable without opening a connection.
///
/// Static and connectionless on purpose: a deployment must be able to ask "can this
/// provider honour what we need?" *before* it moves traffic, which is precisely when no
/// connection exists yet.
///
/// The frame is ten pipe-separated fields: the provider name, the eight guarantees in
/// declaration order, and the receive semantics (H-16). It carries capability metadata only — no endpoint,
/// no credential, no payload — so it is safe to log and safe for a JMX attribute.
#[no_mangle]
pub extern "system" fn Java_com_modernlink_messaging_ModernMessagingClient_nativeProviderGuarantees(
    mut env: JNIEnv,
    _class: JClass,
    provider: JString,
) -> jni::sys::jstring {
    jni_guard(std::ptr::null_mut(), move || {
        let name = match java_string(&mut env, &provider) {
            Some(value) => value,
            None => return messaging_string_error("invalid provider name".to_string()),
        };
        let provider = match messaging_provider(&name) {
            Ok(value) => value,
            Err(error) => return messaging_string_error(error),
        };
        let guarantees = provider.guarantees();
        let frame = format!(
            "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
            messaging_provider_name(provider),
            guarantees.persistence.as_str(),
            guarantees.ordering.as_str(),
            guarantees.server_side_acknowledgement.as_str(),
            guarantees.client_acknowledgement.as_str(),
            guarantees.transactions.as_str(),
            guarantees.redelivery.as_str(),
            guarantees.dead_lettering.as_str(),
            guarantees.replay.as_str(),
            guarantees.receive_semantics.as_str()
        );
        match env.new_string(frame) {
            Ok(value) => value.into_raw(),
            Err(error) => messaging_string_error(error.to_string()),
        }
    })
}

#[no_mangle]
pub extern "system" fn Java_com_modernlink_messaging_ModernMessagingClient_nativeLastError(
    env: JNIEnv,
    _class: JClass,
) -> jni::sys::jstring {
    jni_guard(std::ptr::null_mut(), move || {
        let message = LAST_ERROR.with(|value| value.borrow().clone());
        match env.new_string(message) {
            Ok(value) => value.into_raw(),
            Err(_) => std::ptr::null_mut(),
        }
    })
}

/// Live HTTP responses, keyed by an opaque handle (B-008, H-17).
///
/// Same defect and same fix as the messaging registry: the handle was the address of a
/// leaked `Box`, dereferenced after a null check and nothing else, so a stale or fabricated
/// `jlong` was a use-after-free inside the host JVM.
///
/// Narrower blast radius than the messaging client — a response is short-lived and used by
/// one caller — which is why it was filed as B-008 rather than folded into B-005. It is the
/// same unsafety.
///
/// `Arc` for the same reason as `CLIENTS`: a release racing an in-flight read frees after
/// that read finishes rather than under it.
static RESPONSES: std::sync::LazyLock<
    std::sync::Mutex<std::collections::HashMap<u64, std::sync::Arc<NativeResponse>>>,
> = std::sync::LazyLock::new(|| std::sync::Mutex::new(std::collections::HashMap::new()));

/// Shares the counter with the messaging registry so an id is unique across both. Mixing a
/// response handle and a client handle then misses in both maps instead of finding the wrong
/// object in one.
fn register_response(response: NativeResponse) -> jlong {
    let handle = NEXT_HANDLE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    match RESPONSES.lock() {
        Ok(mut responses) => {
            responses.insert(handle, std::sync::Arc::new(response));
            handle as jlong
        }
        Err(_) => set_error("HTTP response registry is unavailable".to_string()),
    }
}

fn response_for(handle: jlong) -> Option<std::sync::Arc<NativeResponse>> {
    if handle <= 0 {
        return None;
    }
    RESPONSES.lock().ok()?.get(&(handle as u64)).cloned()
}

/// Idempotent: releasing twice, or releasing an unknown handle, is a no-op rather than a
/// double free.
fn unregister_response(handle: jlong) {
    if handle <= 0 {
        return;
    }
    if let Ok(mut responses) = RESPONSES.lock() {
        responses.remove(&(handle as u64));
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
    jni_guard(0, move || {
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
        register_response(NativeResponse(result))
    })
}

#[no_mangle]
pub extern "system" fn Java_com_modernlink_LegacyHttpClient_nativeLastError(
    env: JNIEnv,
    _class: JClass,
) -> jni::sys::jstring {
    jni_guard(std::ptr::null_mut(), move || {
        let message = LAST_ERROR.with(|value| value.borrow().clone());
        match env.new_string(message) {
            Ok(value) => value.into_raw(),
            Err(_) => std::ptr::null_mut(),
        }
    })
}

#[no_mangle]
pub extern "system" fn Java_com_modernlink_LegacyHttpClient_nativeCapabilities(
    _env: JNIEnv,
    _class: JClass,
) -> jlong {
    jni_guard(0, move || 31)
}

#[no_mangle]
pub extern "system" fn Java_com_modernlink_ModernUuid_nativeV7(
    env: JNIEnv,
    _class: JClass,
) -> jni::sys::jstring {
    jni_guard(std::ptr::null_mut(), move || {
        match env.new_string(modernlink_core::uuid_v7()) {
            Ok(value) => value.into_raw(),
            Err(_) => std::ptr::null_mut(),
        }
    })
}

#[no_mangle]
pub extern "system" fn Java_com_modernlink_ModernUuid_nativeV4(
    env: JNIEnv,
    _class: JClass,
) -> jni::sys::jstring {
    jni_guard(std::ptr::null_mut(), move || {
        match env.new_string(modernlink_core::uuid_v4()) {
            Ok(value) => value.into_raw(),
            Err(_) => std::ptr::null_mut(),
        }
    })
}

#[no_mangle]
pub extern "system" fn Java_com_modernlink_ModernBase64_nativeEncode(
    env: JNIEnv,
    _class: JClass,
    value: JByteArray,
) -> jni::sys::jstring {
    jni_guard(std::ptr::null_mut(), move || {
        let value = match env.convert_byte_array(&value) {
            Ok(value) => value,
            Err(_) => return std::ptr::null_mut(),
        };
        match env.new_string(modernlink_core::base64_encode(&value)) {
            Ok(value) => value.into_raw(),
            Err(_) => std::ptr::null_mut(),
        }
    })
}

#[no_mangle]
pub extern "system" fn Java_com_modernlink_ModernBase64_nativeDecode(
    mut env: JNIEnv,
    _class: JClass,
    value: JString,
) -> jni::sys::jbyteArray {
    jni_guard(std::ptr::null_mut(), move || {
        let value = match env
            .get_string(&value)
            .ok()
            .and_then(|text| text.to_str().ok().map(str::to_owned))
        {
            Some(value) => value,
            None => return std::ptr::null_mut(),
        };
        match modernlink_core::base64_decode(&value)
            .ok()
            .and_then(|bytes| env.byte_array_from_slice(&bytes).ok())
        {
            Some(bytes) => bytes.into_raw(),
            None => std::ptr::null_mut(),
        }
    })
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
    jni_guard(std::ptr::null_mut(), move || {
        let fields = match java_string_array(&mut env, &fields) {
            Some(value) => value,
            None => return std::ptr::null_mut(),
        };
        match modernlink_core::json_object(&fields)
            .ok()
            .and_then(|value| env.new_string(value).ok())
        {
            Some(value) => value.into_raw(),
            None => std::ptr::null_mut(),
        }
    })
}

#[no_mangle]
pub extern "system" fn Java_com_modernlink_ModernJson_nativeArray(
    mut env: JNIEnv,
    _class: JClass,
    values: JObjectArray,
) -> jni::sys::jstring {
    jni_guard(std::ptr::null_mut(), move || {
        let values = match java_string_array(&mut env, &values) {
            Some(value) => value,
            None => return std::ptr::null_mut(),
        };
        match modernlink_core::json_array(&values)
            .ok()
            .and_then(|value| env.new_string(value).ok())
        {
            Some(value) => value.into_raw(),
            None => std::ptr::null_mut(),
        }
    })
}

#[no_mangle]
pub extern "system" fn Java_com_modernlink_ModernJson_nativeDecode(
    mut env: JNIEnv,
    _class: JClass,
    json: JString,
) -> jni::sys::jstring {
    jni_guard(std::ptr::null_mut(), move || {
        let json = match env
            .get_string(&json)
            .ok()
            .and_then(|text| text.to_str().ok().map(str::to_owned))
        {
            Some(value) => value,
            None => return std::ptr::null_mut(),
        };
        match modernlink_core::json_decode(&json)
            .ok()
            .and_then(|value| env.new_string(value).ok())
        {
            Some(value) => value.into_raw(),
            None => std::ptr::null_mut(),
        }
    })
}

#[no_mangle]
pub extern "system" fn Java_com_modernlink_LegacyHttpClient_nativeStatus(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) -> jint {
    jni_guard(0, move || {
        response_for(handle)
            .map(|value| value.0.status as jint)
            .unwrap_or(0)
    })
}

#[no_mangle]
pub extern "system" fn Java_com_modernlink_LegacyHttpClient_nativeStatusMessage(
    env: JNIEnv,
    _class: JClass,
    handle: jlong,
) -> jni::sys::jstring {
    jni_guard(std::ptr::null_mut(), move || {
        match response_for(handle)
            .map(|value| value.0.status_message.clone())
            .and_then(|value| env.new_string(value).ok())
        {
            Some(value) => value.into_raw(),
            None => std::ptr::null_mut(),
        }
    })
}

#[no_mangle]
pub extern "system" fn Java_com_modernlink_LegacyHttpClient_nativeHeaders(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
) -> jobjectArray {
    jni_guard(std::ptr::null_mut(), move || {
        // Bind the Arc first: `&value.0.headers` on a temporary would borrow past its
        // lifetime, and cloning the whole header map to avoid that would be wasteful.
        let Some(response) = response_for(handle) else {
            return std::ptr::null_mut();
        };
        let headers = &response.0.headers;
        let string_class = match env.find_class("java/lang/String") {
            Ok(value) => value,
            Err(_) => return std::ptr::null_mut(),
        };
        let array = match env.new_object_array(
            (headers.len() * 2) as i32,
            string_class,
            JString::default(),
        ) {
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
    })
}

#[no_mangle]
pub extern "system" fn Java_com_modernlink_LegacyHttpClient_nativeBody(
    env: JNIEnv,
    _class: JClass,
    handle: jlong,
) -> jbyteArray {
    jni_guard(std::ptr::null_mut(), move || {
        // The Arc is bound before borrowing from it: borrowing a temporary would not
        // outlive the statement, and cloning the whole body to dodge that would copy the
        // entire response payload.
        let Some(response) = response_for(handle) else {
            return std::ptr::null_mut();
        };
        let body = &response.0.body;
        match env.byte_array_from_slice(body) {
            Ok(value) => value.into_raw(),
            Err(_) => std::ptr::null_mut(),
        }
    })
}

#[no_mangle]
pub extern "system" fn Java_com_modernlink_LegacyHttpClient_nativeTlsCertificates(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
) -> jobjectArray {
    jni_guard(std::ptr::null_mut(), move || {
        let Some(response) = response_for(handle) else {
            return std::ptr::null_mut();
        };
        let Some(info) = response.0.tls.as_ref() else {
            return std::ptr::null_mut();
        };
        let certificates = &info.peer_certificates_der;
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
    })
}

fn tls_string(handle: jlong, protocol: bool) -> Option<String> {
    {
        response_for(handle)
            .and_then(|value| value.0.tls.as_ref().cloned())
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
    jni_guard(std::ptr::null_mut(), move || {
        match response_for(handle)
            .map(|value| value.0.final_url.clone())
            .and_then(|value| env.new_string(value).ok())
        {
            Some(value) => value.into_raw(),
            None => std::ptr::null_mut(),
        }
    })
}

#[no_mangle]
pub extern "system" fn Java_com_modernlink_LegacyHttpClient_nativeTlsProtocol(
    env: JNIEnv,
    _class: JClass,
    handle: jlong,
) -> jni::sys::jstring {
    jni_guard(std::ptr::null_mut(), move || {
        match tls_string(handle, true).and_then(|value| env.new_string(value).ok()) {
            Some(value) => value.into_raw(),
            None => std::ptr::null_mut(),
        }
    })
}

#[no_mangle]
pub extern "system" fn Java_com_modernlink_LegacyHttpClient_nativeTlsCipherSuite(
    env: JNIEnv,
    _class: JClass,
    handle: jlong,
) -> jni::sys::jstring {
    jni_guard(std::ptr::null_mut(), move || {
        match tls_string(handle, false).and_then(|value| env.new_string(value).ok()) {
            Some(value) => value.into_raw(),
            None => std::ptr::null_mut(),
        }
    })
}

#[no_mangle]
pub extern "system" fn Java_com_modernlink_LegacyHttpClient_nativeRelease(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) {
    jni_guard((), move || {
        unregister_response(handle);
    })
}

#[cfg(test)]
mod tests;
