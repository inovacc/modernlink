//! The provider-neutral message domain and its transports.
//!
//! One uniform transport boundary ([`MessageTransportKind`]) fronts `InMemoryTransport`
//! (the in-process `LEGACY_JMS` compatibility path), NATS, NATS JetStream, Kafka, Pulsar, and
//! RabbitMQ, so the JNI surface never names a provider-specific type. Trace context is
//! first-class envelope data, not a user property: adapters must not replace or discard it.
//!
//! Implemented capability checks report gaps explicitly instead of rerouting to a weaker
//! transport. The publish path still has the B-003 delivery-mode enforcement gap. Recorded
//! commands have reached each broker transport and the packaged Java 6 facade has reached NATS;
//! those machine facts do not establish vendor-host compatibility or failure semantics
//! (`docs/VERIFICATION.md`).

#[cfg(any(feature = "nats", feature = "pulsar"))]
use futures_util::StreamExt;
#[cfg(feature = "rabbitmq")]
use lapin::acker::Acker;
#[cfg(feature = "rabbitmq")]
use lapin::options::{BasicAckOptions, BasicGetOptions, BasicPublishOptions, QueueDeclareOptions};
#[cfg(feature = "rabbitmq")]
use lapin::types::FieldTable;
#[cfg(feature = "rabbitmq")]
use lapin::{BasicProperties, Connection, ConnectionProperties};
#[cfg(feature = "pulsar")]
use pulsar::consumer::{Consumer as PulsarConsumer, Message as PulsarMessage};
#[cfg(feature = "pulsar")]
use pulsar::producer::Message as PulsarProducerMessage;
#[cfg(feature = "pulsar")]
use pulsar::{Pulsar, TokioExecutor};
#[cfg(feature = "kafka")]
use rdkafka::admin::{AdminClient, AdminOptions, NewTopic, TopicReplication};
#[cfg(feature = "kafka")]
use rdkafka::client::DefaultClientContext;
#[cfg(feature = "kafka")]
use rdkafka::consumer::{CommitMode, Consumer, StreamConsumer};
#[cfg(feature = "kafka")]
use rdkafka::producer::{FutureProducer, FutureRecord};
#[cfg(feature = "kafka")]
use rdkafka::types::RDKafkaErrorCode;
#[cfg(feature = "kafka")]
use rdkafka::{ClientConfig, Message as KafkaMessage, Offset, TopicPartitionList};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;
use std::sync::{Arc, Mutex};
#[cfg(feature = "pulsar")]
use std::thread;
#[cfg(feature = "nats")]
use std::time::Duration;

pub const ENVELOPE_VERSION: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mode {
    Transparent,
    Transform,
    Redirect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Provider {
    LegacyJms,
    Kafka,
    Pulsar,
    Nats,
    NatsJetStream,
    RabbitMq,
}

/// How long a broker connect may take before it is refused (H-02, BACKLOG P1).
///
/// Ten seconds, matching the receive deadline the broker-backed tests already use, so the
/// two bounds in this crate agree rather than each being picked separately.
// Which of these two is live depends on which providers are compiled in: a nats-only build
// never does a control-plane call, and a kafka-only build never uses the connect default
// (its one bounded operation is topic creation). Enumerating that in `cfg` is brittle - it
// was got wrong three times in a row, once per permutation - so both are gated together on
// "any provider" and the dead-code lint is silenced with the reason stated. The alternative
// is a cfg expression nobody can verify by reading.
#[cfg(any(
    feature = "nats",
    feature = "kafka",
    feature = "pulsar",
    feature = "rabbitmq"
))]
#[allow(dead_code)]
const DEFAULT_CONNECT_TIMEOUT_SECS: u64 = 10;

// Control-plane operations get the longer default. Declaring a queue or creating a topic on
// a loaded cluster is legitimately slower than a TCP handshake, and a single bound tight
// enough to catch a hung connect would reject healthy deployments.
#[cfg(any(
    feature = "nats",
    feature = "kafka",
    feature = "pulsar",
    feature = "rabbitmq"
))]
#[allow(dead_code)]
const DEFAULT_ADMIN_TIMEOUT_SECS: u64 = 30;

/// Read a deadline, allowing the deployment to override the built-in default.
///
/// No number chosen here can be right for every broker and network, so `fallback_secs` is a
/// starting point rather than a policy. `MODERNLINK_BROKER_TIMEOUT_SECS` overrides it.
///
/// An unparseable or zero value falls back to the default rather than being honoured: `0`
/// would mean "time out immediately", which is a footgun that would look like a broker
/// outage. A bad value is a configuration mistake, and silently disabling the bound - or
/// making every connect fail - are both worse than ignoring it.
#[cfg(any(
    feature = "nats",
    feature = "kafka",
    feature = "pulsar",
    feature = "rabbitmq"
))]
fn broker_timeout(fallback_secs: u64) -> std::time::Duration {
    let configured = std::env::var("MODERNLINK_BROKER_TIMEOUT_SECS")
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|seconds| *seconds > 0);
    std::time::Duration::from_secs(configured.unwrap_or(fallback_secs))
}

/// Run a broker operation on `runtime` with a hard deadline.
///
/// Every transport used to call `runtime.block_on(connect(...))` with no bound. A broker
/// that completes the TCP handshake and then stalls — or a firewall that DROPs rather than
/// REJECTs — hung the calling thread forever. That thread belongs to the vendor-locked
/// Java 6 application, reached through a JNI call it cannot cancel, so "forever" means
/// until the JVM is restarted.
///
/// Expiry is a `DomainError::Transport`, so the caller is told the operation was refused
/// rather than being left to block. Failing closed on a deadline is the same contract the
/// rest of this crate follows for a capability it cannot honour.
///
/// `what` names the operation in the message; it must never contain an endpoint, because
/// endpoints carry credentials (see `docs/BUGS.md` B-006).
#[cfg(any(
    test,
    feature = "nats",
    feature = "kafka",
    feature = "pulsar",
    feature = "rabbitmq"
))]
fn block_on_with_timeout<T>(
    runtime: &tokio::runtime::Runtime,
    what: &str,
    timeout: std::time::Duration,
    future: impl std::future::Future<Output = T>,
) -> Result<T, DomainError> {
    runtime.block_on(async move {
        match tokio::time::timeout(timeout, future).await {
            Ok(value) => Ok(value),
            Err(_) => Err(DomainError::Transport(format!(
                "{} did not complete within {}s and was refused rather than left to block \
                 the calling thread",
                what,
                timeout.as_secs()
            ))),
        }
    })
}

/// Strip credentials out of any text before it can become an error message (B-006, H-03).
///
/// Broker endpoints carry credentials inline — the documented RabbitMQ form is
/// `amqp://guest:guest@host:5672/%2f`. Provider crates put the endpoint they failed on into
/// their error `Display`, that string crosses the JNI boundary as a
/// `LegacyHttpException` message, and the host application logs it. AGENTS.md is flat:
/// *"Never put credentials, payloads, or message bodies in JMX attributes or logs."*
///
/// This scrubs the **message**, not just the endpoint we passed in, because the text comes
/// from `lapin`/`async-nats`/`rdkafka`/`pulsar` and we do not control what they embed. It
/// rewrites the userinfo of every `scheme://user:pass@host` it finds to `***`.
///
/// Deliberately conservative about what counts as userinfo: only an `@` appearing before the
/// next `/`, `?`, `#` or whitespace after `://`. An `@` inside a path or query is not
/// credentials, and blanking it would corrupt an otherwise useful diagnostic.
///
/// **Known limit:** a credential with no scheme in front of it — a bare `user:pass@host` —
/// is not redacted, because without a scheme there is nothing to distinguish it from
/// ordinary prose containing a colon and an `@`. Redacting on that shape alone would eat
/// email addresses and timestamps out of diagnostics. No provider crate is known to emit
/// that form; if one turns up, the fix is to match it for the specific provider rather than
/// to loosen this rule globally.
///
/// Verified by `scrubber_edge_cases` against: no-password userinfo, `amqps://`/`nats+tls://`,
/// uppercase schemes, a URL ending the string, a percent-encoded `@` in the password,
/// quoted and parenthesised URLs, and multi-byte UTF-8 adjacent to `://` and `@` (which
/// would otherwise risk a panic on a non-char-boundary slice).
// Gated like the transports themselves: every call site lives inside a provider, so in a
// broker-free build these would be dead code. `test` is included because the redaction
// tests must run in the default gate too - that is where a regression would otherwise hide.
#[cfg(any(
    test,
    feature = "nats",
    feature = "kafka",
    feature = "pulsar",
    feature = "rabbitmq"
))]
fn redact_credentials(message: &str) -> String {
    let mut out = String::with_capacity(message.len());
    let mut rest = message;
    while let Some(scheme_end) = rest.find("://") {
        let after = scheme_end + 3;
        out.push_str(&rest[..after]);
        let authority_end = rest[after..]
            .find(|c: char| c == '/' || c == '?' || c == '#' || c.is_whitespace())
            .map(|i| after + i)
            .unwrap_or(rest.len());
        let authority = &rest[after..authority_end];
        match authority.rfind('@') {
            Some(at) => {
                out.push_str("***");
                out.push_str(&authority[at..]);
            }
            None => out.push_str(authority),
        }
        rest = &rest[authority_end..];
    }
    out.push_str(rest);
    out
}

