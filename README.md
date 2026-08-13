# Legacy Exit Gateway SDK

## Purpose

This project studies a compatibility layer for a legacy product that must remain running on Java 6 while communicating with modern services and protocols.

The proposed layer should add current capabilities—such as messaging, TLS, and network clients—without requiring a migration of the main product.

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
        +--> RabbitMQ / Kafka / NATS
```

The JAR should expose a small, stable API to Java 6 code. Modern implementation details are encapsulated in Rust, and platform-specific native binaries are distributed alongside the JAR.

This architecture is still a design hypothesis. The final integration approach—embedded JNI or an external sidecar process—has not yet been decided.

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
6. Decide whether the gateway should be embedded, external, or support both modes.

## Open decisions

- Embedded JNI versus an external process.
- Public gateway protocol.
- Concurrency model compatible with Java 6 without relying on lambdas or the Streams API.
- Retry, timeout, cancellation, and backpressure policies.
- Certificate, key, and TLS configuration management.
- Supported target platforms and architectures.
- Location, permissions, cleanup, and file locking for extracted native libraries.
- Java API and native contract versioning.
- Diagnostics strategy: logs, metrics, traces, and error codes.

## Document status

This document describes a research direction and a candidate architecture. It is not yet an implementation specification, and it is not evidence that the integration works with the legacy product. Validation must later be performed with the Java 6 runtime, target platforms, and real services.

## Current implementation layout

The Rust workspace uses unprefixed internal crate names while retaining `ModernLink` as the public product name:

```text
crates/core  - shared request, response, TLS metadata, and error types
crates/http  - HTTPS execution
crates/tls   - TLS policy boundary
crates/jni   - JNI entry points and native library
```

The Java facade is under `java/src/main/java/com/modernlink`. The native artifact remains `modernlink`.

## Java 6 container

The Java compatibility container is defined in `docker/java6/Dockerfile`. Docker Hub lists the legacy `java:6b38-jdk` tag, but the legacy Java image family is deprecated and may no longer be available from every registry. citeturn3search6turn3search2

With a running Docker daemon, build and run it from the repository root:

```text
docker build -f docker/java6/Dockerfile -t modernlink-java6 .
docker run --rm modernlink-java6
```

The image compiles the Java facade and test sources with `-source 1.6 -target 1.6`, then reports the container's Java version. Native loading and the Java-to-Rust HTTPS path require a later image step that includes the packaged `modernlink` library.
