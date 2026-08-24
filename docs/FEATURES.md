# Features
<!-- rev:005 (RFC 3339) 2026-08-24T01:11:47Z -->

What exists in the current tree, and what is proposed. "Implemented" means the code is present;
it does **not** mean the behavior satisfies the intended runtime contract. See
[VERIFICATION.md](VERIFICATION.md) for recorded command and runtime reach; the vendor Java 6
host product remains outside every recorded run. See [ISSUES.md](ISSUES.md) I-010.

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
| Peer certificate + cipher suite access | `LegacyTlsInfo`, `crates/core` (`modernlink-core`) |
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
| Per-provider guarantee table, queryable **before** connecting | `Provider::guarantees()`, `ModernMessagingClient.guaranteesFor(...)` |
| Fail-closed refusal of an unhonourable delivery / ack mode | `ProviderGuarantees::require_*`, `DomainError::Unsupported` |
| TEXT, BYTES and MAP payload categories across the JNI boundary | `ModernPayload`, `messaging_build_payload` |
| Provider transports behind cargo features; a provider compiled out is refused, not rerouted | `crates/messaging` `[features]`, `build_transport` |
| Panics contained at all 28 JNI entry points; a panic becomes a reported error, not UB in the JVM | `jni_guard`, `crates/jni` |
| Broker connects bounded by a deadline, overridable with `MODERNLINK_BROKER_TIMEOUT_SECS` | `block_on_with_timeout`, `crates/messaging` |
| Credentials scrubbed from every transport error before it can reach a Java exception or log | `redact_credentials`, `transport_error` |
| Native handles are registry ids, not raw pointers — a stale handle misses instead of dereferencing | `CLIENTS` / `RESPONSES`, `crates/jni` |
| A contained panic cannot leave a NATS transport permanently broken | `RestoreOnDrop`, `crates/messaging` |
| `receive()` blocking semantics declared per provider and queryable from Java 6 | `ReceiveSemantics`, `ModernReceiveSemantics` |
| Kafka refuses a TLS endpoint rather than connecting in plaintext | `endpoint_requests_tls`, `crates/messaging` |

### Utilities and packaging

| Feature | Where |
|---|---|
| `ModernUuid.v4()` / `.v7()`, `ModernBase64`, `ModernJson` | `crates/core` via JNI |
| Single JAR with per-platform native resources | `docker/java6/Dockerfile` |
| SHA-256 content-addressed native extraction with cleanup on failure | `NativeLoader.java` |
| Cross-compilation to linux-x86_64, linux-aarch64, windows-x86_64 | `cargo-zigbuild` |
| Executable cross-application contract fixtures | `hacks/` |

### Modernization CLI foundation (separate product workspace)

