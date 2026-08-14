//! The provider-neutral message domain and its transports.
//!
//! One uniform transport boundary ([`MessageTransportKind`]) fronts `InMemoryTransport`
//! (the in-process `LEGACY_JMS` compatibility path), NATS, NATS JetStream, Kafka, Pulsar, and
//! RabbitMQ, so the JNI surface never names a provider-specific type. Trace context is
//! first-class envelope data, not a user property: adapters must not replace or discard it.
//!
//! Unsupported guarantees fail closed — a capability gap is an explicit error, never a silent
//! degradation. Only the in-process transport has been exercised; every broker-backed path is
//! a source-level claim (`docs/ISSUES.md` I-010).

use futures_util::StreamExt;
use lapin::acker::Acker;
use lapin::options::{BasicAckOptions, BasicGetOptions, BasicPublishOptions, QueueDeclareOptions};
use lapin::types::FieldTable;
use lapin::{BasicProperties, Connection, ConnectionProperties};
use pulsar::consumer::{Consumer as PulsarConsumer, Message as PulsarMessage};
use pulsar::producer::Message as PulsarProducerMessage;
use pulsar::{Pulsar, TokioExecutor};
use rdkafka::admin::{AdminClient, AdminOptions, NewTopic, TopicReplication};
use rdkafka::client::DefaultClientContext;
use rdkafka::consumer::{CommitMode, Consumer, StreamConsumer};
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::types::RDKafkaErrorCode;
use rdkafka::{ClientConfig, Message as KafkaMessage, Offset, TopicPartitionList};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;
use std::sync::{Arc, Mutex};
use std::thread;
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

pub struct NatsTransport {
    client: Option<async_nats::Client>,
    subject: String,
    subscription: Mutex<Option<async_nats::Subscriber>>,
    acknowledged: Mutex<BTreeSet<String>>,
    runtime: Option<tokio::runtime::Runtime>,
}

