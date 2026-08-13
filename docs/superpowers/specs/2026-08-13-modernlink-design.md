# ModernLink HTTPS Compatibility Layer Design

## Status

Design specification for review. No Java or Rust implementation is included in this document.

## Goal

Provide a Java 6-compatible HTTPS client facade backed by a modern Rust HTTP/TLS engine, distributed as a JAR with platform-specific native libraries using a Snappy-style extraction and loading model.

The public Java namespace is `com.modernlink`. The native library is `modernlink`.

## Context

The host product is vendor-locked to Java 6 and cannot be migrated to a modern Java runtime. Its existing networking and messaging capabilities are constrained by the APIs and TLS support available in that environment.

ModernLink is an exit layer: legacy Java code calls a stable Java facade, while Rust supplies modern transport and cryptographic capabilities behind a native boundary.

The OpenJDK 7u `com.sun.net.ssl.HttpsURLConnection` source is used as a behavioral reference. It is an abstract HTTPS-specific facade that exposes cipher-suite information, the server certificate chain, hostname-verification policy, and SSL socket-factory configuration. It is not the complete HTTP implementation and should not be copied as the implementation model.

Reference: [OpenJDK HttpsURLConnection.java](https://github.com/openjdk-mirror/jdk7u-jdk/blob/master/src/share/classes/com/sun/net/ssl/HttpsURLConnection.java).

## Non-goals for the first release

- Replacing every Java networking API.
- Reproducing every undocumented behavior of `HttpURLConnection`.
- Implementing RabbitMQ, Kafka, or NATS clients in the initial HTTPS milestone.
- Requiring Java 8 language features or runtime classes.
- Exposing Rust types, lifetimes, or memory ownership rules through the Java API.
- Allowing insecure TLS or hostname-verification bypasses by default.

## Architecture

```text
Legacy Java 6 application
        |
        v
com.modernlink Java facade
        |
        +-- Java 6 API and lifecycle
        +-- native extraction and loading
        +-- JNI declarations and value conversion
        +-- Java exception translation
        |
        v
modernlink native library
        |
        +-- JNI entry points
        +-- opaque native handles
        +-- request execution
        +-- HTTP transport
        +-- modern TLS
```

The Java facade is the compatibility boundary. The Rust implementation is divided internally so that JNI-specific code does not leak into the HTTP and TLS layers.

## Public Java API direction

The first public API should be custom and independent of `HttpURLConnection`:

```text
com.modernlink.LegacyHttpClient
com.modernlink.LegacyHttpRequest
com.modernlink.LegacyHttpResponse
com.modernlink.LegacyTlsInfo
com.modernlink.LegacyHttpException
```

The API must use Java 6-compatible syntax and types. It must not require lambdas, method references, Streams API classes, `CompletableFuture`, or other post-Java-6 runtime features.

An optional compatibility adapter may later expose a `HttpURLConnection`-style surface. That adapter is intentionally deferred because URL-handler registration, connection state, stream behavior, redirects, and exception semantics would substantially expand the initial scope.

## HTTPS behavior

The initial HTTPS behavior should include:

- HTTPS request creation;
- GET and POST;
- request headers;
- request and connection timeouts;
- response status;
- response headers;
- response body access;
- TLS protocol configuration;
- default hostname verification;
- negotiated cipher-suite reporting;
- peer certificate-chain reporting;
- explicit connection and resource lifecycle;
- native errors translated into stable Java exceptions.

Hostname verification must be enabled by default. Any opt-out must be explicit, narrowly scoped, and visible in configuration and diagnostics.

## Native boundary

The JNI surface should be minimal and based on opaque handles rather than exposing Rust structures directly.

Conceptually, the boundary contains operations for:

1. Creating a client or request context.
2. Supplying URL, method, headers, body, and timeout configuration.
3. Executing the request.
4. Reading status, headers, body, and TLS metadata.
5. Releasing native resources.

Java owns Java-visible objects. Rust owns native resources. Every native handle must have one documented release path, and Java cleanup must not depend solely on finalization.

The JNI crate is the only Rust crate permitted to depend on JNI-specific bindings. HTTP and TLS crates must communicate through Rust-native interfaces.

## Rust workspace direction

Use Cargo for the Rust project and keep responsibilities separated:

```text
modernlink-core   - shared Rust domain types and error model
modernlink-http   - HTTP request execution and response handling
modernlink-tls    - TLS configuration, verification, and peer metadata
modernlink-jni    - JNI entry points, handle registry, and Java conversion
```

The produced native artifact is named `modernlink` for every platform, with the platform-specific filename convention applied by the build toolchain.

## JAR and native packaging

The distributable JAR should contain one native artifact for each supported operating-system and architecture tuple:

```text
modernlink.jar
  native/windows-x86_64/modernlink.dll
  native/linux-x86_64/libmodernlink.so
  native/linux-aarch64/libmodernlink.so
  native/macos-x86_64/libmodernlink.dylib
```

The Java loader must:

1. Determine the operating system and architecture.
2. Map that tuple to an embedded resource.
3. Extract the resource to a controlled location.
4. Prevent unsafe collisions between versions or processes.
5. Load the extracted library.
6. Report platform, architecture, resource, and load errors clearly.

The exact supported platform matrix is a later decision, but unsupported combinations must fail explicitly rather than silently falling back to an incompatible binary.

## Error model

The Java facade should expose stable exception categories for:

- invalid request configuration;
- unsupported platform or architecture;
- native library load failure;
- TLS configuration or certificate failure;
- hostname-verification failure;
- connection timeout;
- read timeout;
- protocol or transport failure;
- native resource lifecycle failure.

Native diagnostics may include an internal error code and message, but Java callers must not depend on Rust-specific error type names.

## Compatibility and evolution

The Java API and native ABI must be versioned independently but checked for compatibility at load time. The native layer must expose a version/capability query before request execution.

New capabilities should be additive where possible. Existing request and response semantics must remain stable when the Rust HTTP/TLS implementation evolves.

The public API should avoid exposing implementation-specific names such as `RustHttpsURLConnection`; `modernlink` identifies the product, while Rust remains an implementation detail.

## Security requirements

- Secure TLS configuration is the default.
- Hostname verification is enabled by default.
- Certificate validation failures are surfaced as errors.
- Secrets and private key material must not be written to diagnostic logs.
- Extracted native libraries must use controlled paths and predictable ownership/permissions.
- Native library loading must defend against accidental loading of an unrelated file with the same name.
- Request and response buffers must have explicit size and lifecycle policies.

## Phased delivery

### Phase 1: HTTPS vertical slice

Deliver one real path end to end: Java 6 facade → extracted `modernlink` library → Rust HTTPS request → Java response.

Scope: one target platform, GET, response status/headers/body, TLS verification, and explicit resource cleanup.

### Phase 2: Request features

Add POST bodies, configurable headers, timeouts, redirects, and richer error categories.

### Phase 3: TLS and diagnostics

Add TLS metadata, certificate-chain access, configuration controls, structured diagnostics, and capability reporting.

### Phase 4: Compatibility adapter

Evaluate and, if justified, add a `HttpURLConnection`-style adapter over the custom ModernLink API.

### Phase 5: Messaging extensions

Study and add messaging capabilities only after the HTTPS/TLS foundation has a stable lifecycle and error model.

## Validation boundary

Build and test results are machine observations, not proof that the product satisfies the legacy application's intent. Human validation must eventually exercise the actual Java 6 runtime, target operating systems, native loading behavior, TLS endpoints, failure modes, and deployment constraints.

