# Features
<!-- rev:001 (RFC 3339) 2026-08-14T01:21:30Z -->

What exists in the tree at HEAD `af02427`, and what is proposed. "Implemented" means the code
is present and compiles — it does **not** mean the behavior has been validated against the
Java 6 host product or a real broker. See [ISSUES.md](ISSUES.md) I-010.

## Implemented

### HTTPS / TLS

| Feature | Where |
|---|---|
| Java 6 `HttpsURLConnection`-style facade | `java/.../ModernHttpsURLConnection.java` |
| Request methods, properties, buffered output | `java/.../LegacyHttpRequest.java` |
| Connect + read timeouts covering TCP **and** TLS handshake | `crates/http` |
| Response status, reason phrase, headers, body streams | `java/.../LegacyHttpResponse.java` |
| Indexed header access with status line at index 0 | `ModernHttpsURLConnection` |
| Typed content metadata (`getContentType`, `getContentLength`) | `ModernHttpsURLConnection` |
| Redirect policy + `maxRedirects(int)` | `LegacyHttpRequest` |
| TLS floor 1.2, selectable 1.2 / 1.3 | `crates/tls` |
| Peer certificate + cipher suite access | `LegacyTlsInfo`, `crates/core` |
| Capability bitmask for feature discovery | `LegacyHttpClient.getCapabilities()` |

### Messaging

| Feature | Where |
|---|---|
| JMS-shaped facade: ConnectionFactory / Connection / Session / Producer / Consumer | `java/.../messaging/` |
| Provider selection: `LEGACY_JMS`, `NATS`, `NATS_JETSTREAM`, Kafka, Pulsar, RabbitMQ | `crates/jni`, `crates/messaging` |
| Uniform transport boundary across providers | `crates/messaging` (`MessageTransportKind`) |
| Durable JetStream pull consumer with server-side ack | `NatsJetStreamTransport` |
| In-process `LEGACY_JMS` transparent transport | `InMemoryTransport` |
| Typed delivery receipts + acknowledgement modes | `ModernDeliveryReceipt`, `AcknowledgementMode` |
| Trace context as first-class envelope data (trace/span/parent/state/sampling) | `ModernTraceContext`, `crates/messaging` |
| Routing dispatch with policy, mismatch rejection, auditable receipt | `crates/messaging` |
| Read-only JMX metrics MBean, Java 6-compatible | `ModernMessagingMetricsMBean` |

### Utilities and packaging

| Feature | Where |
|---|---|
| `ModernUuid.v4()` / `.v7()`, `ModernBase64`, `ModernJson` | `crates/core` via JNI |
| Single JAR with per-platform native resources | `docker/java6/Dockerfile` |
| SHA-256 content-addressed native extraction with cleanup on failure | `NativeLoader.java` |
| Cross-compilation to linux-x86_64, linux-aarch64, windows-x86_64 | `cargo-zigbuild` |
| Executable cross-application contract fixtures | `hacks/` |

## Proposed

Derived from [BACKLOG.md](BACKLOG.md); not started unless noted.

| Feature | Milestone | Note |
|---|---|---|
| Versioned envelope schema with documented per-provider mappings | M1 | partial — types exist, schema not versioned |
| JMS API compatibility matrix (every supported method + semantic) | M1 | blocked on identifying the vendor's JMS version |
| Class-loading / packaging model for Java 6 application servers | M1 | |
| Broker-backed transparent pass-through prototype | M1 | needs rollback, redelivery, dead-letter, selectors |
| Full JMX management model (health, route decisions, retries, dead letters) | M1 | metrics MBean exists; management surface does not |
| Routing + redirect policy config (patterns, tenants, predicates, dry-run) | M2 | `dry_run` field exists; behavior unverified |
| Transform envelope: serialization, schema versioning, idempotency keys, replay | M2 | |
| Per-adapter guarantee declarations (ordering, persistence, TLS, auth, DLQ) | M2 | |
| Migration controls: shadow publish, dual delivery, cutover, pause/resume, rollback | M2 | |
| JNDI lookup compatibility | M2 | required for true transparent mode |
| Transactions, selectors, rollback / redelivery, dead-letter | M2 | |

## Explicitly not planned

- Honouring custom Java `HostnameVerifier` / `SSLSocketFactory` — rejected by design (I-008).
- A Java-side JSON object model — would put a modern dependency on the legacy class path (I-007).
- Registering a global URL handler — the HTTPS adapter is constructed explicitly.