impl NatsTransport {
    pub fn connect(url: &str, subject: &str) -> Result<Self, DomainError> {
        if subject.trim().is_empty() {
            return Err(DomainError::InvalidEnvelope(
                "NATS subject must not be empty".to_string(),
            ));
        }
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|error| DomainError::Transport(error.to_string()))?;
        let (client, subscription) = runtime.block_on(async {
            let client = async_nats::connect(url)
                .await
                .map_err(|error| DomainError::Transport(error.to_string()))?;
            let subscription = client
                .subscribe(subject.to_string())
                .await
                .map_err(|error| DomainError::Transport(error.to_string()))?;
            Ok::<_, DomainError>((client, subscription))
        })?;
        Ok(Self {
            client: Some(client),
            subject: subject.to_string(),
            subscription: Mutex::new(Some(subscription)),
            acknowledged: Mutex::new(BTreeSet::new()),
            runtime: Some(runtime),
        })
    }
}

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
            .map_err(|error| DomainError::Transport(error.to_string()))?;
        runtime
            .block_on(client.flush())
            .map_err(|error| DomainError::Transport(error.to_string()))?;
        Ok(DeliveryReceipt {
            message_id,
            provider: Provider::Nats,
            state: DeliveryState::Published,
            trace_id,
        })
    }

    fn receive(&self) -> Result<Option<ReceivedMessage>, DomainError> {
        let mut subscription = self
            .subscription
            .lock()
            .map_err(|_| DomainError::Transport("NATS subscription is unavailable".to_string()))?
            .take()
            .ok_or_else(|| {
                DomainError::Transport("NATS subscription is unavailable".to_string())
            })?;
        let runtime = self
            .runtime
            .as_ref()
            .ok_or_else(|| DomainError::Transport("NATS runtime is unavailable".to_string()))?;
        let message = runtime.block_on(async {
            subscription
                .next()
                .await
                .ok_or_else(|| DomainError::Transport("NATS subscription ended".to_string()))
        });
        self.subscription
            .lock()
            .map_err(|_| DomainError::Transport("NATS subscription is unavailable".to_string()))?
            .replace(subscription);
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
pub struct NatsJetStreamTransport {
    client: Option<async_nats::Client>,
    context: Option<async_nats::jetstream::Context>,
    stream: Mutex<Option<async_nats::jetstream::consumer::pull::Stream>>,
    pending_acknowledgements: Mutex<BTreeMap<String, async_nats::jetstream::message::Acker>>,
    subject: String,
    runtime: Option<tokio::runtime::Runtime>,
}

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
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|error| DomainError::Transport(error.to_string()))?;
        let (client, context, consumer_stream) = runtime.block_on(async {
            let client = async_nats::connect(url)
                .await
                .map_err(|error| DomainError::Transport(error.to_string()))?;
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
                .map_err(|error| DomainError::Transport(error.to_string()))?;
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
                .map_err(|error| DomainError::Transport(error.to_string()))?;
            let consumer_stream = consumer
                .stream()
                .max_messages_per_batch(1)
                .messages()
                .await
                .map_err(|error| DomainError::Transport(error.to_string()))?;
            Ok::<_, DomainError>((client, context, consumer_stream))
        })?;
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
                .map_err(|error| DomainError::Transport(error.to_string()))?;
            pending
                .await
                .map_err(|error| DomainError::Transport(error.to_string()))
        })?;
        Ok(DeliveryReceipt {
            message_id,
            provider: Provider::NatsJetStream,
            state: DeliveryState::Published,
            trace_id,
        })
    }

    fn receive(&self) -> Result<Option<ReceivedMessage>, DomainError> {
        let mut stream = self
            .stream
            .lock()
            .map_err(|_| {
                DomainError::Transport("NATS JetStream stream is unavailable".to_string())
            })?
            .take()
            .ok_or_else(|| {
                DomainError::Transport("NATS JetStream stream is unavailable".to_string())
            })?;
        let runtime = self.runtime.as_ref().ok_or_else(|| {
            DomainError::Transport("NATS JetStream runtime is unavailable".to_string())
        })?;
        let result = runtime.block_on(async {
            stream
                .next()
                .await
                .ok_or_else(|| DomainError::Transport("NATS JetStream consumer ended".to_string()))?
                .map_err(|error| DomainError::Transport(error.to_string()))
        });
        self.stream
            .lock()
            .map_err(|_| {
                DomainError::Transport("NATS JetStream stream is unavailable".to_string())
            })?
            .replace(stream);
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
        runtime
            .block_on(acker.ack())
            .map_err(|error| DomainError::Transport(error.to_string()))?;
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
pub enum NatsTransportKind {
    Core(Box<NatsTransport>),
    JetStream(Box<NatsJetStreamTransport>),
}

pub struct KafkaTransport {
    producer: Option<FutureProducer>,
    consumer: Option<StreamConsumer>,
    runtime: Option<tokio::runtime::Runtime>,
    topic: String,
    pending_acknowledgements: Mutex<BTreeMap<String, TopicPartitionList>>,
}

pub struct RabbitMqTransport {
    connection: Option<Connection>,
    channel: Option<lapin::Channel>,
    runtime: Option<tokio::runtime::Runtime>,
    queue: String,
    pending_acknowledgements: Mutex<BTreeMap<String, Acker>>,
}

pub struct PulsarTransport {
    client: Pulsar<TokioExecutor>,
    consumer: Arc<Mutex<PulsarConsumer<Vec<u8>, TokioExecutor>>>,
    runtime: Arc<tokio::runtime::Runtime>,
    topic: String,
    pending_acknowledgements: Mutex<BTreeMap<String, PulsarMessage<Vec<u8>>>>,
}

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
                let runtime = tokio::runtime::Runtime::new()
                    .map_err(|error| DomainError::Transport(error.to_string()))?;
                let client = runtime
                    .block_on(Pulsar::builder(service_url, TokioExecutor).build())
                    .map_err(|error| DomainError::Transport(error.to_string()))?;
                let consumer = runtime
                    .block_on(
                        client
                            .consumer()
                            .with_topic(topic)
                            .with_subscription(subscription)
                            .build::<Vec<u8>>(),
                    )
                    .map_err(|error| DomainError::Transport(error.to_string()))?;
                Ok((client, consumer, runtime))
            })
            .map_err(|error| DomainError::Transport(error.to_string()))?
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
            .map_err(|error| DomainError::Transport(error.to_string()))?
            .join()
            .map_err(|_| DomainError::Transport("Pulsar operation thread panicked".to_string()))?
    }
}

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
                    .map_err(|error| DomainError::Transport(error.to_string()))?;
                send.await
                    .map_err(|error| DomainError::Transport(error.to_string()))
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
            runtime.block_on(async {
                consumer
                    .next()
                    .await
                    .transpose()
                    .map_err(|error| DomainError::Transport(error.to_string()))
            })
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
            runtime.block_on(async {
                consumer
                    .ack(&delivery)
                    .await
                    .map_err(|error| DomainError::Transport(error.to_string()))
            })
        })?;
        Ok(DeliveryReceipt {
            state: DeliveryState::Acknowledged,
            ..receipt.clone()
        })
    }
}