/// Build a transport error with credentials removed.
///
/// Every `DomainError::Transport` built from a provider error goes through this. Doing it at
/// construction rather than at the JNI boundary means a future caller of this crate cannot
/// reintroduce the leak by formatting the error somewhere else.
// Gated like the transports themselves: every call site lives inside a provider, so in a
// broker-free build these would be dead code. `test` is included because the redaction
// tests must run in the default gate too - that is where a regression would otherwise hide.
#[cfg(any(
    test,
    feature = "nats",
    feature = "kafka",
    feature = "pulsar",
    feature = "rabbitmq"
))]
fn transport_error(error: impl std::fmt::Display) -> DomainError {
    DomainError::Transport(redact_credentials(&error.to_string()))
}

/// Put a borrowed-out value back in its slot even if the work in between unwinds (B-007).
///
/// `NatsTransport::receive` and `NatsJetStreamTransport::receive` take their subscription
/// out of a `Mutex<Option<_>>`, await the next message, and put it back afterwards. If the
/// await unwinds, the value is dropped on the stack and the slot stays `None` **for the life
/// of the client** — every later `receive` then reports "unavailable" on a handle that still
/// looks open. The `Mutex` is not poisoned by this, because the guard is released before the
/// await, so Rust's own protection never fires.
///
/// Before H-01 that unwind was undefined behaviour crossing into the JVM, i.e. usually a
/// crash. Containing the panic converted it into this silent permanent degradation, which is
/// an improvement and a new failure mode. This closes it: restoration happens in `Drop`, so
/// the normal path and the unwinding path take the same route.
// Only the two NATS receive paths borrow out of a slot like this, so this is dead in every
// other build. `test` is included so the B-007 regression tests run in the default gate --
// that is where a regression would otherwise hide.
#[cfg(any(test, feature = "nats"))]
struct RestoreOnDrop<'a, T> {
    slot: &'a Mutex<Option<T>>,
    value: Option<T>,
}

// Only the two NATS receive paths borrow out of a slot like this, so this is dead in every
// other build. `test` is included so the B-007 regression tests run in the default gate --
// that is where a regression would otherwise hide.
#[cfg(any(test, feature = "nats"))]
impl<'a, T> RestoreOnDrop<'a, T> {
    /// Take the value out, or report `missing` if the slot is empty or the lock is poisoned.
    fn take_from(slot: &'a Mutex<Option<T>>, missing: &str) -> Result<Self, DomainError> {
        let value = slot
            .lock()
            .map_err(|_| DomainError::Transport(missing.to_string()))?
            .take()
            .ok_or_else(|| DomainError::Transport(missing.to_string()))?;
        Ok(Self {
            slot,
            value: Some(value),
        })
    }

    /// Borrow the taken value.
    ///
    /// Returns `Option` rather than unwrapping an invariant: the value is `Some` for this
    /// type's whole lifetime except inside `Drop`, so an `expect` here would be provably
    /// unreachable — and an unreachable panic is still a panic path in a library that just
    /// spent H-01 removing them.
    fn value_mut(&mut self) -> Option<&mut T> {
        self.value.as_mut()
    }
}

// Only the two NATS receive paths borrow out of a slot like this, so this is dead in every
// other build. `test` is included so the B-007 regression tests run in the default gate --
// that is where a regression would otherwise hide.
#[cfg(any(test, feature = "nats"))]
impl<T> Drop for RestoreOnDrop<'_, T> {
    fn drop(&mut self) {
        if let Some(value) = self.value.take() {
            // A poisoned lock means another thread panicked holding it. Nothing useful can
            // be done here and Drop must not panic, so the value is dropped rather than
            // restored — the slot was already unusable in that case.
            if let Ok(mut slot) = self.slot.lock() {
                *slot = Some(value);
            }
        }
    }
}

/// Does this endpoint ask for an encrypted connection? (H-07)
///
/// Checked by scheme because that is the only TLS signal this API carries — there is no
/// way for a caller to request TLS explicitly, which is itself the gap H-07 names.
///
/// Used to FAIL CLOSED where the transport cannot honour the request. Silently connecting
/// in plaintext to an endpoint the caller marked as encrypted is the worst outcome
/// available: the deployment believes its broker traffic and credentials are protected, and
/// nothing anywhere says otherwise.
// Only Kafka consults this today, because it is the only transport that cannot honour a
// TLS request at all. `test` is included so the scheme-recognition tests run in the default
// gate. When the other transports gain explicit TLS selection this widens with them.
#[cfg(any(test, feature = "kafka"))]
fn endpoint_requests_tls(endpoint: &str) -> bool {
    let lower = endpoint.trim().to_ascii_lowercase();
    [
        "amqps://",
        "nats+tls://",
        "tls://",
        "ssl://",
        "sasl_ssl://",
        "pulsar+ssl://",
        "https://",
    ]
    .iter()
    .any(|prefix| lower.starts_with(prefix))
}

/// What `MessageTransport::receive` does when no message is waiting (H-16, B-010).
///
/// The signature says `Result<Option<ReceivedMessage>, _>`, which promises "there may be no
/// message". That promise holds for two of six providers. The other four block until one
/// arrives, so their `Ok(None)` is unreachable and the call simply never returns.
///
/// This matters more than it looks. A legacy JMS application polling for work expects
/// `receiveNoWait()` semantics — call, get null, do something else. On a blocking provider
/// that call never comes back, through a JNI boundary the application cannot cancel. It is
/// exactly the kind of capability gap MSG-04 exists to make visible before traffic moves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReceiveSemantics {
    /// Returns `Ok(None)` promptly when nothing is waiting.
    NonBlocking,
    /// Blocks until a message arrives. **There is no timeout and no way to cancel.**
    BlocksIndefinitely,
}

impl ReceiveSemantics {
    pub fn as_str(self) -> &'static str {
        match self {
            ReceiveSemantics::NonBlocking => "NON_BLOCKING",
            ReceiveSemantics::BlocksIndefinitely => "BLOCKS_INDEFINITELY",
        }
    }
}

/// How well a guarantee is backed, for one provider.
///
/// MSG-04 exists so a capability gap is **queryable before traffic moves**, and a gap is
/// only useful if the reader can tell a measured guarantee from a claimed one. Three
/// levels, not two, because "the transport implements it" and "a test proved it" are
/// different statements and this project has been bitten by conflating them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Support {
    /// The transport implements it **and** a test has exercised it against a real broker.
    Verified,
    /// The transport implements it. **No test has exercised it.** Treat as a claim.
    Declared,
    /// The provider cannot offer it, or this transport does not implement it. Asking for
    /// it must be refused, never quietly downgraded.
    Unsupported,
}

impl Support {
    /// True only for `Verified`. Deliberately not true for `Declared`: a caller deciding
    /// whether to move production traffic must not be handed a claim dressed as evidence.
    pub fn is_proven(self) -> bool {
        matches!(self, Support::Verified)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Support::Verified => "VERIFIED",
            Support::Declared => "DECLARED",
            Support::Unsupported => "UNSUPPORTED",
        }
    }
}

/// What one provider is declared to offer -- **MSG-04**.
///
/// This table is assembled from the transport implementations in this file, not from
/// vendor documentation and not from a benchmark. Anything marked [`Support::Declared`]
/// has never been executed against a broker; see `docs/providers.md` for the per-field
/// reasoning and `docs/BUGS.md` for the gaps this table exposed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderGuarantees {
    pub provider: Provider,
    /// Messages survive a broker restart.
    pub persistence: Support,
    /// Delivery order is preserved within one destination (or partition).
    pub ordering: Support,
    /// The broker tracks acknowledgement state, so an unacknowledged message is
    /// redelivered rather than lost with the consumer.
    pub server_side_acknowledgement: Support,
    /// `AcknowledgementMode::Client` is honoured end to end.
    pub client_acknowledgement: Support,
    /// Transactional publish/consume.
    pub transactions: Support,
    /// An unacknowledged message is redelivered.
    pub redelivery: Support,
    /// Poison messages can be diverted to a dead-letter destination.
    pub dead_lettering: Support,
    /// Already-consumed messages can be re-read.
    pub replay: Support,
    /// What `receive` does when nothing is waiting -- H-16.
    pub receive_semantics: ReceiveSemantics,
}

