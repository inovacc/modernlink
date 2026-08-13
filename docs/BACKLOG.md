# ModernLink Messaging Compatibility Backlog

## Objective

Allow the vendor-locked Java 6 application to keep using its existing JMS and
JMX contracts while ModernLink evolves the transport behind the native
boundary. The application should be able to connect to existing infrastructure
without source changes, then move selected traffic to modern providers through
configuration.

JMS is the application-facing messaging contract. JMX is the management and
observability contract; it is not a message transport. Both must remain
available independently.

## Operating modes

### 1. Transparent pass-through

The legacy JMS calls and message semantics are preserved while messages flow
through the existing provider or broker.

Required behavior:

- preserve destination names, message properties, headers, correlation IDs,
  selectors, acknowledgement mode, ordering, transactions, retries, and
  dead-letter behavior where the provider supports them;
- preserve synchronous receive and listener/callback behavior expected by the
  Java 6 application;
- preserve JNDI lookup behavior where the host application uses it;
- expose the same operational state through JMX-compatible MBeans;
- make the mode selectable without changing application code.

Transparent mode must not silently alter delivery guarantees. Any provider
feature that cannot be preserved must be reported as an explicit capability
gap before traffic is moved.

### 2. Transform mode

The JMS message is converted into a provider-neutral ModernLink message
envelope, then encoded for a target system such as Kafka, Pulsar, NATS, or
RabbitMQ.

The envelope must define stable mappings for:

- message ID, correlation ID, timestamp, expiration, priority, and delivery
  mode;
- destination, source, tenant, and routing metadata;
- text, bytes, map, stream, and object payload categories;
- application properties and reserved transport headers;
- tracing context and retry/dead-letter metadata;
- acknowledgement outcome and replay/idempotency identity.

Transform mode must document information loss, ordering scope, retry behavior,
and the point at which acknowledgement is considered committed.

### 3. Redirect mode

The legacy JMS call remains the application-facing API, but routing sends the
message directly to a selected modern provider without requiring a payload
transformation beyond the agreed envelope mapping.

Redirect rules should support destination/provider mappings, tenant or header
conditions, allow/deny policies, fallback behavior, and dry-run decisions.
Every routing decision must be observable through JMX and structured native
diagnostics.

## Proposed boundary

```text
Java 6 application
        |
        +--> JMS compatibility facade / provider adapter
        +--> JMX-compatible management facade
        |
        v
ModernLink message domain
        |
        +--> transparent pass-through
        +--> transform
        +--> redirect
        |
        +--> legacy JMS provider
        +--> Kafka adapter
        +--> Pulsar adapter
        +--> NATS adapter
        +--> RabbitMQ adapter
```

The Java facade should remain Java 6-compatible. Provider clients and modern
protocol libraries belong behind Rust/Cargo-backed adapters or isolated native
components so the legacy class path does not need modern Java dependencies.

## Backlog items

### M1 — Define the canonical message domain

Create provider-neutral types for envelope metadata, payload variants,
properties, delivery outcome, acknowledgement, retry, and dead-letter state.
Define which fields are required, optional, immutable, or provider-specific.

Acceptance criteria:

- a versioned envelope schema exists;
- mappings to JMS, Kafka, Pulsar, NATS, and RabbitMQ are documented;
- unsupported mappings fail explicitly rather than being silently dropped.

### M1 — Specify JMS compatibility surface

Inventory the exact JMS interfaces and versions used by the vendor product,
including `ConnectionFactory`, `Connection`, `Session`, `MessageProducer`,
`MessageConsumer`, `MessageListener`, destination types, transactions, and
acknowledgement modes.

Decide whether compatibility is provided by a binary-compatible `javax.jms`
provider façade, a source-compatible `com.modernlink.jms` façade, or a vendor
adapter. The decision must account for class-loader conflicts with the
existing application server.

Acceptance criteria:

- an API compatibility matrix identifies every supported method and semantic;
- class-loading and packaging behavior is defined for Java 6 application
  servers;
- transparent mode can be selected without application source changes.

### M1 — Define the JMX management model

Specify MBeans for provider health, connection/session state, route decisions,
queue/topic metrics, retries, dead letters, inflight messages, and security
configuration. Preserve stable object names and attribute meanings across
transport providers.

Acceptance criteria:

- existing monitoring can discover the MBeans;
- read-only operational metrics are separated from mutating controls;
- sensitive payloads, credentials, and message bodies never appear in JMX
  attributes or logs.

### M1 — Implement transparent pass-through prototype

Wrap one existing JMS provider and prove that the application-facing contract
preserves acknowledgement, transactions, ordering, selectors, listeners,
timeouts, and redelivery behavior.

Acceptance criteria:

- a broker-backed integration fixture exercises send, receive, listener,
  rollback, redelivery, and dead-letter paths;
- before/after message metadata comparisons are recorded;
- capability gaps are visible before enabling the mode.

### M2 — Implement routing and redirect policy

Add configuration for exact destination mappings, pattern mappings, tenant
rules, header predicates, priority, fallback, and dry-run evaluation.

Acceptance criteria:

- a message route is deterministic and explainable;
- policy changes are versioned and auditable;
- a failed target does not silently acknowledge the legacy message.

### M2 — Implement transform envelope and replay controls

Add serialization, schema versioning, idempotency keys, trace propagation,
retry classification, and replay tooling.

Acceptance criteria:

- round-trip mappings preserve all supported JMS fields;
- duplicate delivery and redelivery are distinguishable;
- poison messages can be quarantined without blocking unrelated traffic.

### M2 — Add provider adapters

Implement adapters in this order unless operational evidence changes the
priority:

1. RabbitMQ for queue-oriented interoperability;
2. Kafka for durable partitioned event streams;
3. NATS for lightweight low-latency messaging;
4. Pulsar for multi-tenant durable streams and queue semantics.

Each adapter must declare its guarantees for ordering, persistence,
acknowledgement, transactions, replay, backpressure, TLS, authentication, and
dead letters.

### M2 — Add migration and rollback controls

Support shadow publishing, sampled dual delivery, cutover by destination or
tenant, pause/resume, replay, and rollback to transparent mode.

Acceptance criteria:

- migration can be rehearsed without acknowledging the target as authoritative;
- cutover and rollback are observable through JMX;
- an operator can identify in-flight and duplicated messages.

## Cross-cutting constraints

- Java 6 source and runtime compatibility remains mandatory at the application
  boundary.
- No provider-specific dependency may leak into the legacy application’s class
  path unless explicitly chosen by deployment.
- TLS, authentication, authorization, and credential storage must be defined
  per provider.
- Delivery semantics are part of the contract, not an implementation detail.
- Configuration must fail closed when a requested guarantee is unsupported.
- The JAR remains the primary distributable for the Java façade; native
  artifacts continue to use the Snappy-style architecture/resource split.

## Open decisions

- Exact JMS version and vendor implementation used by the locked product.
- Whether binary compatibility with `javax.jms` is technically safe in the
  target application server.
- Whether JMX calls are local, remote, or broker-mediated.
- Canonical envelope encoding: JSON, binary schema, or both.
- At-least-once versus exactly-once claims per provider and per mode.
- Configuration source and reload model.
- Required Java 6 application-server class-loader isolation.