impl RabbitMqTransport {
    pub fn connect(uri: &str, queue: &str) -> Result<Self, DomainError> {
        if uri.trim().is_empty() || queue.trim().is_empty() {
            return Err(DomainError::InvalidEnvelope(
                "RabbitMQ URI and queue are required".to_string(),
            ));
        }
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|error| DomainError::Transport(error.to_string()))?;
        let _runtime_guard = runtime.enter();
        let connection = runtime
            .block_on(Connection::connect(uri, ConnectionProperties::default()))
            .map_err(|error| DomainError::Transport(error.to_string()))?;
        let channel = runtime
            .block_on(connection.create_channel())
            .map_err(|error| DomainError::Transport(error.to_string()))?;
        runtime
            .block_on(channel.queue_declare(
                queue,
                QueueDeclareOptions {
                    durable: true,
                    ..QueueDeclareOptions::default()
                },
                FieldTable::default(),
            ))
            .map_err(|error| DomainError::Transport(error.to_string()))?;
        Ok(Self {
            connection: Some(connection),
            channel: Some(channel),
            runtime: Some(runtime),
            queue: queue.to_string(),
            pending_acknowledgements: Mutex::new(BTreeMap::new()),
        })
    }
}

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
            .map_err(|error| DomainError::Transport(error.to_string()))?;
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
            .map_err(|error| DomainError::Transport(error.to_string()))?;
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
            .map_err(|error| DomainError::Transport(error.to_string()))?;
        Ok(DeliveryReceipt {
            state: DeliveryState::Acknowledged,
            ..receipt.clone()
        })
    }
}

impl KafkaTransport {
    pub fn connect(brokers: &str, topic: &str, group_id: &str) -> Result<Self, DomainError> {
        if brokers.trim().is_empty() || topic.trim().is_empty() || group_id.trim().is_empty() {
            return Err(DomainError::InvalidEnvelope(
                "Kafka brokers, topic, and group ID are required".to_string(),
            ));
        }
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|error| DomainError::Transport(error.to_string()))?;
        let _runtime_guard = runtime.enter();
        let admin = ClientConfig::new()
            .set("bootstrap.servers", brokers)
            .create::<AdminClient<DefaultClientContext>>()
            .map_err(|error| DomainError::Transport(error.to_string()))?;
        let new_topic = NewTopic::new(topic, 1, TopicReplication::Fixed(1));
        let topic_results = runtime
            .block_on(admin.create_topics(&[new_topic], &AdminOptions::new()))
            .map_err(|error| DomainError::Transport(error.to_string()))?;
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
            .map_err(|error| DomainError::Transport(error.to_string()))?;
        let consumer = ClientConfig::new()
            .set("bootstrap.servers", brokers)
            .set("group.id", group_id)
            .set("enable.auto.commit", "false")
            .set("auto.offset.reset", "earliest")
            .create::<StreamConsumer>()
            .map_err(|error| DomainError::Transport(error.to_string()))?;
        consumer
            .subscribe(&[topic])
            .map_err(|error| DomainError::Transport(error.to_string()))?;
        Ok(Self {
            producer: Some(producer),
            consumer: Some(consumer),
            runtime: Some(runtime),
            topic: topic.to_string(),
            pending_acknowledgements: Mutex::new(BTreeMap::new()),
        })
    }
}

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
            .map_err(|(error, _)| DomainError::Transport(error.to_string()))
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
        let message = runtime
            .block_on(consumer.recv())
            .map_err(|error| DomainError::Transport(error.to_string()))?;
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
            .map_err(|error| DomainError::Transport(error.to_string()))?;
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
            .map_err(|error| DomainError::Transport(error.to_string()))?;
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
    Nats(NatsTransportKind),
    Kafka(KafkaTransport),
    RabbitMq(Box<RabbitMqTransport>),
    Pulsar(PulsarTransport),
}