impl ProviderGuarantees {
    /// Fail closed on a delivery mode the provider cannot honour.
    ///
    /// AGENTS.md: "A capability gap must be reported explicitly - never silently
    /// degraded." This is the check that makes that enforceable by a caller.
    ///
    /// **It is not called on the publish path yet, and that is deliberate.**
    /// `MessageEnvelope::new` defaults to `DeliveryMode::Persistent`, so wiring this in
    /// would start refusing every default NATS-core publish -- a change to delivery
    /// semantics that only the maintainer should make. See `docs/BUGS.md` B-003.
    pub fn require_delivery_mode(&self, mode: DeliveryMode) -> Result<(), DomainError> {
        match (mode, self.persistence) {
            (DeliveryMode::NonPersistent, _) => Ok(()),
            (DeliveryMode::Persistent, Support::Unsupported) => {
                Err(DomainError::Unsupported(format!(
                    "provider {:?} cannot offer persistent delivery; the request was refused rather than downgraded to non-persistent",
                    self.provider
                )))
            }
            (DeliveryMode::Persistent, _) => Ok(()),
        }
    }

    /// Fail closed on an acknowledgement mode the provider cannot honour.
    pub fn require_acknowledgement_mode(
        &self,
        mode: AcknowledgementMode,
    ) -> Result<(), DomainError> {
        let (needed, name) = match mode {
            AcknowledgementMode::Auto | AcknowledgementMode::DuplicateOk => return Ok(()),
            AcknowledgementMode::Client => (self.client_acknowledgement, "CLIENT"),
            AcknowledgementMode::Transacted => (self.transactions, "TRANSACTED"),
        };
        if needed == Support::Unsupported {
            return Err(DomainError::Unsupported(format!(
                "provider {:?} does not support {} acknowledgement; the request was refused rather than downgraded",
                self.provider, name
            )));
        }
        Ok(())
    }
}

