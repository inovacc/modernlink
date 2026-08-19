# ModernLink Messaging Compatibility Backlog
<!-- rev:005 (RFC 3339) 2026-08-19T00:00:00Z -->

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

### P1 — broker-backed evidence is partial and not reproducible (VER-01, VER-02)

**Narrowed 2026-08-14 by `a2419b5`.** `crates/messaging/tests/broker_backed.rs` now runs one
send/receive/ack round trip against live **NATS core, NATS JetStream and RabbitMQ**. What
remains open, and why this is still P1 and still the largest gap:

- **Kafka and Pulsar have no broker-backed test at all.**
- All three existing tests are `#[ignore]`d (`:117`, `:132`, `:149`), so **no CI run executes
  them** — the evidence is one operator's manual run against local containers (VER-01).
- Only the happy path is covered. Durability, acknowledgement under failure, reconnect,
  ordering, concurrency and redelivery remain source-level claims for **every** provider.

See ISSUES I-010.

### P2 — coverage cannot be measured (SC-04)

`cargo llvm-cov --workspace --summary-only` fails on Windows: `combine`, `lapin`, `pulsar`, and
`async-nats` all fail to compile under coverage instrumentation. No coverage number exists for
this repo. Running llvm-cov inside the Linux container is the likely fix. The Java facade has no
coverage tooling at all, since there is no Maven or Gradle build.

### ~~P2 — CI does not enforce formatting or lint (SC-04)~~ — **DONE `dd080b2`**

`.github/workflows/test.yml` now runs `cargo fmt --all -- --check` and
`cargo clippy --workspace --all-targets -- -D warnings` alongside test and check; all four
passed on run
[31782837766](https://github.com/inovacc/modernlink/actions/runs/31782837766) at `d2479bd`.
Original text and correction history retained below.

**Correction history (2026-08-14).** An earlier revision claimed both commands "pass today"
while the same file recorded an `unreachable_patterns` warning that `-D warnings` would promote
to an error — an unverified claim the file itself contradicted. Both were then actually run:
clippy reported **six** findings, including two `MutexGuard`s held across an `.await` in the
Pulsar transport (a real deadlock risk), all fixed in `242cb3c`. The gates were added to CI in
`dd080b2`.

**Current state (2026-08-19):** all four gates ran and passed on the runner —
`test`, `check`, `fmt` and `clippy`, run
[31782837766](https://github.com/inovacc/modernlink/actions/runs/31782837766) at `d2479bd`. The
earlier `fmt` failure on `crates/messaging/tests/broker_backed.rs` was fixed before that file was
committed in `a2419b5`; `cargo fmt --all -- --check` → **exit 0**, verified independently by
Codex on 2026-08-19.

### ~~P2 — seven of ten Java test classes never run (VER-03)~~ — **DONE `dd080b2`**

The workflow now enumerates all **13** classes (the count grew from ten: VER-05 added the native
smoke test, VER-08 added two messaging tests), and all 13 executed and passed on runs
31781200582 and 31782837766. Original text retained below for history.


CI executes `LegacyHttpResponseStructuredTest`, `ModernHttpsURLConnectionTest`, and
`LegacyHttpsTest`. The other seven under `java/src/test/java/com/modernlink/` are compiled into
the JAR but never invoked — a test the workflow does not call never runs.

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

### P3 — crate names collide with well-known crates (SC-05, SC-06)

`crates/jni` shadows the external `jni` crate it depends on, forcing `-p jni@0.1.0` in CI and
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
