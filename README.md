# ModernLink
<!-- rev:011 (RFC 3339) 2026-08-21T20:41:35Z -->

[![Dependabot Updates](https://github.com/inovacc/modernlink/actions/workflows/dependabot/dependabot-updates/badge.svg)](https://github.com/inovacc/modernlink/actions/workflows/dependabot/dependabot-updates)
[![CI](https://github.com/inovacc/modernlink/actions/workflows/test.yml/badge.svg)](https://github.com/inovacc/modernlink/actions/workflows/test.yml)

⚠️ Still actively under development ⚠️

*(formerly "Legacy Exit Gateway SDK" — the product name is ModernLink, matching `AGENTS.md`,
the `modernlink` native library, and the `inovacc/modernlink` repository.)*

## Purpose

ModernLink is a compatibility layer for a legacy product that must remain running on Java 6
while communicating with modern services and protocols.

It adds current messaging, TLS, and network clients without requiring a migration of the main
product.

## Known constraints

- The host product remains on Java 6 because of vendor lock-in.
- Java 6 does not include lambdas, method references, or the Java 8 Streams API.
- Legacy enterprise APIs—such as JMS, JMX, JNDI, JAXB, and JAX-WS—must not be assumed to be equivalent to modern alternatives.
- Current versions of many libraries may not support Java 6.
- Cryptographic and TLS capabilities available in the legacy runtime may be insufficient for current requirements.
- The integration must avoid instability, memory leaks, and ABI incompatibilities in the legacy Java process.

## Architecture hypothesis

```text
Java 6 application
        |
        v
Java 6-compatible JAR facade
        |
        v
Stable JNI boundary
        |
        v
Rust native library
        |
        +--> Modern TLS
        +--> Modern HTTP
        +--> RabbitMQ / Kafka / NATS / JetStream / Pulsar
```

The JAR should expose a small, stable API to Java 6 code. Modern implementation details are encapsulated in Rust, and platform-specific native binaries are distributed alongside the JAR.

The messaging compatibility backlog is documented in [`docs/BACKLOG.md`](docs/BACKLOG.md). It separates the JMS application contract from JMX management and defines transparent pass-through, transform, and redirect modes for legacy infrastructure and modern providers.

## Samples: legacy APIs through modern providers

The [`samples/`](samples/README.md) directory shows how a Java 6 host can keep its
JMS-shaped application flow while ModernLink carries messages through the stable JNI
boundary into Rust and an explicitly selected provider. It includes flow diagrams and
small examples for JMS-style send/receive, JMX metrics, JAXB XML payloads, and a JAX-WS
service endpoint. “JABX” is not a ModernLink API; the sample name uses the standard
Java 6 API spelling, JAXB.

The examples are integration blueprints, not evidence that a vendor host satisfies the
contract. Provider selection remains explicit (`KAFKA`, `RABBITMQ`, `NATS`,
`NATS_JETSTREAM`, or `PULSAR`). Unsupported acknowledgement modes are refused; delivery-mode
enforcement is still open as [BUGS.md](docs/BUGS.md) B-003 and must not be inferred from a
sample.

Executable cross-application contract fixtures live under [`hacks/`](hacks/README.md). They include a JMS/JMX-shaped publisher, a provider-neutral consumer, and native broker probes for NATS, Kafka, Pulsar, and RabbitMQ, with first-class trace ID and span ID propagation. The `LEGACY_JMS` transparent fixture uses an in-process compatibility transport; it is not a vendor JMS broker bridge.

## Java 6 HTTPS adapter

The packaged JAR also provides `com.modernlink.ModernHttpsURLConnection`, a Java 6-compatible `HttpsURLConnection`-style facade over the ModernLink request API. It supports request methods and properties, connect/read timeouts covering TCP connection and TLS handshake, buffered request output, response streams, status and headers, redirect policy inherited from `LegacyHttpRequest`, and TLS cipher/certificate access.

The messaging facade can select `LEGACY_JMS`, `NATS`, durable `NATS_JETSTREAM`,
Kafka, Pulsar, or RabbitMQ through the same Java 6 connection factory; provider
selection remains explicit and does not imply vendor-level JMS compatibility.

Each provider's delivery guarantees are declared in code and queryable **before** any
traffic moves — `ModernMessagingClient.guaranteesFor(provider)` returns ordering,
persistence, acknowledgement, transaction, redelivery, dead-letter and replay support,
each marked `VERIFIED` (a test has run), `DECLARED` (implemented, untested) or
`UNSUPPORTED`. The reasoning per provider is in [`docs/providers.md`](docs/providers.md).
Acknowledgement-mode checks can refuse unsupported requests. **Delivery-mode enforcement is not
wired into the publish path yet**: [BUGS.md](docs/BUGS.md) B-003 records the resulting persistent
delivery downgrade risk, including RabbitMQ publishing transient messages to a durable queue.

Message bodies may be text, raw bytes, or a string map. `OBJECT` and `STREAM` bodies are
deliberately refused with their reason — see [`docs/FEATURES.md`](docs/FEATURES.md).

The provider transports are behind cargo features and **off by default**, so a broker-free
`cargo test --workspace` compiles no broker client at all; the distributable is built with
`--features all-providers`.

The adapter is created explicitly with `new ModernHttpsURLConnection(new URL("https://..."))`; it does not register a global URL handler. The Docker build produces the distributable artifact at `/workspace/modernlink.jar`, with the platform native library embedded under the JAR's native resource path.

The response model preserves the HTTP status reason phrase, exposed as `LegacyHttpResponse.getStatusMessage()` and `ModernHttpsURLConnection.getResponseMessage()`. The adapter also exposes indexed headers and typed content metadata through `getHeaderField(int)`, `getHeaderFieldKey(int)`, `getContentType()`, and `getContentLength()`. Header access lazily connects and exposes the HTTP status line at indexed header `0`, matching the Java URL-connection convention.

`LegacyHttpClient.getCapabilities()` provides a stable bitmask for feature discovery before requests. The current bits identify HTTPS, TLS 1.2, TLS 1.3, redirects, and peer-certificate access.

The Java facade also exposes independent Cargo-backed utilities: `ModernUuid.v4()`, `ModernUuid.v7()`, `ModernBase64.encode(byte[])`, `ModernBase64.decode(String)`, `ModernJson.object(...)`, `ModernJson.array(...)`, and `ModernJson.decode(String)`. JSON decode returns normalized JSON text because Java 6 has no standard JSON object model.

The current JAR contains Linux x86_64, Linux ARM64, and Windows x86_64 native resources:

- `native/linux-x86_64/libmodernlink.so`
- `native/linux-aarch64/libmodernlink.so`
- `native/windows-x86_64/modernlink.dll`

Native extraction hashes the embedded bytes with SHA-256, writes to a unique temporary file, and renames it into a deterministic content-addressed path before loading. A later JVM process can reuse the extracted file; temporary files are removed when extraction or loading fails.

TLS policy defaults to a minimum of TLS 1.2. Java callers may select `LegacyHttpRequest.TLS_1_2` or `LegacyHttpRequest.TLS_1_3` with `minimumTlsVersion(...)`; unsupported protocol values are rejected before the native request is started.

`ModernHttpsURLConnection` forwards its instance redirect flag and exposes `maxRedirects(int)`. Custom Java `HostnameVerifier` and `SSLSocketFactory` instances are explicitly rejected because the connection is terminated and verified by Rust; accepting those objects while ignoring them would create a misleading security contract.

**What has and has not executed at runtime** is maintained in
[`docs/VERIFICATION.md`](docs/VERIFICATION.md). Machine results below are observations at their
recorded revisions, not a verdict that the integration satisfies the vendor contract:

- **Recorded on a real Java 6 JVM** (`1.6.0_38`, `linux-x86_64`, packaged JAR, CI run [31781200582](https://github.com/inovacc/modernlink/actions/runs/31781200582)): native loading, live TLSv1_3 HTTPS, the JMS-shaped facade, and routing probes executed.
- **Executed on a modern JVM** (Windows, JVM 21): the same native path on **windows-x86_64**, `status=200` with a 4-certificate chain, and a send → receive → acknowledge round trip against **live NATS, NATS JetStream and RabbitMQ**.
- CI run [32386474212](https://github.com/inovacc/modernlink/actions/runs/32386474212) at `3b64484` recorded the configured broker tests for all five providers and a linux-aarch64 native load on JVM 21.
- CI run [32523731422](https://github.com/inovacc/modernlink/actions/runs/32523731422) at `686adaa` recorded `success` conclusions for all seven jobs. Its reports recorded Rust behavior-crate line coverage at **1,496/1,650 (90.67%)**, full Rust production-source coverage at **2,814/3,075 (91.51%)**, and Java production-class coverage at **802/889 (90.21%)**.
- **Not executed:** the vendor host product and its JMS implementation. Durability across restart, reconnect, ordering under load, concurrency, failure recovery, rollback, redelivery, and dead-letter behavior remain unexercised for every provider.

The integration approach itself is no longer open: embedded JNI was chosen over an external sidecar process, recorded in [`docs/adr/0001-jni-boundary-over-sidecar.md`](docs/adr/0001-jni-boundary-over-sidecar.md) (Status: Accepted). The section below is retained as the rationale behind that decision, not as an open question.

## Integration alternatives

### Embedded JNI

The JAR loads a Rust library inside the same JVM process.

Advantages:

- Synchronous or asynchronous APIs can be exposed through one library.
- Distribution can be transparent to the host application.
- Fewer processes need to be managed.

Risks:

- A native failure can terminate the JVM.
- Memory management and callbacks between Java and Rust require strict contracts.
- Deployment must handle operating system, architecture, permissions, and temporary locations.

### External sidecar process

The Java 6 application communicates with a Rust process over HTTP, TCP, or another local protocol.

Advantages:

- Fault and dependency isolation.
- The Rust process can evolve more independently from the Java runtime.
- The integration boundary can be a versioned protocol.

Risks:

- The external process lifecycle must be managed.
- Additional latency, local authentication, and communication observability are introduced.
- Deployment includes more components.

### Preliminary criterion

JNI should be studied as the embedded distribution option. The sidecar approach should remain available when the operational risk of loading native code inside the JVM is unacceptable.

## Native distribution pattern

The reference pattern is to package one native library for each target platform and architecture inside the JAR. At runtime, the facade should:

1. Detect the operating system and architecture.
2. Select the corresponding native resource.
3. Extract it to a controlled location.
4. Apply appropriate permissions when necessary.
5. Load it through the selected native mechanism.
6. Report failures with enough context for diagnosis.

Xerial SQLite JDBC is the primary reference for this pattern: its documentation describes a single JAR containing native libraries for multiple operating systems and automatically extracting the appropriate library when the driver is loaded.

## Reference projects

### Native library packaging and loading

- [Xerial SQLite JDBC](https://github.com/xerial/sqlite-jdbc)
- [Snappy Java](https://github.com/xerial/snappy-java)
- [LZ4 Java](https://github.com/lz4/lz4-java)

### Rust, JNI, and FFI

- [JNI-RS](https://github.com/jni-rs/jni-rs)
- [Safer FFI](https://github.com/getditto/safer_ffi)

### Modern messaging

- [RabbitMQ Java Client](https://github.com/rabbitmq/rabbitmq-java-client)
- [Apache Kafka](https://github.com/apache/kafka)
- [NATS Java Client](https://github.com/nats-io/nats.java)

## Study order

1. Analyze the JAR structure and native loader in SQLite JDBC.
2. Compare the strategies used by Snappy Java and LZ4 Java.
3. Study the JNI access model provided by JNI-RS.
4. Evaluate Safer FFI for contracts between Rust and external code.
5. Compare RabbitMQ, Kafka, and NATS by persistence, delivery, ordering, backpressure, security, and operations.
6. ~~Decide whether the gateway should be embedded, external, or support both modes.~~ —
   **decided:** embedded JNI, see [ADR-0001](docs/adr/0001-jni-boundary-over-sidecar.md).

## Open decisions

- ~~Embedded JNI versus an external process.~~ — **closed** by
  [ADR-0001](docs/adr/0001-jni-boundary-over-sidecar.md) (Accepted).
- Public gateway protocol.
- Concurrency model compatible with Java 6 without relying on lambdas or the Streams API.
- Retry, timeout, cancellation, and backpressure policies.
- Certificate, key, and TLS configuration management.
- Supported target platforms and architectures.
- Location, permissions, cleanup, and file locking for extracted native libraries.
- Java API and native contract versioning.
- Diagnostics strategy: logs, metrics, traces, and error codes.

## Document status

This document describes a research direction and a candidate architecture. Recorded Java,
native, and broker probes are listed in [`docs/VERIFICATION.md`](docs/VERIFICATION.md), but they
are not evidence that the integration satisfies the legacy product: the vendor application and
its JMS implementation have never been part of a recorded run.

## Current implementation layout

The Rust workspace keeps provider-neutral package names where unambiguous; the shared core and
JNI packages are `modernlink-core` and `jni-bridge`. `ModernLink` remains the public product name:

```text
crates/core      - shared request, response, TLS metadata, and error types
crates/http      - HTTPS execution
crates/tls       - TLS policy boundary
crates/messaging - provider-neutral message domain and transports
crates/jni       - JNI entry points and native library
```

The Java facade is under `java/src/main/java/com/modernlink`. The native artifact remains `modernlink`.

Two crate names used to collide with well-known ones. Resolved: the packages are
`modernlink-core` and `jni-bridge`, while the folders stay `crates/core` and `crates/jni` and
the native artifact stays `modernlink`. Cargo invocations spell the JNI crate `-p jni-bridge`.
See [`docs/ISSUES.md`](docs/ISSUES.md) I-001 and I-002.

Agent-facing build rules live in [`AGENTS.md`](AGENTS.md); the documentation index is
[`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md), [`docs/ROADMAP.md`](docs/ROADMAP.md), and
[`docs/ISSUES.md`](docs/ISSUES.md).

## Java 6 container

The Java compatibility container is defined in `docker/java6/Dockerfile`. Docker Hub lists the legacy `java:6b38-jdk` tag, but the legacy Java image family is deprecated and may no longer be available from every registry.

With a running Docker daemon, build and run it from the repository root:

```text
docker build -f docker/java6/Dockerfile -t modernlink-java6-https .
docker run --rm modernlink-java6-https
```

The image compiles the Java facade and test sources with `-source 1.6 -target 1.6`, packages the platform-selected native libraries into `/workspace/modernlink.jar`, and reports the container's Java version. The same JAR can be used to exercise the Java-to-Rust HTTPS and messaging paths.