impl Provider {
    /// The guarantee table for this provider -- **MSG-04**.
    ///
    /// Every `Verified` entry below is backed by a test that has actually run. Today that
    /// is only the happy-path send/receive/ack proven on 2026-08-14 for NATS core,
    /// JetStream and RabbitMQ, so `Verified` appears sparingly and on purpose.
    pub fn guarantees(self) -> ProviderGuarantees {
        use Support::{Declared, Unsupported, Verified};
        match self {
            // In-process VecDeque. Nothing survives the process, and that is the point:
            // it is a compatibility fixture, not a broker.
            Provider::LegacyJms => ProviderGuarantees {
                provider: self,
                persistence: Unsupported,
                ordering: Verified,
                server_side_acknowledgement: Unsupported,
                client_acknowledgement: Verified,
                transactions: Unsupported,
                redelivery: Unsupported,
                dead_lettering: Unsupported,
                replay: Unsupported,
                receive_semantics: ReceiveSemantics::NonBlocking,
            },
            // Core NATS is fire-and-forget pub/sub. There is no broker-side state, so an
            // unacknowledged message is simply gone. `acknowledge` returns Acknowledged
            // because the local receipt advances -- not because a server confirmed it.
            Provider::Nats => ProviderGuarantees {
                provider: self,
                persistence: Unsupported,
                ordering: Declared,
                server_side_acknowledgement: Unsupported,
                client_acknowledgement: Unsupported,
                transactions: Unsupported,
                redelivery: Unsupported,
                dead_lettering: Unsupported,
                replay: Unsupported,
                receive_semantics: ReceiveSemantics::BlocksIndefinitely,
            },
            // JetStream keeps a stream and a durable pull consumer with
            // AckPolicy::Explicit, so acknowledgement is genuinely server-side.
            Provider::NatsJetStream => ProviderGuarantees {
                provider: self,
                persistence: Declared,
                ordering: Declared,
                server_side_acknowledgement: Verified,
                client_acknowledgement: Verified,
                transactions: Unsupported,
                redelivery: Declared,
                dead_lettering: Unsupported,
                replay: Declared,
                receive_semantics: ReceiveSemantics::BlocksIndefinitely,
            },
            // Kafka commits offsets with CommitMode::Sync. Ordering holds per partition,
            // not per topic -- this transport does not choose partitions, so ordering is
            // only as strong as the default partitioner makes it.
            Provider::Kafka => ProviderGuarantees {
                provider: self,
                persistence: Declared,
                ordering: Declared,
                server_side_acknowledgement: Declared,
                client_acknowledgement: Declared,
                transactions: Unsupported,
                redelivery: Declared,
                dead_lettering: Unsupported,
                replay: Declared,
                receive_semantics: ReceiveSemantics::BlocksIndefinitely,
            },
            Provider::Pulsar => ProviderGuarantees {
                provider: self,
                persistence: Declared,
                ordering: Declared,
                server_side_acknowledgement: Declared,
                client_acknowledgement: Declared,
                transactions: Unsupported,
                redelivery: Declared,
                dead_lettering: Unsupported,
                replay: Declared,
                receive_semantics: ReceiveSemantics::BlocksIndefinitely,
            },
            // The queue is declared durable, but the publisher sends
            // BasicProperties::default(), which is delivery_mode 1 (transient). A durable
            // queue holding transient messages loses them on restart, so persistence is
            // NOT delivered here today. Marked Unsupported rather than Declared because
            // recording the intent instead of the behaviour is how this table would start
            // lying. See docs/BUGS.md B-003.
            Provider::RabbitMq => ProviderGuarantees {
                provider: self,
                persistence: Unsupported,
                ordering: Declared,
                server_side_acknowledgement: Verified,
                client_acknowledgement: Verified,
                transactions: Unsupported,
                redelivery: Declared,
                dead_lettering: Unsupported,
                replay: Unsupported,
                receive_semantics: ReceiveSemantics::NonBlocking,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeliveryMode {
    NonPersistent,
    Persistent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AcknowledgementMode {
    Auto,
    Client,
    DuplicateOk,
    Transacted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeliveryState {
    Published,
    Received,
    Acknowledged,
    Rejected,
    Retried,
    DeadLettered,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Payload {
    Text(String),
    Bytes(Vec<u8>),
    Map(BTreeMap<String, String>),
    Stream(Vec<u8>),
    Object {
        content_type: String,
        bytes: Vec<u8>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetryState {
    pub attempts: u32,
    pub max_attempts: u32,
    pub dead_letter_destination: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceContext {
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub trace_state: Option<String>,
    pub sampled: bool,
}

impl TraceContext {
    pub fn new() -> Self {
        let trace_id = uuid::Uuid::new_v4().to_string().replace('-', "");
        let span_id = uuid::Uuid::new_v4().to_string().replace('-', "")[..16].to_string();
        Self {
            trace_id,
            span_id,
            parent_span_id: None,
            trace_state: None,
            sampled: true,
        }
    }
}

impl Default for TraceContext {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for RetryState {
    fn default() -> Self {
        Self {
            attempts: 0,
            max_attempts: 5,
            dead_letter_destination: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageEnvelope {
    pub version: u16,
    pub message_id: String,
    pub correlation_id: Option<String>,
    pub timestamp_millis: i64,
    pub expiration_millis: Option<i64>,
    pub priority: u8,
    pub delivery_mode: DeliveryMode,
    pub acknowledgement_mode: AcknowledgementMode,
    pub destination: String,
    pub source: Option<String>,
    pub tenant: Option<String>,
    pub headers: BTreeMap<String, String>,
    pub properties: BTreeMap<String, String>,
    pub tracing: TraceContext,
    pub payload: Payload,
    pub retry: RetryState,
    pub idempotency_key: Option<String>,
}

impl MessageEnvelope {
    pub fn new(
        destination: &str,
        payload: Payload,
        timestamp_millis: i64,
    ) -> Result<Self, DomainError> {
        if destination.trim().is_empty() {
            return Err(DomainError::InvalidEnvelope(
                "destination must not be empty".to_string(),
            ));
        }
        Ok(Self {
            version: ENVELOPE_VERSION,
            message_id: uuid::Uuid::now_v7().to_string(),
            correlation_id: None,
            timestamp_millis,
            expiration_millis: None,
            priority: 4,
            delivery_mode: DeliveryMode::Persistent,
            acknowledgement_mode: AcknowledgementMode::Auto,
            destination: destination.to_string(),
            source: None,
            tenant: None,
            headers: BTreeMap::new(),
            properties: BTreeMap::new(),
            tracing: TraceContext::default(),
            payload,
            retry: RetryState::default(),
            idempotency_key: None,
        })
    }

    pub fn to_json(&self) -> Result<String, DomainError> {
        serde_json::to_string(self).map_err(|error| DomainError::Serialization(error.to_string()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainError {
    InvalidEnvelope(String),
    InvalidRoute(String),
    Serialization(String),
    Transport(String),
    /// A guarantee the selected provider cannot honour was requested -- MSG-04.
    ///
    /// Distinct from `Transport` on purpose: a transport error means the attempt failed,
    /// while this means the attempt was never made because honouring it was impossible.
    /// Collapsing the two would let a capability gap read as a transient outage, and the
    /// caller would retry forever instead of choosing another provider.
    Unsupported(String),
}

impl fmt::Display for DomainError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidEnvelope(message)
            | Self::InvalidRoute(message)
            | Self::Serialization(message)
            | Self::Transport(message)
            | Self::Unsupported(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for DomainError {}

pub trait MessageTransport: Send + Sync {
    fn provider(&self) -> Provider;
    fn publish(&self, message: MessageEnvelope) -> Result<DeliveryReceipt, DomainError>;
    fn receive(&self) -> Result<Option<ReceivedMessage>, DomainError>;
    fn acknowledge(&self, receipt: &DeliveryReceipt) -> Result<DeliveryReceipt, DomainError>;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeliveryReceipt {
    pub message_id: String,
    pub provider: Provider,
    pub state: DeliveryState,
    pub trace_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceivedMessage {
    pub message: MessageEnvelope,
    pub receipt: DeliveryReceipt,
}

#[derive(Clone)]
pub struct InMemoryTransport {
    provider: Provider,
    queue: Arc<Mutex<VecDeque<MessageEnvelope>>>,
    acknowledged: Arc<Mutex<BTreeSet<String>>>,
}

impl InMemoryTransport {
    pub fn new(provider: Provider) -> Self {
        Self {
            provider,
            queue: Arc::new(Mutex::new(VecDeque::new())),
            acknowledged: Arc::new(Mutex::new(BTreeSet::new())),
        }
    }
}

impl MessageTransport for InMemoryTransport {
    fn provider(&self) -> Provider {
        self.provider
    }

    fn publish(&self, message: MessageEnvelope) -> Result<DeliveryReceipt, DomainError> {
        let receipt = DeliveryReceipt {
            message_id: message.message_id.clone(),
            provider: self.provider,
            state: DeliveryState::Published,
            trace_id: message.tracing.trace_id.clone(),
        };
        self.queue
            .lock()
            .map_err(|_| DomainError::Transport("transport queue is unavailable".to_string()))?
            .push_back(message);
        Ok(receipt)
    }

    fn receive(&self) -> Result<Option<ReceivedMessage>, DomainError> {
        let message = self
            .queue
            .lock()
            .map_err(|_| DomainError::Transport("transport queue is unavailable".to_string()))?
            .pop_front();
        Ok(message.map(|message| ReceivedMessage {
            receipt: DeliveryReceipt {
                message_id: message.message_id.clone(),
                provider: self.provider,
                state: DeliveryState::Received,
                trace_id: message.tracing.trace_id.clone(),
            },
            message,
        }))
    }

    fn acknowledge(&self, receipt: &DeliveryReceipt) -> Result<DeliveryReceipt, DomainError> {
        if receipt.provider != self.provider {
            return Err(DomainError::Transport(
                "receipt provider does not match transport".to_string(),
            ));
        }
        self.acknowledged
            .lock()
            .map_err(|_| {
                DomainError::Transport("acknowledgement store is unavailable".to_string())
            })?
            .insert(receipt.message_id.clone());
        Ok(DeliveryReceipt {
            state: DeliveryState::Acknowledged,
            ..receipt.clone()
        })
    }
}

#[cfg(feature = "nats")]
pub struct NatsTransport {
    client: Option<async_nats::Client>,
    subject: String,
    subscription: Mutex<Option<async_nats::Subscriber>>,
    acknowledged: Mutex<BTreeSet<String>>,
    runtime: Option<tokio::runtime::Runtime>,
}

#[cfg(feature = "nats")]
impl NatsTransport {
    pub fn connect(url: &str, subject: &str) -> Result<Self, DomainError> {
        if subject.trim().is_empty() {
            return Err(DomainError::InvalidEnvelope(
                "NATS subject must not be empty".to_string(),
            ));
        }
        let runtime = tokio::runtime::Runtime::new().map_err(transport_error)?;
        // H-02: bounded. An unreachable-but-not-refusing broker used to hang this thread
        // forever, and it is the legacy application's thread.
        let (client, subscription) = block_on_with_timeout(
            &runtime,
            "NATS connect",
            broker_timeout(DEFAULT_CONNECT_TIMEOUT_SECS),
            async {
                let client = async_nats::connect(url).await.map_err(transport_error)?;
                let subscription = client
                    .subscribe(subject.to_string())
                    .await
                    .map_err(transport_error)?;
                Ok::<_, DomainError>((client, subscription))
            },
        )??;
        Ok(Self {
            client: Some(client),
            subject: subject.to_string(),
            subscription: Mutex::new(Some(subscription)),
            acknowledged: Mutex::new(BTreeSet::new()),
            runtime: Some(runtime),
        })
    }
}

#[cfg(feature = "nats")]
impl Drop for NatsTransport {
    fn drop(&mut self) {
        let subscription = self
            .subscription
            .lock()
            .ok()
            .and_then(|mut subscription| subscription.take());
        let client = self.client.take();

        if let Some(runtime) = self.runtime.take() {
            runtime.block_on(async move {
                drop(subscription);
                drop(client);
            });
        }
    }
}

#[cfg(feature = "nats")]
impl MessageTransport for NatsTransport {
    fn provider(&self) -> Provider {
        Provider::Nats
    }

    fn publish(&self, message: MessageEnvelope) -> Result<DeliveryReceipt, DomainError> {
        let message_id = message.message_id.clone();
        let trace_id = message.tracing.trace_id.clone();
        let payload = serde_json::to_vec(&message)
            .map_err(|error| DomainError::Serialization(error.to_string()))?;
        let runtime = self
            .runtime
            .as_ref()
            .ok_or_else(|| DomainError::Transport("NATS runtime is unavailable".to_string()))?;
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| DomainError::Transport("NATS client is unavailable".to_string()))?;
        runtime
            .block_on(client.publish(self.subject.clone(), payload.into()))
            .map_err(transport_error)?;
        runtime.block_on(client.flush()).map_err(transport_error)?;
        Ok(DeliveryReceipt {
            message_id,
            provider: Provider::Nats,
            state: DeliveryState::Published,
            trace_id,
        })
    }

    fn receive(&self) -> Result<Option<ReceivedMessage>, DomainError> {
        // B-007: the subscription is restored by Drop, so an unwind between here and the
        // end of the borrow cannot leave the slot empty for the life of the client.
        let mut subscription =
            RestoreOnDrop::take_from(&self.subscription, "NATS subscription is unavailable")?;
        let runtime = self
            .runtime
            .as_ref()
            .ok_or_else(|| DomainError::Transport("NATS runtime is unavailable".to_string()))?;
        let message = runtime.block_on(async {
            subscription
                .value_mut()
                .ok_or_else(|| {
                    DomainError::Transport("NATS subscription is unavailable".to_string())
                })?
                .next()
                .await
                .ok_or_else(|| DomainError::Transport("NATS subscription ended".to_string()))
        });
        drop(subscription);
        let message = message?;
        let message: MessageEnvelope = serde_json::from_slice(&message.payload)
            .map_err(|error| DomainError::Serialization(error.to_string()))?;
        let receipt = DeliveryReceipt {
            message_id: message.message_id.clone(),
            provider: Provider::Nats,
            state: DeliveryState::Received,
            trace_id: message.tracing.trace_id.clone(),
        };
        Ok(Some(ReceivedMessage { message, receipt }))
    }

    fn acknowledge(&self, receipt: &DeliveryReceipt) -> Result<DeliveryReceipt, DomainError> {
        if receipt.provider != Provider::Nats {
            return Err(DomainError::Transport(
                "receipt provider does not match NATS".to_string(),
            ));
        }
        self.acknowledged
            .lock()
            .map_err(|_| {
                DomainError::Transport("NATS acknowledgement state is unavailable".to_string())
            })?
            .insert(receipt.message_id.clone());
        Ok(DeliveryReceipt {
            state: DeliveryState::Acknowledged,
            ..receipt.clone()
        })
    }
}

/// JetStream-backed NATS transport with server-side durable acknowledgement state.
#[cfg(feature = "nats")]
pub struct NatsJetStreamTransport {
    client: Option<async_nats::Client>,
    context: Option<async_nats::jetstream::Context>,
    stream: Mutex<Option<async_nats::jetstream::consumer::pull::Stream>>,
    pending_acknowledgements: Mutex<BTreeMap<String, async_nats::jetstream::message::Acker>>,
    subject: String,
    runtime: Option<tokio::runtime::Runtime>,
}

#[cfg(feature = "nats")]
impl NatsJetStreamTransport {
    pub fn connect(
        url: &str,
        subject: &str,
        stream_name: &str,
        consumer_name: &str,
    ) -> Result<Self, DomainError> {
        if subject.trim().is_empty()
            || stream_name.trim().is_empty()
            || consumer_name.trim().is_empty()
        {
            return Err(DomainError::InvalidEnvelope(
                "NATS JetStream subject, stream, and consumer names are required".to_string(),
            ));
        }
        let runtime = tokio::runtime::Runtime::new().map_err(transport_error)?;
        // H-02: bounded, same reason as the core NATS connect.
        let (client, context, consumer_stream) = block_on_with_timeout(
            &runtime,
            "NATS JetStream connect",
            broker_timeout(DEFAULT_CONNECT_TIMEOUT_SECS),
            async {
                let client = async_nats::connect(url).await.map_err(transport_error)?;
                let context = async_nats::jetstream::new(client.clone());
                let stream = context
                    .get_or_create_stream(async_nats::jetstream::stream::Config {
                        name: stream_name.to_string(),
                        subjects: vec![subject.to_string()],
                        max_messages: -1,
                        max_age: Duration::from_secs(0),
                        ..Default::default()
                    })
                    .await
                    .map_err(transport_error)?;
                let consumer = stream
                    .get_or_create_consumer(
                        consumer_name,
                        async_nats::jetstream::consumer::pull::Config {
                            durable_name: Some(consumer_name.to_string()),
                            ack_policy: async_nats::jetstream::consumer::AckPolicy::Explicit,
                            ..Default::default()
                        },
                    )
                    .await
                    .map_err(transport_error)?;
                let consumer_stream = consumer
                    .stream()
                    .max_messages_per_batch(1)
                    .messages()
                    .await
                    .map_err(transport_error)?;
                Ok::<_, DomainError>((client, context, consumer_stream))
            },
        )??;
        Ok(Self {
            client: Some(client),
            context: Some(context),
            stream: Mutex::new(Some(consumer_stream)),
            pending_acknowledgements: Mutex::new(BTreeMap::new()),
            subject: subject.to_string(),
            runtime: Some(runtime),
        })
    }
}

#[cfg(feature = "nats")]
impl Drop for NatsJetStreamTransport {
    fn drop(&mut self) {
        let stream = self.stream.lock().ok().and_then(|mut stream| stream.take());
        if let Some(runtime) = self.runtime.take() {
            runtime.block_on(async move {
                drop(stream);
            });
        }
        if let Ok(mut acknowledgements) = self.pending_acknowledgements.lock() {
            acknowledgements.clear();
        }
        self.context.take();
        self.client.take();
    }
}

#[cfg(feature = "nats")]
impl MessageTransport for NatsJetStreamTransport {
    fn provider(&self) -> Provider {
        Provider::NatsJetStream
    }

    fn publish(&self, message: MessageEnvelope) -> Result<DeliveryReceipt, DomainError> {
        let message_id = message.message_id.clone();
        let trace_id = message.tracing.trace_id.clone();
        let payload = serde_json::to_vec(&message)
            .map_err(|error| DomainError::Serialization(error.to_string()))?;
        let runtime = self.runtime.as_ref().ok_or_else(|| {
            DomainError::Transport("NATS JetStream runtime is unavailable".to_string())
        })?;
        let context = self.context.as_ref().ok_or_else(|| {
            DomainError::Transport("NATS JetStream context is unavailable".to_string())
        })?;
        runtime.block_on(async {
            let pending = context
                .publish(self.subject.clone(), payload.into())
                .await
                .map_err(transport_error)?;
            pending.await.map_err(transport_error)
        })?;
        Ok(DeliveryReceipt {
            message_id,
            provider: Provider::NatsJetStream,
            state: DeliveryState::Published,
            trace_id,
        })
    }

    fn receive(&self) -> Result<Option<ReceivedMessage>, DomainError> {
        // B-007: same restore-on-unwind contract as the core NATS receive.
        let mut stream =
            RestoreOnDrop::take_from(&self.stream, "NATS JetStream stream is unavailable")?;
        let runtime = self.runtime.as_ref().ok_or_else(|| {
            DomainError::Transport("NATS JetStream runtime is unavailable".to_string())
        })?;
        let result = runtime.block_on(async {
            stream
                .value_mut()
                .ok_or_else(|| {
                    DomainError::Transport("NATS JetStream stream is unavailable".to_string())
                })?
                .next()
                .await
                .ok_or_else(|| DomainError::Transport("NATS JetStream consumer ended".to_string()))?
                .map_err(transport_error)
        });
        drop(stream);
        let message = result?;
        let (message, acker) = message.split();
        let message: MessageEnvelope = serde_json::from_slice(&message.payload)
            .map_err(|error| DomainError::Serialization(error.to_string()))?;
        let receipt = DeliveryReceipt {
            message_id: message.message_id.clone(),
            provider: Provider::NatsJetStream,
            state: DeliveryState::Received,
            trace_id: message.tracing.trace_id.clone(),
        };
        self.pending_acknowledgements
            .lock()
            .map_err(|_| {
                DomainError::Transport(
                    "NATS JetStream acknowledgement store is unavailable".to_string(),
                )
            })?
            .insert(message.message_id.clone(), acker);
        Ok(Some(ReceivedMessage { message, receipt }))
    }

    fn acknowledge(&self, receipt: &DeliveryReceipt) -> Result<DeliveryReceipt, DomainError> {
        if receipt.provider != Provider::NatsJetStream {
            return Err(DomainError::Transport(
                "receipt provider does not match NATS JetStream".to_string(),
            ));
        }
        let acker = self
            .pending_acknowledgements
            .lock()
            .map_err(|_| {
                DomainError::Transport(
                    "NATS JetStream acknowledgement store is unavailable".to_string(),
                )
            })?
            .remove(&receipt.message_id)
            .ok_or_else(|| {
                DomainError::Transport(
                    "no pending JetStream acknowledgement for receipt".to_string(),
                )
            })?;
        let runtime = self.runtime.as_ref().ok_or_else(|| {
            DomainError::Transport("NATS JetStream runtime is unavailable".to_string())
        })?;
        runtime.block_on(acker.ack()).map_err(transport_error)?;
        Ok(DeliveryReceipt {
            state: DeliveryState::Acknowledged,
            ..receipt.clone()
        })
    }
}

// Both variants are boxed deliberately. NatsJetStreamTransport is ~792 bytes and
// NatsTransport ~312, and this enum is nested inside MessageTransportKind, so the
// larger of the two set the size of EVERY transport variant. Boxing both keeps this
// enum one pointer wide and stops NATS from inflating Kafka, RabbitMQ and Pulsar.
#[cfg(feature = "nats")]
pub enum NatsTransportKind {
    Core(Box<NatsTransport>),
    JetStream(Box<NatsJetStreamTransport>),
}

#[cfg(feature = "kafka")]
pub struct KafkaTransport {
    producer: Option<FutureProducer>,
    consumer: Option<StreamConsumer>,
    runtime: Option<tokio::runtime::Runtime>,
    topic: String,
    pending_acknowledgements: Mutex<BTreeMap<String, TopicPartitionList>>,
}

#[cfg(feature = "rabbitmq")]
pub struct RabbitMqTransport {
    connection: Option<Connection>,
    channel: Option<lapin::Channel>,
    runtime: Option<tokio::runtime::Runtime>,
    queue: String,
    pending_acknowledgements: Mutex<BTreeMap<String, Acker>>,
}

#[cfg(feature = "pulsar")]
pub struct PulsarTransport {
    client: Pulsar<TokioExecutor>,
    consumer: Arc<Mutex<PulsarConsumer<Vec<u8>, TokioExecutor>>>,
    runtime: Arc<tokio::runtime::Runtime>,
    topic: String,
    pending_acknowledgements: Mutex<BTreeMap<String, PulsarMessage<Vec<u8>>>>,
}

#[cfg(feature = "pulsar")]
impl PulsarTransport {
    pub fn connect(
        service_url: &str,
        topic: &str,
        subscription: &str,
    ) -> Result<Self, DomainError> {
        if service_url.trim().is_empty()
            || topic.trim().is_empty()
            || subscription.trim().is_empty()
        {
            return Err(DomainError::InvalidEnvelope(
                "Pulsar service URL, topic, and subscription are required".to_string(),
            ));
        }
        let service_url = service_url.to_string();
        let topic = topic.to_string();
        let subscription = subscription.to_string();
        let transport_topic = topic.clone();
        let (client, consumer, runtime) = thread::Builder::new()
            .name("modernlink-pulsar-connect".to_string())
            .stack_size(8 * 1024 * 1024)
            .spawn(move || {
                let runtime = tokio::runtime::Runtime::new().map_err(transport_error)?;
                // H-02: bounded. This runs on a dedicated worker thread, so an unbounded
                // hang here leaks a thread per attempt as well as never returning - the
                // join below would block the caller forever.
                let client = block_on_with_timeout(
                    &runtime,
                    "Pulsar connect",
                    broker_timeout(DEFAULT_CONNECT_TIMEOUT_SECS),
                    Pulsar::builder(service_url, TokioExecutor).build(),
                )?
                .map_err(transport_error)?;
                let consumer = block_on_with_timeout(
                    &runtime,
                    "Pulsar consumer build",
                    broker_timeout(DEFAULT_ADMIN_TIMEOUT_SECS),
                    client
                        .consumer()
                        .with_topic(topic)
                        .with_subscription(subscription)
                        .build::<Vec<u8>>(),
                )?
                .map_err(transport_error)?;
                Ok((client, consumer, runtime))
            })
            .map_err(transport_error)?
            .join()
            .map_err(|_| {
                DomainError::Transport("Pulsar connection thread panicked".to_string())
            })??;
        Ok(Self {
            client,
            consumer: Arc::new(Mutex::new(consumer)),
            runtime: Arc::new(runtime),
            topic: transport_topic,
            pending_acknowledgements: Mutex::new(BTreeMap::new()),
        })
    }

    fn run_on_worker<F, T>(&self, operation: F) -> Result<T, DomainError>
    where
        F: FnOnce(Arc<tokio::runtime::Runtime>) -> Result<T, DomainError> + Send + 'static,
        T: Send + 'static,
    {
        let runtime = self.runtime.clone();
        thread::Builder::new()
            .name("modernlink-pulsar-operation".to_string())
            .stack_size(8 * 1024 * 1024)
            .spawn(move || operation(runtime))
            .map_err(transport_error)?
            .join()
            .map_err(|_| DomainError::Transport("Pulsar operation thread panicked".to_string()))?
    }
}

#[cfg(feature = "pulsar")]
impl MessageTransport for PulsarTransport {
    fn provider(&self) -> Provider {
        Provider::Pulsar
    }

    fn publish(&self, message: MessageEnvelope) -> Result<DeliveryReceipt, DomainError> {
        let message_id = message.message_id.clone();
        let trace_id = message.tracing.trace_id.clone();
        let payload = serde_json::to_vec(&message)
            .map_err(|error| DomainError::Serialization(error.to_string()))?;
        let client = self.client.clone();
        let topic = self.topic.clone();
        self.run_on_worker(move |runtime| {
            runtime.block_on(async {
                let send = client
                    .send(
                        topic,
                        PulsarProducerMessage {
                            payload,
                            ..Default::default()
                        },
                    )
                    .await
                    .map_err(transport_error)?;
                send.await.map_err(transport_error)
            })
        })?;
        Ok(DeliveryReceipt {
            message_id,
            provider: Provider::Pulsar,
            state: DeliveryState::Published,
            trace_id,
        })
    }

    fn receive(&self) -> Result<Option<ReceivedMessage>, DomainError> {
        let consumer = self.consumer.clone();
        let delivery = self.run_on_worker(move |runtime| {
            // Take the guard on the worker thread BEFORE entering the async block.
            // Holding a std::sync MutexGuard across an .await is a deadlock risk the
            // moment anything else on this runtime wants the consumer; scoping it to
            // the synchronous worker frame keeps the lock and the blocking wait in
            // the same, obviously-serialized place.
            let mut consumer = consumer.lock().map_err(|_| {
                DomainError::Transport("Pulsar consumer is unavailable".to_string())
            })?;
            runtime.block_on(async { consumer.next().await.transpose().map_err(transport_error) })
        })?;
        let Some(delivery) = delivery else {
            return Ok(None);
        };
        let envelope: MessageEnvelope = serde_json::from_slice(&delivery.deserialize())
            .map_err(|error| DomainError::Serialization(error.to_string()))?;
        let receipt = DeliveryReceipt {
            message_id: envelope.message_id.clone(),
            provider: Provider::Pulsar,
            state: DeliveryState::Received,
            trace_id: envelope.tracing.trace_id.clone(),
        };
        self.pending_acknowledgements
            .lock()
            .map_err(|_| {
                DomainError::Transport("Pulsar acknowledgement store is unavailable".to_string())
            })?
            .insert(envelope.message_id.clone(), delivery);
        Ok(Some(ReceivedMessage {
            message: envelope,
            receipt,
        }))
    }

    fn acknowledge(&self, receipt: &DeliveryReceipt) -> Result<DeliveryReceipt, DomainError> {
        if receipt.provider != Provider::Pulsar {
            return Err(DomainError::Transport(
                "receipt provider does not match Pulsar".to_string(),
            ));
        }
        let delivery = self
            .pending_acknowledgements
            .lock()
            .map_err(|_| {
                DomainError::Transport("Pulsar acknowledgement store is unavailable".to_string())
            })?
            .remove(&receipt.message_id)
            .ok_or_else(|| {
                DomainError::Transport("no pending Pulsar acknowledgement for receipt".to_string())
            })?;
        let consumer = self.consumer.clone();
        self.run_on_worker(move |runtime| {
            // Same reason as `receive`: acquire the guard synchronously on the worker
            // thread rather than holding it across the .await inside block_on.
            let mut consumer = consumer.lock().map_err(|_| {
                DomainError::Transport("Pulsar consumer is unavailable".to_string())
            })?;
            runtime.block_on(async { consumer.ack(&delivery).await.map_err(transport_error) })
        })?;
        Ok(DeliveryReceipt {
            state: DeliveryState::Acknowledged,
            ..receipt.clone()
        })
    }
}

#[cfg(feature = "rabbitmq")]
impl RabbitMqTransport {
    pub fn connect(uri: &str, queue: &str) -> Result<Self, DomainError> {
        if uri.trim().is_empty() || queue.trim().is_empty() {
            return Err(DomainError::InvalidEnvelope(
                "RabbitMQ URI and queue are required".to_string(),
            ));
        }
        let runtime = tokio::runtime::Runtime::new().map_err(transport_error)?;
        let _runtime_guard = runtime.enter();
        // H-02: each leg is bounded separately. A broker can complete the TCP handshake
        // and then stall on the AMQP handshake, so bounding only the first would leave the
        // hang intact one step later.
        let connection = block_on_with_timeout(
            &runtime,
            "RabbitMQ connect",
            broker_timeout(DEFAULT_CONNECT_TIMEOUT_SECS),
            Connection::connect(uri, ConnectionProperties::default()),
        )?
        .map_err(transport_error)?;
        let channel = block_on_with_timeout(
            &runtime,
            "RabbitMQ channel open",
            broker_timeout(DEFAULT_CONNECT_TIMEOUT_SECS),
            connection.create_channel(),
        )?
        .map_err(transport_error)?;
        block_on_with_timeout(
            &runtime,
            "RabbitMQ queue declare",
            broker_timeout(DEFAULT_ADMIN_TIMEOUT_SECS),
            channel.queue_declare(
                queue,
                QueueDeclareOptions {
                    durable: true,
                    ..QueueDeclareOptions::default()
                },
                FieldTable::default(),
            ),
        )?
        .map_err(transport_error)?;
        Ok(Self {
            connection: Some(connection),
            channel: Some(channel),
            runtime: Some(runtime),
            queue: queue.to_string(),
            pending_acknowledgements: Mutex::new(BTreeMap::new()),
        })
    }
}

#[cfg(feature = "rabbitmq")]
impl Drop for RabbitMqTransport {
    fn drop(&mut self) {
        let channel = self.channel.take();
        let connection = self.connection.take();
        if let Some(runtime) = self.runtime.take() {
            runtime.block_on(async move {
                drop(channel);
                drop(connection);
            });
        }
    }
}

#[cfg(feature = "rabbitmq")]
impl MessageTransport for RabbitMqTransport {
    fn provider(&self) -> Provider {
        Provider::RabbitMq
    }

    fn publish(&self, message: MessageEnvelope) -> Result<DeliveryReceipt, DomainError> {
        let message_id = message.message_id.clone();
        let trace_id = message.tracing.trace_id.clone();
        let payload = serde_json::to_vec(&message)
            .map_err(|error| DomainError::Serialization(error.to_string()))?;
        let runtime = self
            .runtime
            .as_ref()
            .ok_or_else(|| DomainError::Transport("RabbitMQ runtime is unavailable".to_string()))?;
        let channel = self
            .channel
            .as_ref()
            .ok_or_else(|| DomainError::Transport("RabbitMQ channel is unavailable".to_string()))?;
        let confirmation = runtime
            .block_on(async {
                let confirm = channel
                    .basic_publish(
                        "",
                        &self.queue,
                        BasicPublishOptions::default(),
                        &payload,
                        BasicProperties::default(),
                    )
                    .await?;
                confirm.await
            })
            .map_err(transport_error)?;
        if !confirmation.is_ack()
            && !matches!(
                confirmation,
                lapin::publisher_confirm::Confirmation::NotRequested
            )
        {
            return Err(DomainError::Transport(
                "RabbitMQ publish was not acknowledged".to_string(),
            ));
        }
        Ok(DeliveryReceipt {
            message_id,
            provider: Provider::RabbitMq,
            state: DeliveryState::Published,
            trace_id,
        })
    }

    fn receive(&self) -> Result<Option<ReceivedMessage>, DomainError> {
        let runtime = self
            .runtime
            .as_ref()
            .ok_or_else(|| DomainError::Transport("RabbitMQ runtime is unavailable".to_string()))?;
        let channel = self
            .channel
            .as_ref()
            .ok_or_else(|| DomainError::Transport("RabbitMQ channel is unavailable".to_string()))?;
        let result = runtime
            .block_on(channel.basic_get(&self.queue, BasicGetOptions::default()))
            .map_err(transport_error)?;
        let Some(delivery) = result else {
            return Ok(None);
        };
        let envelope: MessageEnvelope = serde_json::from_slice(&delivery.data)
            .map_err(|error| DomainError::Serialization(error.to_string()))?;
        let receipt = DeliveryReceipt {
            message_id: envelope.message_id.clone(),
            provider: Provider::RabbitMq,
            state: DeliveryState::Received,
            trace_id: envelope.tracing.trace_id.clone(),
        };
        self.pending_acknowledgements
            .lock()
            .map_err(|_| {
                DomainError::Transport("RabbitMQ acknowledgement store is unavailable".to_string())
            })?
            .insert(envelope.message_id.clone(), delivery.delivery.acker.clone());
        Ok(Some(ReceivedMessage {
            message: envelope,
            receipt,
        }))
    }

    fn acknowledge(&self, receipt: &DeliveryReceipt) -> Result<DeliveryReceipt, DomainError> {
        if receipt.provider != Provider::RabbitMq {
            return Err(DomainError::Transport(
                "receipt provider does not match RabbitMQ".to_string(),
            ));
        }
        let acker = self
            .pending_acknowledgements
            .lock()
            .map_err(|_| {
                DomainError::Transport("RabbitMQ acknowledgement store is unavailable".to_string())
            })?
            .remove(&receipt.message_id)
            .ok_or_else(|| {
                DomainError::Transport(
                    "no pending RabbitMQ acknowledgement for receipt".to_string(),
                )
            })?;
        let runtime = self
            .runtime
            .as_ref()
            .ok_or_else(|| DomainError::Transport("RabbitMQ runtime is unavailable".to_string()))?;
        runtime
            .block_on(acker.ack(BasicAckOptions::default()))
            .map_err(transport_error)?;
        Ok(DeliveryReceipt {
            state: DeliveryState::Acknowledged,
            ..receipt.clone()
        })
    }
}

#[cfg(feature = "kafka")]
impl KafkaTransport {
    pub fn connect(brokers: &str, topic: &str, group_id: &str) -> Result<Self, DomainError> {
        if brokers.trim().is_empty() || topic.trim().is_empty() || group_id.trim().is_empty() {
            return Err(DomainError::InvalidEnvelope(
                "Kafka brokers, topic, and group ID are required".to_string(),
            ));
        }
        let runtime = tokio::runtime::Runtime::new().map_err(transport_error)?;
        let _runtime_guard = runtime.enter();
        // H-07: this build has no Kafka TLS. rdkafka is compiled with `cmake-build` and
        // `libz` only - no `ssl` feature - so librdkafka is built without OpenSSL, and this
        // transport sets no `security.protocol`. A TLS-looking endpoint therefore cannot be
        // honoured, and connecting in plaintext instead would leave the deployment believing
        // its traffic and credentials are encrypted. Refuse instead.
        if endpoint_requests_tls(brokers) {
            return Err(DomainError::Unsupported(
                concat!(
                    "Kafka TLS is not available in this build: rdkafka is compiled without ",
                    "the `ssl` feature and this transport exposes no `security.protocol` ",
                    "setting. The connection was refused rather than made in plaintext."
                )
                .to_string(),
            ));
        }
        let admin = ClientConfig::new()
            .set("bootstrap.servers", brokers)
            .create::<AdminClient<DefaultClientContext>>()
            .map_err(transport_error)?;
        let new_topic = NewTopic::new(topic, 1, TopicReplication::Fixed(1));
        // H-02: bounded. Topic creation talks to the cluster controller, which can accept
        // the connection and then stall exactly like a broker connect.
        let topic_results = block_on_with_timeout(
            &runtime,
            "Kafka topic creation",
            broker_timeout(DEFAULT_ADMIN_TIMEOUT_SECS),
            admin.create_topics(&[new_topic], &AdminOptions::new()),
        )?
        .map_err(transport_error)?;
        for result in topic_results {
            if let Err((_, code)) = result {
                if code != RDKafkaErrorCode::TopicAlreadyExists {
                    return Err(DomainError::Transport(format!(
                        "Kafka topic creation failed: {:?}",
                        code
                    )));
                }
            }
        }
        let producer = ClientConfig::new()
            .set("bootstrap.servers", brokers)
            .set("message.timeout.ms", "10000")
            .create::<FutureProducer>()
            .map_err(transport_error)?;
        let consumer = ClientConfig::new()
            .set("bootstrap.servers", brokers)
            .set("group.id", group_id)
            .set("enable.auto.commit", "false")
            .set("auto.offset.reset", "earliest")
            .create::<StreamConsumer>()
            .map_err(transport_error)?;
        consumer.subscribe(&[topic]).map_err(transport_error)?;
        Ok(Self {
            producer: Some(producer),
            consumer: Some(consumer),
            runtime: Some(runtime),
            topic: topic.to_string(),
            pending_acknowledgements: Mutex::new(BTreeMap::new()),
        })
    }
}

#[cfg(feature = "kafka")]
impl Drop for KafkaTransport {
    fn drop(&mut self) {
        let producer = self.producer.take();
        let consumer = self.consumer.take();
        if let Some(runtime) = self.runtime.take() {
            runtime.block_on(async move {
                drop(producer);
                drop(consumer);
            });
        }
    }
}

#[cfg(feature = "kafka")]
impl MessageTransport for KafkaTransport {
    fn provider(&self) -> Provider {
        Provider::Kafka
    }

    fn publish(&self, message: MessageEnvelope) -> Result<DeliveryReceipt, DomainError> {
        let message_id = message.message_id.clone();
        let trace_id = message.tracing.trace_id.clone();
        let payload = serde_json::to_vec(&message)
            .map_err(|error| DomainError::Serialization(error.to_string()))?;
        let runtime = self
            .runtime
            .as_ref()
            .ok_or_else(|| DomainError::Transport("Kafka runtime is unavailable".to_string()))?;
        let producer = self
            .producer
            .as_ref()
            .ok_or_else(|| DomainError::Transport("Kafka producer is unavailable".to_string()))?;
        runtime
            .block_on(
                producer.send(
                    FutureRecord::to(&self.topic)
                        .key(&message_id)
                        .payload(&payload),
                    std::time::Duration::from_secs(10),
                ),
            )
            .map_err(|(error, _)| transport_error(error))
            .map(|_| DeliveryReceipt {
                message_id,
                provider: Provider::Kafka,
                state: DeliveryState::Published,
                trace_id,
            })
    }

    fn receive(&self) -> Result<Option<ReceivedMessage>, DomainError> {
        let runtime = self
            .runtime
            .as_ref()
            .ok_or_else(|| DomainError::Transport("Kafka runtime is unavailable".to_string()))?;
        let consumer = self
            .consumer
            .as_ref()
            .ok_or_else(|| DomainError::Transport("Kafka consumer is unavailable".to_string()))?;
        let message = runtime.block_on(consumer.recv()).map_err(transport_error)?;
        let payload = message.payload().ok_or_else(|| {
            DomainError::Serialization("Kafka message has no payload".to_string())
        })?;
        let envelope: MessageEnvelope = serde_json::from_slice(payload)
            .map_err(|error| DomainError::Serialization(error.to_string()))?;
        let mut offsets = TopicPartitionList::new();
        offsets
            .add_partition_offset(
                message.topic(),
                message.partition(),
                Offset::Offset(message.offset() + 1),
            )
            .map_err(transport_error)?;
        let receipt = DeliveryReceipt {
            message_id: envelope.message_id.clone(),
            provider: Provider::Kafka,
            state: DeliveryState::Received,
            trace_id: envelope.tracing.trace_id.clone(),
        };
        self.pending_acknowledgements
            .lock()
            .map_err(|_| {
                DomainError::Transport("Kafka acknowledgement store is unavailable".to_string())
            })?
            .insert(envelope.message_id.clone(), offsets);
        Ok(Some(ReceivedMessage {
            message: envelope,
            receipt,
        }))
    }

    fn acknowledge(&self, receipt: &DeliveryReceipt) -> Result<DeliveryReceipt, DomainError> {
        if receipt.provider != Provider::Kafka {
            return Err(DomainError::Transport(
                "receipt provider does not match Kafka".to_string(),
            ));
        }
        let offsets = self
            .pending_acknowledgements
            .lock()
            .map_err(|_| {
                DomainError::Transport("Kafka acknowledgement store is unavailable".to_string())
            })?
            .remove(&receipt.message_id)
            .ok_or_else(|| {
                DomainError::Transport("no pending Kafka acknowledgement for receipt".to_string())
            })?;
        self.consumer
            .as_ref()
            .ok_or_else(|| DomainError::Transport("Kafka consumer is unavailable".to_string()))?
            .commit(&offsets, CommitMode::Sync)
            .map_err(transport_error)?;
        Ok(DeliveryReceipt {
            state: DeliveryState::Acknowledged,
            ..receipt.clone()
        })
    }
}

// RabbitMqTransport is boxed for the same reason NATS is: at ~432 bytes it was more
// than double the next variant, so every transport paid for it. See NatsTransportKind.
pub enum MessageTransportKind {
    LegacyJms(InMemoryTransport),
    #[cfg(feature = "nats")]
    Nats(NatsTransportKind),
    #[cfg(feature = "kafka")]
    Kafka(KafkaTransport),
    #[cfg(feature = "rabbitmq")]
    RabbitMq(Box<RabbitMqTransport>),
    #[cfg(feature = "pulsar")]
    Pulsar(PulsarTransport),
}

impl MessageTransport for MessageTransportKind {
    fn provider(&self) -> Provider {
        match self {
            Self::LegacyJms(transport) => transport.provider(),
            #[cfg(feature = "nats")]
            Self::Nats(transport) => transport.provider(),
            #[cfg(feature = "kafka")]
            Self::Kafka(transport) => transport.provider(),
            #[cfg(feature = "rabbitmq")]
            Self::RabbitMq(transport) => transport.provider(),
            #[cfg(feature = "pulsar")]
            Self::Pulsar(transport) => transport.provider(),
        }
    }

    fn publish(&self, message: MessageEnvelope) -> Result<DeliveryReceipt, DomainError> {
        match self {
            Self::LegacyJms(transport) => transport.publish(message),
            #[cfg(feature = "nats")]
            Self::Nats(transport) => transport.publish(message),
            #[cfg(feature = "kafka")]
            Self::Kafka(transport) => transport.publish(message),
            #[cfg(feature = "rabbitmq")]
            Self::RabbitMq(transport) => transport.publish(message),
            #[cfg(feature = "pulsar")]
            Self::Pulsar(transport) => transport.publish(message),
        }
    }

    fn receive(&self) -> Result<Option<ReceivedMessage>, DomainError> {
        match self {
            Self::LegacyJms(transport) => transport.receive(),
            #[cfg(feature = "nats")]
            Self::Nats(transport) => transport.receive(),
            #[cfg(feature = "kafka")]
            Self::Kafka(transport) => transport.receive(),
            #[cfg(feature = "rabbitmq")]
            Self::RabbitMq(transport) => transport.receive(),
            #[cfg(feature = "pulsar")]
            Self::Pulsar(transport) => transport.receive(),
        }
    }

    fn acknowledge(&self, receipt: &DeliveryReceipt) -> Result<DeliveryReceipt, DomainError> {
        match self {
            Self::LegacyJms(transport) => transport.acknowledge(receipt),
            #[cfg(feature = "nats")]
            Self::Nats(transport) => transport.acknowledge(receipt),
            #[cfg(feature = "kafka")]
            Self::Kafka(transport) => transport.acknowledge(receipt),
            #[cfg(feature = "rabbitmq")]
            Self::RabbitMq(transport) => transport.acknowledge(receipt),
            #[cfg(feature = "pulsar")]
            Self::Pulsar(transport) => transport.acknowledge(receipt),
        }
    }
}

#[cfg(feature = "nats")]
impl MessageTransport for NatsTransportKind {
    fn provider(&self) -> Provider {
        match self {
            Self::Core(transport) => transport.provider(),
            Self::JetStream(transport) => transport.provider(),
        }
    }

    fn publish(&self, message: MessageEnvelope) -> Result<DeliveryReceipt, DomainError> {
        match self {
            Self::Core(transport) => transport.publish(message),
            Self::JetStream(transport) => transport.publish(message),
        }
    }

    fn receive(&self) -> Result<Option<ReceivedMessage>, DomainError> {
        match self {
            Self::Core(transport) => transport.receive(),
            Self::JetStream(transport) => transport.receive(),
        }
    }

    fn acknowledge(&self, receipt: &DeliveryReceipt) -> Result<DeliveryReceipt, DomainError> {
        match self {
            Self::Core(transport) => transport.acknowledge(receipt),
            Self::JetStream(transport) => transport.acknowledge(receipt),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteRule {
    pub id: String,
    pub destination: Option<String>,
    pub destination_prefix: Option<String>,
    pub tenant: Option<String>,
    pub header_name: Option<String>,
    pub header_value: Option<String>,
    pub mode: Mode,
    pub provider: Provider,
    pub allowed: bool,
}

impl RouteRule {
    fn matches(&self, message: &MessageEnvelope) -> bool {
        if let Some(destination) = &self.destination {
            if &message.destination != destination {
                return false;
            }
        }
        if let Some(prefix) = &self.destination_prefix {
            if !message.destination.starts_with(prefix) {
                return false;
            }
        }
        if let Some(tenant) = &self.tenant {
            if message.tenant.as_ref() != Some(tenant) {
                return false;
            }
        }
        if let Some(name) = &self.header_name {
            if message.headers.get(name) != self.header_value.as_ref() {
                return false;
            }
        }
        true
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteConfig {
    pub default_mode: Mode,
    pub default_provider: Provider,
    pub rules: Vec<RouteRule>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteDecision {
    pub mode: Mode,
    pub provider: Provider,
    pub rule_id: Option<String>,
    pub allowed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DispatchResult {
    pub decision: RouteDecision,
    pub receipt: DeliveryReceipt,
}

impl RouteConfig {
    /// Evaluate a route without publishing or requiring a provider transport.
    ///
    /// A denied rule is returned as a decision so callers can explain a
    /// dry-run result; `dispatch` continues to reject denied routes.
    pub fn dry_run(&self, message: &MessageEnvelope) -> Result<RouteDecision, DomainError> {
        self.decide(message)
    }

    pub fn decide(&self, message: &MessageEnvelope) -> Result<RouteDecision, DomainError> {
        let (mode, provider, rule_id, allowed) = self
            .rules
            .iter()
            .find(|rule| rule.matches(message))
            .map(|rule| {
                (
                    rule.mode,
                    rule.provider,
                    Some(rule.id.clone()),
                    rule.allowed,
                )
            })
            .unwrap_or((self.default_mode, self.default_provider, None, true));
        if mode == Mode::Transparent && provider != Provider::LegacyJms {
            return Err(DomainError::InvalidRoute(
                "transparent mode requires the LegacyJms provider".to_string(),
            ));
        }
        if mode != Mode::Transparent && provider == Provider::LegacyJms {
            return Err(DomainError::InvalidRoute(
                "transform and redirect modes require a modern provider".to_string(),
            ));
        }
        Ok(RouteDecision {
            mode,
            provider,
            rule_id,
            allowed,
        })
    }

    pub fn dispatch<T: MessageTransport>(
        &self,
        mut message: MessageEnvelope,
        transport: &T,
    ) -> Result<DispatchResult, DomainError> {
        let decision = self.decide(&message)?;
        if !decision.allowed {
            return Err(DomainError::InvalidRoute(
                "message route is denied by policy".to_string(),
            ));
        }
        if transport.provider() != decision.provider {
            return Err(DomainError::Transport(
                "transport provider does not match route decision".to_string(),
            ));
        }
        if decision.mode != Mode::Transparent {
            message.properties.insert(
                "modernlink.mode".to_string(),
                format!("{:?}", decision.mode),
            );
            message.properties.insert(
                "modernlink.provider".to_string(),
                format!("{:?}", decision.provider),
            );
        }
        let receipt = transport.publish(message)?;
        Ok(DispatchResult { decision, receipt })
    }
}

#[cfg(test)]
mod tests;