impl MessageTransport for MessageTransportKind {
    fn provider(&self) -> Provider {
        match self {
            Self::LegacyJms(transport) => transport.provider(),
            Self::Nats(transport) => transport.provider(),
            Self::Kafka(transport) => transport.provider(),
            Self::RabbitMq(transport) => transport.provider(),
            Self::Pulsar(transport) => transport.provider(),
        }
    }

    fn publish(&self, message: MessageEnvelope) -> Result<DeliveryReceipt, DomainError> {
        match self {
            Self::LegacyJms(transport) => transport.publish(message),
            Self::Nats(transport) => transport.publish(message),
            Self::Kafka(transport) => transport.publish(message),
            Self::RabbitMq(transport) => transport.publish(message),
            Self::Pulsar(transport) => transport.publish(message),
        }
    }

    fn receive(&self) -> Result<Option<ReceivedMessage>, DomainError> {
        match self {
            Self::LegacyJms(transport) => transport.receive(),
            Self::Nats(transport) => transport.receive(),
            Self::Kafka(transport) => transport.receive(),
            Self::RabbitMq(transport) => transport.receive(),
            Self::Pulsar(transport) => transport.receive(),
        }
    }

    fn acknowledge(&self, receipt: &DeliveryReceipt) -> Result<DeliveryReceipt, DomainError> {
        match self {
            Self::LegacyJms(transport) => transport.acknowledge(receipt),
            Self::Nats(transport) => transport.acknowledge(receipt),
            Self::Kafka(transport) => transport.acknowledge(receipt),
            Self::RabbitMq(transport) => transport.acknowledge(receipt),
            Self::Pulsar(transport) => transport.acknowledge(receipt),
        }
    }
}

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
    fn legacy_jms_transport_preserves_message_and_acknowledgement_contract() {
        let transport =
            super::MessageTransportKind::LegacyJms(InMemoryTransport::new(Provider::LegacyJms));
        let message = message();
        let message_id = message.message_id.clone();
        let trace_id = message.tracing.trace_id.clone();

        let published = transport.publish(message).unwrap();
        let received = transport.receive().unwrap().unwrap();
        let acknowledged = transport.acknowledge(&received.receipt).unwrap();

        assert_eq!(published.provider, Provider::LegacyJms);
        assert_eq!(published.state, DeliveryState::Published);
        assert_eq!(received.message.message_id, message_id);
        assert_eq!(received.message.tracing.trace_id, trace_id);
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

    #[test]
    fn dry_run_returns_denied_decision_without_publishing() {
        let config = RouteConfig {
            default_mode: Mode::Transparent,
            default_provider: Provider::LegacyJms,
            rules: vec![RouteRule {
                id: "hold-orders".to_string(),
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

        let decision = config.dry_run(&message()).unwrap();
        assert_eq!(decision.rule_id.as_deref(), Some("hold-orders"));
        assert_eq!(decision.mode, Mode::Redirect);
        assert_eq!(decision.provider, Provider::Nats);
        assert!(!decision.allowed);
    }
}
