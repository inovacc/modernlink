# ModernLink Messaging Compatibility Backlog
<!-- rev:027 (RFC 3339) 2026-08-21T22:45:19Z -->

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

Tracing is first-class domain data, not only a user property or transport
header. Every envelope carries a trace ID, span ID, optional parent span ID,
optional trace state, and sampling decision. Transparent mode forwards it;
transform mode maps it to the target provider's tracing metadata; redirect mode
preserves it across the route decision. Provider adapters must not silently
replace or discard these fields.

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

Current implementation slice: the Java facade now exposes a JMS-shaped
`ConnectionFactory`/`Connection`/`Session`/`MessageProducer`/`MessageConsumer`
surface backed by the native NATS adapter. It preserves message identity,
acknowledgement receipts, and trace context, and exposes read-only counters via
`ModernMessagingMetricsMBean`. This is a concrete NATS path, not completion of
the provider-neutral compatibility scope below. The Rust messaging crate also
contains a JetStream transport that uses a durable pull consumer and server-side
acknowledgement, and the Java/JNI provider selection surface accepts
`NATS_JETSTREAM`. Kafka, RabbitMQ, and Pulsar adapters are available through the
same uniform transport boundary and Java/JNI provider selection; their broker
fixtures remain separate runtime evidence paths.

The `LEGACY_JMS` provider now has an in-process compatibility transport for
transparent-mode contract fixtures. It is not yet a vendor-broker JMS bridge;
JNDI, transactions, selectors, rollback/redelivery, and dead-letter behavior
remain provider-adapter work.

## Backlog items

### M1 — Define the canonical message domain

Create provider-neutral types for envelope metadata, payload variants,
properties, delivery outcome, acknowledgement, retry, and dead-letter state.
Define which fields are required, optional, immutable, or provider-specific.

Acceptance criteria:

- a versioned envelope schema exists;
- mappings to JMS, Kafka, Pulsar, NATS, and RabbitMQ are documented;
- unsupported mappings fail explicitly rather than being silently dropped.
- trace context is preserved as a typed envelope field across all modes.
- acknowledgement mode and typed delivery receipts are preserved across the
  uniform transport boundary.
- routing dispatch applies policy, rejects provider mismatches, and returns an
  auditable publish receipt.

### M1 — Maintain cross-application contract fixtures

Keep the executable JMS/JMX-shaped publisher and modern-provider consumer
fixtures under `hacks/`. They must exchange the same provider-neutral envelope
while switching between transparent, transform, and redirect modes. The
fixtures are deterministic contract probes; they are not substitutes for
external broker integration.

Acceptance criteria:

- separate publisher and consumer processes exchange a message;
- Kafka, Pulsar, NATS, and RabbitMQ provider identities can be selected by the
  same consumer contract;
- destination, payload, message ID, and trace context remain observable after
  switching mode;
- the Java 6 fixture registers a JMX metrics MBean without requiring Java 8.

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

## Engineering hygiene and tech debt

Added 2026-08-14 from a docs/state reconciliation. Tasks are broken out in
[IMPLEMENTATION_TASKS.md](IMPLEMENTATION_TASKS.md); the constraints behind them are in
[ISSUES.md](ISSUES.md).

### ~~P1 — no crate declares `publish = false` (SC-01)~~ — **DONE `315fe87`**

All six manifests now carry `publish = false`. Original text retained below for history.


None of the six manifests blocks publication, and the crate names (`core`, `http`, `tls`,
`jni`) are exactly the ones most likely to collide on crates.io. One line per manifest.
_Evidence:_ no `publish` key in `crates/{core,http,tls,jni,messaging}/Cargo.toml` or
`hacks/messaging-demo/Cargo.toml`. See ISSUES I-012.

### P1 — broker-backed evidence covers only one happy path per provider (VER-01, VER-02)