| Feature | Where | Scope / limitation |
|---|---|---|
| Rust-first deterministic Java repository analysis | `cli/crates/analyzer` | Static source facts only; architecture and domain conclusions remain higher-layer inference. |
| Unified static + evolution evidence graph | `modernlink inspect [--history]` | Merges versioned Java and Git adapters into `modernlink.evidence/v1alpha1`; deeper detectors remain pending. |
| Architectural layer and candidate-context report | `modernlink architecture` | Inference/hypothesis states, confidence, evidence IDs, and reasoning are explicit; not a DDD fact assertion. |
| Decomposed modernization seam report | `modernlink seams` | Scores observed vendor, JMS, JNDI, JDBC/persistence, and SOAP import boundaries with evidence-linked components; call/data-flow-specific seam families are pending. |
| Legacy infrastructure inventory | `modernlink inspect` | Emits deterministic import-derived signals for JMS, JNDI, JDBC/persistence, EJB, JTA, JAX-WS, JAXB, JMX, Servlet, RMI, Spring, and server APIs; imports do not prove runtime use. |
| Target-runtime compatibility review | `modernlink compatibility --target <major>` | Emits evidence-linked review findings for observed internal-JDK, Java EE, and application-server imports; it does not claim a readiness percentage or inspect resolved dependencies/runtime behavior yet. |
| Lifecycle status | `modernlink status --journal <events.jsonl> --run-id <id>` | Replays an append-only journal into a machine-readable state snapshot; setup/workspace state creation is still pending. |
| Lifecycle transition | `modernlink lifecycle advance --journal … --run-id …` | Appends only the next valid phase, requires `--approve` for Modernize/Cutover/Detach, and records explicit artifact digests. |
| Evidence-linked migration DAG | `modernlink plan --seams … --compatibility …` | Turns existing seam and compatibility evidence into prerequisite tasks; it proposes no target technology or automatic cutover. |
| Safe local workspace setup | `modernlink setup [repo] [--tools <ids\|all\|none>]` | Interactive terminals get a Space/Enter multi-select with detected harnesses preselected; automated callers must specify `--tools`. Setup creates only `.modernlink/` state/cache directories and its owned manifest while adapter files remain pending ownership contracts. |
| Local prerequisite report | `modernlink doctor [repo]` | Read-only structured report for binary, repository access, Rust Git availability, workspace state, Java/Maven/Gradle launchability, and known harness markers; it intentionally does not parse tool-version text. |
| Canonical lifecycle plugin bundle | `cli/plugin/` | Includes harness-neutral analyze/plan/modernize/verify/migrate/status command contracts, focused skills, specialist-role descriptors, and lifecycle rules; Codex/Claude materialization remains intentionally pending ownership contracts. |
| Explicit plugin materialization | `modernlink plugin install --destination <dir>` | Embeds and writes the canonical bundle only into a new caller-selected directory; it creates no binary pointer and does not guess a harness install location. |
| Local Git evolution evidence | `cli/crates/git`, `modernlink history` | Uses structured `gix` APIs; default artifacts fingerprint commit messages instead of storing message text. |
| Path deltas, bounded co-change, and knowledge signals | `modernlink.git-history/v1alpha1` | Co-change expansion and commit traversal report caps explicitly; no contributor productivity ranking. |
| Git evolution x-ray | `modernlink evolution --history …` | Produces changed-path hotspots, co-change relationships, and contributor continuity from structured Git facts; it deliberately avoids productivity/quality rankings. |
| Repository-local Git cache | `.modernlink/cache/git/` | Cache is ignored, input-keyed, and local only. |
| `analyze --history` sibling artifact | `cli/crates/modernlink-cli` | Writes `analysis.json` and `git-history.json` without merging schemas; publication refuses to replace reviewed outputs. |
| Plugin binary pointer binding | `modernlink plugin bind`, `cli/plugin/` | Binds an explicitly materialized plugin bundle to one local binary without overwriting a pointer; harness-path installation remains pending. |

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
| STREAM and OBJECT payload categories | — | **not planned by default** — see "Explicitly not planned" |
| Per-adapter guarantee declarations for TLS, auth and DLQ | M2 | ordering / persistence / ack / transactions / redelivery / replay are **done** — see [providers.md](providers.md). TLS and auth are deliberately absent until broker connections terminate through `crates/tls` |
| Migration controls: shadow publish, dual delivery, cutover, pause/resume, rollback | M2 | |
| JNDI lookup compatibility | M2 | required for true transparent mode |
| Transactions, selectors, rollback / redelivery, dead-letter | M2 | |

## Explicitly not planned

- **`OBJECT` payloads (JMS `ObjectMessage`).** Reconstructing one means deserializing
  broker-supplied bytes into Java objects, a remote-code-execution surface. A compatibility
  layer fronting a locked-down legacy application must not open that by default. Callers who
  accept the risk can use `BYTES` and deserialize explicitly.
- **`STREAM` payloads.** The frame does not encode the typed field ordering a `StreamMessage`
  exists to carry, and delivering it as opaque bytes would lose that structure silently.
- Honouring custom Java `HostnameVerifier` / `SSLSocketFactory` — rejected by design (I-008).
- A Java-side JSON object model — would put a modern dependency on the legacy class path (I-007).
- Registering a global URL handler — the HTTPS adapter is constructed explicitly.
