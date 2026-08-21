# ADR-0001: Cross the Java 6 boundary with embedded JNI, keeping a sidecar in reserve

- **Status:** Accepted (provisional — the README still records this as a design hypothesis)
- **Date:** 2026-08-14
- **Deciders:** Dyam Marcano

## Context

A vendor-locked product must remain on Java 6 while gaining modern TLS, HTTPS, and messaging.
Java 6 lacks lambdas, method references, and the Streams API; current library versions largely
do not support it; and its cryptographic and TLS capabilities are insufficient for present
requirements. Migrating the host product is not an option.

Two integration shapes were considered:

**Embedded JNI** — the JAR loads a Rust library inside the same JVM process.
Advantages: one library can expose synchronous or asynchronous APIs, distribution is
transparent to the host application, and fewer processes need managing.
Risks: a native failure can terminate the JVM; memory management and callbacks across the
boundary need strict contracts; deployment must handle OS, architecture, permissions, and
temporary locations.

**External sidecar** — the Java 6 application talks to a Rust process over a local protocol.
Advantages: fault and dependency isolation, independent evolution, a versioned protocol
boundary.
Risks: process lifecycle management, added latency, local authentication, and more deployment
components.

## Decision

Build the embedded JNI path as the primary distribution option.

- The Java 6 facade (`java/src/main/java/com/modernlink`) exposes a small, stable API and
  contains no modern syntax; it compiles with `-source 1.6 -target 1.6`.
- All modern capability lives in Rust behind `crates/jni`, which exports the `Java_*` entry
  points and builds `libmodernlink`.
- Native libraries for each platform ship inside a single JAR under `native/<os>-<arch>/`,
  following the Xerial SQLite JDBC packaging pattern. `NativeLoader` detects the platform,
  hashes the embedded bytes with SHA-256, extracts to a content-addressed path, and loads it.
- The sidecar option is **not discarded**. It remains the fallback for deployments where
  loading native code inside the JVM is an unacceptable operational risk.

## Consequences

**Positive**
- The legacy class path needs no modern Java dependencies; broker clients and TLS live in Rust.
- One artifact to distribute; the host application's build is untouched.
- TLS is terminated and verified in Rust, which lets the facade present a single, honest
  security contract rather than a partially-honoured Java one.

**Negative**
- A panic or memory error in Rust can take down the host JVM. This is the central accepted risk.
- The JNI surface is wide and hand-written — 28 `Java_*` functions across HTTP, messaging, and
  utilities — and every one is an FFI boundary that must stay guarded and in sync with its Java
  caller.
- Packaging must cover every supported OS/architecture pair; a missing native resource is a
  runtime failure, not a build failure.
- Java-side correctness is invisible to `cargo test`; only the Docker build compiles the facade.

**Follow-on consequences that became their own problems**
- Naming `crates/jni` after the domain collides with the external `jni` crate it depends on,
  forcing `-p jni-bridge` everywhere (ISSUES I-001). Naming `crates/core` shadows Rust's
  built-in `core` (I-002).
- Because the modern half is Rust, the Java 6 build needs Docker, and the only available base
  image is deprecated (I-004).

## Status of validation

None. This decision has not been exercised against the real Java 6 host product, the target
platforms, or real services. The architecture is implemented, not proven.
