use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;
use std::sync::{Arc, Mutex};

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
    RabbitMq,
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
}

impl fmt::Display for DomainError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidEnvelope(message)
            | Self::InvalidRoute(message)
            | Self::Serialization(message)
            | Self::Transport(message) => formatter.write_str(message),
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
mod tests {
    use super::{
        AcknowledgementMode, DeliveryMode, DeliveryState, InMemoryTransport, MessageEnvelope,
        MessageTransport, Mode, Payload, Provider, RouteConfig, RouteRule,
    };

    fn message() -> MessageEnvelope {
        let mut message = MessageEnvelope::new(
            "orders.created",
            Payload::Text("body".to_string()),
            1_700_000_000_000,
        )
        .unwrap();
        message.tenant = Some("acme".to_string());
        message
            .headers
            .insert("region".to_string(), "us".to_string());
        message
    }

    #[test]
    fn creates_versioned_uuidv7_envelope() {
        let message = message();
        assert_eq!(message.version, 1);
        assert_eq!(message.message_id.len(), 36);
        assert_eq!(&message.message_id[14..15], "7");
        assert_eq!(message.delivery_mode, DeliveryMode::Persistent);
        assert_eq!(message.acknowledgement_mode, AcknowledgementMode::Auto);
        assert_eq!(message.tracing.trace_id.len(), 32);
        assert_eq!(message.tracing.span_id.len(), 16);
    }

    #[test]
    fn first_matching_rule_wins_and_preserves_rule_id() {
        let config = RouteConfig {
            default_mode: Mode::Transparent,
            default_provider: Provider::LegacyJms,
            rules: vec![RouteRule {
                id: "orders-to-kafka".to_string(),
                destination: Some("orders.created".to_string()),
                destination_prefix: None,
                tenant: Some("acme".to_string()),
                header_name: Some("region".to_string()),
                header_value: Some("us".to_string()),
                mode: Mode::Transform,
                provider: Provider::Kafka,
                allowed: true,
            }],
        };
        let decision = config.decide(&message()).unwrap();
        assert_eq!(decision.rule_id.as_deref(), Some("orders-to-kafka"));
        assert_eq!(decision.mode, Mode::Transform);
        assert_eq!(decision.provider, Provider::Kafka);
    }

    #[test]
    fn rejects_invalid_transparent_provider_pair() {
        let config = RouteConfig {
            default_mode: Mode::Transparent,
            default_provider: Provider::Kafka,
            rules: Vec::new(),
        };
        assert!(config.decide(&message()).is_err());
    }

    #[test]
    fn serializes_envelope_without_java_runtime_types() {
        let json = message().to_json().unwrap();
        assert!(json.contains("orders.created"));
        assert!(json.contains("message_id"));
    }

    #[test]
    fn uniform_transport_preserves_message_for_modern_provider_identity() {
        let transport = InMemoryTransport::new(Provider::Kafka);
        let message = message();
        let message_id = message.message_id.clone();
        let published = transport.publish(message).unwrap();
        assert_eq!(transport.provider(), Provider::Kafka);
        assert_eq!(published.state, DeliveryState::Published);
        let received = transport.receive().unwrap().unwrap();
        assert_eq!(received.message.message_id, message_id);
        let acknowledged = transport.acknowledge(&received.receipt).unwrap();
        assert_eq!(acknowledged.state, DeliveryState::Acknowledged);
    }

    #[test]
    fn dispatch_applies_transform_metadata_and_returns_receipt() {
        let config = RouteConfig {
            default_mode: Mode::Transform,
            default_provider: Provider::Kafka,
            rules: Vec::new(),
        };
        let transport = InMemoryTransport::new(Provider::Kafka);
        let result = config.dispatch(message(), &transport).unwrap();
        assert_eq!(result.decision.mode, Mode::Transform);
        assert_eq!(result.receipt.state, DeliveryState::Published);
        let received = transport.receive().unwrap().unwrap();
        assert_eq!(
            received.message.properties.get("modernlink.mode"),
            Some(&"Transform".to_string())
        );
    }

    #[test]
    fn dispatch_rejects_provider_mismatch_and_denied_routes() {
        let mismatch = RouteConfig {
            default_mode: Mode::Redirect,
            default_provider: Provider::Nats,
            rules: Vec::new(),
        };
        assert!(mismatch
            .dispatch(message(), &InMemoryTransport::new(Provider::Kafka))
            .is_err());

        let denied = RouteConfig {
            default_mode: Mode::Redirect,
            default_provider: Provider::Nats,
            rules: vec![RouteRule {
                id: "deny-orders".to_string(),
                destination: Some("orders.created".to_string()),
                destination_prefix: None,
                tenant: None,
                header_name: None,
                header_value: None,
                mode: Mode::Redirect,
                provider: Provider::Nats,
                allowed: false,
            }],
        };
        assert!(denied
            .dispatch(message(), &InMemoryTransport::new(Provider::Nats))
            .is_err());
    }
}
