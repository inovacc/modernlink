# ModernLink HTTPS Vertical Slice Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build the first real ModernLink path from a Java 6-compatible facade through a Snappy-style native loader and JNI into a Rust HTTPS GET engine.

**Architecture:** Cargo owns a Rust workspace with `core`, `http`, `tls`, and `jni` responsibilities. The Java side owns the Java 6 API, native resource extraction, JNI declarations, and stable exception translation. The first slice supports one target platform, HTTPS GET, status, headers, body, TLS verification, and explicit cleanup.

**Tech Stack:** Rust/Cargo, JNI-RS, Rust HTTP/TLS stack selected through Cargo, Java 6-compatible source, JAR native-resource packaging.

**Spec:** `docs/superpowers/specs/2026-08-13-modernlink-design.md`

## Global Constraints

- Public Java namespace: `com.modernlink`.
- Native library name: `modernlink`.
- Java source and bytecode must remain compatible with Java 6.
- No Java 8 language or runtime APIs.
- Native resources are selected by operating system and architecture.
- Hostname verification and certificate validation are enabled by default.
- Rust JNI-specific code stays isolated from HTTP and TLS code.
- Do not claim runtime success from automated build or test results; report machine observations separately from human validation.

---

### Task 1: Establish the Cargo workspace

**Files:**
- Create: `Cargo.toml`
- Create: `crates/modernlink-core/Cargo.toml`
- Create: `crates/modernlink-core/src/lib.rs`
- Create: `crates/modernlink-http/Cargo.toml`
- Create: `crates/modernlink-http/src/lib.rs`
- Create: `crates/modernlink-tls/Cargo.toml`
- Create: `crates/modernlink-tls/src/lib.rs`
- Create: `crates/modernlink-jni/Cargo.toml`
- Create: `crates/modernlink-jni/src/lib.rs`

**Interfaces:**
- Produces a Cargo workspace whose crates compile independently and whose JNI crate emits the `modernlink` native library.

- [ ] Run `cargo new` for each crate and configure the workspace.
- [ ] Add the minimum shared error and result types in `core`.
- [ ] Add crate dependency declarations without implementing request behavior.
- [ ] Run `cargo check --workspace` and record only the raw command result.

### Task 2: Define the Java facade contract

**Files:**
- Create: `java/src/main/java/com/modernlink/LegacyHttpClient.java`
- Create: `java/src/main/java/com/modernlink/LegacyHttpRequest.java`
- Create: `java/src/main/java/com/modernlink/LegacyHttpResponse.java`
- Create: `java/src/main/java/com/modernlink/LegacyTlsInfo.java`
- Create: `java/src/main/java/com/modernlink/LegacyHttpException.java`
- Create: `java/src/test/java/com/modernlink/LegacyHttpRequestTest.java`

**Interfaces:**
- `LegacyHttpRequest` accepts a URL and HTTP method and exposes Java 6-compatible headers and timeout configuration.
- `LegacyHttpResponse` exposes status, headers, body bytes, and TLS metadata.
- `LegacyHttpClient` executes a request and owns the native client handle.

- [ ] Write a failing test for request URL/method validation.
- [ ] Run the focused test and observe the expected missing-class or missing-behavior failure.
- [ ] Implement the smallest Java 6-compatible request value object and exception classes.
- [ ] Run the focused test and record the raw result.

### Task 3: Implement Snappy-style native loading

**Files:**
- Create: `java/src/main/java/com/modernlink/NativeLoader.java`
- Create: `java/src/test/java/com/modernlink/NativeLoaderTest.java`
- Create: `java/src/main/resources/native/windows-x86_64/modernlink.dll.placeholder`

**Interfaces:**
- `NativeLoader.load()` selects a resource using `os.name` and `os.arch`, extracts it to a versioned temporary path, and calls `System.load`.

- [ ] Write a failing test for unsupported platform reporting.
- [ ] Run the focused test and observe the expected failure.
- [ ] Implement platform mapping and explicit unsupported-platform errors.
- [ ] Add extraction collision and cleanup rules without loading a placeholder as a native binary.
- [ ] Run the focused test and record the raw result.

### Task 4: Add JNI handles and a Rust request boundary

**Files:**
- Modify: `crates/modernlink-core/src/lib.rs`
- Modify: `crates/modernlink-jni/src/lib.rs`
- Modify: `java/src/main/java/com/modernlink/LegacyHttpClient.java`
- Create: `java/src/test/java/com/modernlink/LegacyHttpClientContractTest.java`

**Interfaces:**
- Java client methods map to opaque `jlong` handles and explicit release operations.
- Rust exposes a capability/version query before request execution.

- [ ] Write a failing contract test for client creation and release semantics.
- [ ] Run it and observe the expected missing-native-entry-point failure.
- [ ] Implement the minimum handle registry and JNI lifecycle methods.
- [ ] Translate native failures into `LegacyHttpException` without exposing Rust error type names.
- [ ] Run the focused test and record the raw result.

### Task 5: Implement the HTTPS GET vertical slice

**Files:**
- Modify: `crates/modernlink-http/src/lib.rs`
- Modify: `crates/modernlink-tls/src/lib.rs`
- Modify: `crates/modernlink-jni/src/lib.rs`
- Modify: `java/src/main/java/com/modernlink/LegacyHttpClient.java`
- Modify: `java/src/main/java/com/modernlink/LegacyHttpResponse.java`
- Create: `java/src/test/java/com/modernlink/LiveHttpsGetTest.java`

**Interfaces:**
- `LegacyHttpClient.execute(LegacyHttpRequest)` returns a response with status, headers, body, and TLS metadata.

- [ ] Write an integration test against a human-approved HTTPS endpoint.
- [ ] Run it before implementation and record the expected failure.
- [ ] Implement a Rust HTTPS GET using Cargo-managed HTTP/TLS dependencies.
- [ ] Connect the JNI response conversion to Java response objects.
- [ ] Run the integration command and report machine output separately from human runtime judgment.

### Task 6: Package the first native artifact

**Files:**
- Create: `java/pom.xml`
- Create: `java/README.md`
- Create: `scripts/package-native.ps1`
- Modify: `README.md`

**Interfaces:**
- The packaging command places the target-platform `modernlink` artifact under the JAR resource path and builds a Java 6-compatible artifact.

- [ ] Define the packaging command and target-platform resource layout.
- [ ] Build the JAR without adding release or publishing machinery.
- [ ] Inspect the JAR contents and report the raw listing.