The repository contains five explicit broker tests. Run
[32386474212](https://github.com/inovacc/modernlink/actions/runs/32386474212) at `3b64484`
recorded `success` conclusions for the dedicated NATS/JetStream/RabbitMQ and Kafka/Pulsar jobs.
That is a machine fact about one configured send/receive/ack path, not a delivery-semantics
verdict. What remains open:

- All five tests are `#[ignore]`d and ordinary workspace test commands execute none of them.
  Dedicated workflow jobs must invoke them explicitly.
- Only the happy path has a recorded broker run. Durability, acknowledgement under failure, reconnect,
  ordering, concurrency and redelivery remain source-level claims for **every** provider.

See ISSUES I-010.

### ~~P2 — `crates/http` coverage is structurally capped (H-12)~~ — **MEASUREMENT BLOCKER REMOVED**

The missing local test-root seam still exists, but it no longer caps measurement. The Rust
coverage harness loads an instrumented native library from Java and sends real HTTPS requests
through the shipped JNI boundary. Run 32523731422 includes `crates/http` in the thresholded
behavior-crate report. The remaining improvement is determinism: replace reliance on a public
HTTPS endpoint with a security-reviewed test-only trust seam without making custom roots
available in production.

### ~~P2 — coverage cannot be measured (SC-04)~~ — **GATES RECORDED AT `686adaa`**

The workflow enforces separate 90% line thresholds. Run
[32523731422](https://github.com/inovacc/modernlink/actions/runs/32523731422) recorded:

- Rust: `scripts/run_rust_coverage.sh` cleans profiles, extracts inline unit bodies to the
  reporter-excluded `src/tests.rs` files, then combines unit tests, instrumented Java→JNI calls,
  demo binaries, and sequential live-broker/fault paths. The production behavior crates
  (`core`, `http`, `messaging`, `tls`) recorded 1,496/1,650 lines (90.67%); the full production
  report, including JNI ABI glue and demo CLIs, recorded 2,814/3,075 lines (91.51%).
- Java: JaCoCo runs on JDK 8 against the same classes compiled for Java 6; the packaged Java 6
  runtime path is a separate step. The production classes recorded 802/889 lines (90.21%).

No percentage establishes delivery durability, reconnect, ordering, rollback/redelivery, or
vendor-host compatibility; those remain separate verification work.

### ~~P2 — CI does not enforce formatting or lint (SC-04)~~ — **DONE `dd080b2`**

`.github/workflows/test.yml` now runs `cargo fmt --all -- --check` and
`cargo clippy --workspace --all-targets -- -D warnings` alongside test and check. Recorded
command results and revision scope are maintained in [VERIFICATION.md](VERIFICATION.md).
Original text and correction history retained below.

**Correction history (2026-08-14).** An earlier revision claimed both commands "pass today"
while the same file recorded an `unreachable_patterns` warning that `-D warnings` would promote
to an error — an unverified claim the file itself contradicted. Both were then actually run:
clippy reported **six** findings, including two `MutexGuard`s held across an `.await` in the
Pulsar transport (a real deadlock risk), all fixed in `242cb3c`. The gates were added to CI in
`dd080b2`.

**Recorded state:** run 31782837766 reported exit-successful test/check/fmt/clippy steps at
`d2479bd`; a local 2026-08-19 fmt command exited 0. These are command facts, not runtime or
contract validation.

### ~~P2 — seven of ten Java test classes never run (VER-03)~~ — **DONE `dd080b2`**

The workflow discovers all no-argument compiled `*Test.class` files instead of maintaining
a fixed list. There are 19 Java test sources: 18 no-argument probes and one parameterized broker
probe invoked explicitly with NATS. Run 32523731422 recorded the Java 6 integration job with a
`success` conclusion after executing both groups.

### ~~P2 — unreachable match arm in the JNI provider dispatch~~ — **CLOSED**

`cargo clippy --workspace --all-targets -- -D warnings` passes on the runner (run
31782837766), which it could not do while an `unreachable_patterns` warning existed. The `_ =>`
arm now surviving at `crates/jni/src/lib.rs:265-268` is a *payload* fallback, not a provider one,
and it is reachable: it rejects every non-text `Payload` variant. Original text below.

_`cargo check` reports `unreachable_patterns` at `crates/jni/src/lib.rs:264`: the `_ =>` fallback
is dead because the `Provider` arms above it are already exhaustive._

### ~~P3 — no toolchain pin or declared MSRV (SC-02, SC-03)~~ — **DONE**

`rust-toolchain.toml` pins `1.96.0` (with `rustfmt` and `clippy`), and
`[workspace.package] rust-version = "1.96"` is inherited by all six crates. The CI job no longer
installs a toolchain of its own and `docker/java6/Dockerfile` COPYs the file in before any cargo
call, so the gate and the packaging image now read the same source.

Verified locally: `rustup show active-toolchain` reports
`1.96.0-… (overridden by 'rust-toolchain.toml')` and `cargo metadata` shows `1.96` on all six
packages. **The CI and Docker halves are unproven** — a workflow edit cannot be verified locally,
and the Docker build was not run (no daemon on the authoring machine).

The declared MSRV is the version the project is built and gated with. **No older toolchain has
been tried**, so it is a pin, not a measured floor.

Original text: _CI resolves `dtolnay/rust-toolchain@stable` while the packaging image pins
`rust:1.96-bookworm`, so the two can drift apart. No crate declares `rust-version`._

### P1 — broker connections are not TLS-terminated, and Kafka cannot be at all (H-07)

No provider connection terminates through `crates/tls`; only the HTTPS path does. There is no
way for a caller to request TLS explicitly — the endpoint scheme is the only signal the API
carries — so a deployment cannot state its intent and have it enforced.

`rustls` is present in the graph for NATS/JetStream (`async-nats`), RabbitMQ
(`rustls-connector`) and Pulsar (`tokio-rustls`), so those three would negotiate TLS from
`tls://`, `amqps://` and `pulsar+ssl://` respectively — **declared, never tested against a
broker**. **Kafka cannot**: `rdkafka` is compiled with `cmake-build` and `libz` only, with no
`ssl` feature, so librdkafka has no OpenSSL and the transport sets no `security.protocol`.

Partially closed: a TLS-looking Kafka endpoint is now **refused** rather than connected in
plaintext, because a deployment that believes its credentials are encrypted and is wrong is
worse off than one that gets an error. What remains:

- enable `rdkafka`'s `ssl` feature (adds an OpenSSL build to the `kafka` feature),
- expose TLS configuration through the Java API instead of inferring it from a scheme,
- terminate through `crates/tls` so brokers inherit the TLS 1.2 floor the HTTPS path has,
- and actually verify a negotiated protocol against a TLS broker, the way the HTTPS path
  records `tls-protocol=TLSv1_3`.

### ~~P1 — broker connects have no timeout and block a JVM thread indefinitely~~ — **BOUNDED**

`block_on_with_timeout` now bounds setup for NATS, JetStream, RabbitMQ, Kafka, and Pulsar;
environment overrides retain the documented 10s/30s defaults. Unit commands exercise timeout
selection, but no recorded live run simulates a broker that accepts TCP and stalls its handshake,
so cancellation behavior at that boundary remains unproven. Found by `/project:harden` (H-02).

### P2 — `crates/http` still shadows the external `http` crate (SC-05/SC-06 missed it)

`cargo check -p http` is ambiguous and must be spelled `-p http@0.1.0`, exactly the footgun
SC-05 and SC-06 removed for `jni` and `core`. Found while touching `crates/tls`. `tls` and
`messaging` are unambiguous; `http` is the last one. Same fix as the others: give the package
a distinguishing name and keep the folder path.

### P2 — `PulsarTransport` is the only transport without an explicit `Drop`

`impl Drop` exists for `NatsTransport`, `NatsJetStreamTransport`, `RabbitMqTransport` and
`KafkaTransport`, and not for `PulsarTransport` — which owns
`runtime: Arc<tokio::runtime::Runtime>` and spawns named worker threads.

**Investigated 2026-08-19 and downgraded.** The stated hazard — dropping a tokio `Runtime`
from inside an async context, which panics — is **not reachable through the JNI path**:
`PulsarTransport` is dropped when the client leaves the handle registry, which happens on the
JVM thread that called `nativeClose`, never on a runtime worker. The residual value is a
graceful consumer close, and unacknowledged messages are redelivered anyway, so nothing is
lost by its absence.

The clean fix is to make the fields `Option<_>` so `Drop` can `take()` them the way the other
four do. That is a struct-wide change to a transport with only one recorded happy-path broker
round trip and no shutdown/reconnect probe. Deferred deliberately rather than forced.
`/project:harden` H-06.

### ~~P2 — no dependency vulnerability audit has ever been run~~ — **AUDIT IS A BLOCKING GATE; B-009 RESOLVED**

The workflow audit is now a blocking release-readiness job. The broker-client upgrades in
`f991820` remove the B-009 dependency set, and local `cargo audit --deny warnings` exits 0.
The post-change GitHub run remains the machine record required before a release-readiness
conclusion can be recorded. `/project:harden` H-09.

### P3 — two panic sites that are correct today and fragile tomorrow

`crates/tls/src/lib.rs:43` `.expect("ModernLink TLS versions are supported")` asserts a
*dependency's* behaviour rather than this crate's own invariant. `crates/http/src/lib.rs:43`
`location.as_ref().unwrap()` is safe only because of an `is_none()` early return six lines
above — restructure as `let Some(location) = location else { … }` so the compiler enforces
what convention currently enforces. `/project:harden` H-10, H-14.

### P1 — `delivery_mode` is requested by every message and honoured by none (B-003)

Every `MessageEnvelope` defaults to `DeliveryMode::Persistent` and **no transport reads the
field**. RabbitMQ is the sharpest case: a `durable: true` queue fed by a publisher sending
`BasicProperties::default()` (transient), so messages do not survive a restart while the queue
looks durable. This is a silently degraded delivery guarantee, which the AGENTS.md hard rules
forbid outright. Filed as [BUGS.md](BUGS.md) B-003; `require_delivery_mode` exists and is
deliberately **not** wired into the publish path, because doing so changes delivery semantics on
the default path and that is the maintainer's call.

### ~~P3 — crate names collide with well-known crates (SC-05, SC-06)~~ — **DONE**

`crates/jni` is now the package `jni-bridge` and `crates/core` is `modernlink-core`; both keep
their short folder paths. `cargo check -p jni` is unambiguous again and `-p jni@0.1.0` is
retired. `modernlink-core` is the carve-out the crate-naming rule allows for a name that would
otherwise shadow a Rust built-in; `jni-bridge` is not a registry-uniqueness prefix, it
disambiguates from a real dependency in this graph. `[lib] name` stays `modernlink`, so the
native artifact is unchanged. Original text follows.


`crates/jni` shadows the external `jni` crate it depends on, forcing `-p jni-bridge` in CI and
the Dockerfile. `crates/core` shadows Rust's built-in `core`. Both work today; both are traps.
See ISSUES I-001, I-002.

### P3 — each crate is a single `lib.rs` (DOC-02)

All five crates are one file. `crates/messaging` carries six transports plus the domain model in
a single module, which is the hardest file in the repo to review.

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
