# AGENTS.md — ModernLink
<!-- rev:014 (RFC 3339) 2026-08-20T00:00:00Z -->

Canonical cross-tool agent instructions for the ModernLink repo (read by Claude Code,
Codex, Cursor, Gemini, etc. — Claude Code imports this from `CLAUDE.md`). Must-know
rules live here; deeper reference is in `docs/*`.

## Project overview

ModernLink is a **compatibility layer that lets a vendor-locked Java 6 application talk to
modern protocols and brokers without migrating the host product**. A Java 6-compatible JAR
facade (`java/src/main/java/com/modernlink`) calls across a stable JNI boundary into a Rust
native library, which owns modern TLS, HTTPS, and messaging.

```text
Java 6 app -> Java 6 JAR facade -> JNI boundary -> Rust native lib -> TLS / HTTPS / NATS·Kafka·Pulsar·RabbitMQ
```

The Rust workspace uses **unprefixed crate names** while `ModernLink` stays the product name:

Six crates — `core` (shared types), `http` (HTTPS), `tls` (policy boundary), `messaging`
(the five provider transports plus in-process `LEGACY_JMS`), `jni` (the entry points, builds
the `modernlink` native library), and `hacks/messaging-demo` (contract fixtures). Full source
layout with per-crate detail: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

The distributable is one JAR with per-platform native resources embedded
(`native/linux-x86_64`, `native/linux-aarch64`, `native/windows-x86_64`).

## Hard rules (NON-NEGOTIABLE)

- **Java 6 compatibility is mandatory at the application boundary.** Everything under
  `java/src` compiles with `-source 1.6 -target 1.6`. **No lambdas, no method references, no
  Streams API, no diamond operator, no try-with-resources.** If you need a modern capability,
  it belongs behind the JNI boundary in Rust, not in the facade.
- **No provider-specific dependency may leak into the legacy class path** unless deployment
  chooses it explicitly. Broker clients live in Rust.
- **Fail closed on unsupported guarantees.** Delivery semantics are part of the contract, not
  an implementation detail. A capability gap must be reported explicitly — never silently
  degraded. Transparent mode must not quietly alter delivery guarantees.
- **Never put credentials, payloads, or message bodies in JMX attributes or logs.**
- **Use `-p jni-bridge`** for the JNI crate. It used to be named `jni`, which collided with the
  external `jni` crate it depends on and forced `-p jni@0.1.0` everywhere; SC-05 renamed the
  package (the folder is still `crates/jni`, and `[lib] name` is still `modernlink`). `crates/core`
  is likewise the package `modernlink-core`, so dependents write `use modernlink_core::{...}`
  rather than something that reads as the standard library. See [docs/ISSUES.md](docs/ISSUES.md)
  I-001 and I-002, both resolved.
- **Do not publish to crates.io.** All six manifests now carry `publish = false`
  (`crates/{core,http,tls,jni,messaging}/Cargo.toml:6`, `hacks/messaging-demo/Cargo.toml:6`,
  landed in `315fe87`), so publication is blocked mechanically rather than by convention.
  Do not run `cargo publish` regardless.

## Build & test commands

No task runner is wired up (no Taskfile/Makefile/justfile). These are the real commands, and
they mirror `.github/workflows/test.yml`.

| Purpose | Command |
|---|---|
| Test the Rust workspace (no broker client compiled) | `cargo test --workspace` |
| Test with every provider transport | `cargo test --workspace --all-features` |
| Check the JNI crate | `cargo check -p jni-bridge` |
| Format | `cargo fmt --all -- --check` |
| Lint | `cargo clippy --workspace --all-targets -- -D warnings` |
| Coverage | `cargo llvm-cov --workspace --all-features --summary-only` |
| Build the Java 6 JAR | `docker build -f docker/java6/Dockerfile -t modernlink-java6 .` |
| Run a packaged Java test | `docker run --rm modernlink-java6 sh -c "java -cp /workspace/modernlink.jar com.modernlink.LegacyHttpsTest"` |

**Provider transports are feature-gated as of SC-07.** `crates/messaging` declares
`default = []`, and `crates/jni` forwards the selection, so a plain `cargo test --workspace`
compiles **no** broker client — no cmake, no librdkafka, no protoc. Build the distributable and
anything that must talk to a broker with `--features all-providers`; `docker/java6/Dockerfile`
already does. **Asking for a provider that was not compiled in fails closed** with an error
naming the missing cargo feature — it is never rerouted to another transport.

**`cargo test --workspace`, `cargo check -p jni-bridge`, `cargo fmt --all -- --check` and
`cargo clippy --workspace --all-targets -- -D warnings` are all CI-gated** for Rust as of
`dd080b2` (`.github/workflows/test.yml`), and `--all-features` variants of test and clippy run
alongside them as of SC-07. Coverage is still not gated. The Java side has **no
Maven or Gradle build** — `javac` runs inside `docker/java6/Dockerfile`, which is the only
supported way to compile and package the facade.

**CI runs on every push to `main` and on every PR, and it now reaches real brokers.** Six jobs:
the Rust workspace, the Java 6 JAR on JVM 1.6.0_38, NATS/JetStream/RabbitMQ against live
brokers, Kafka and Pulsar against live brokers, the linux-aarch64 native load, and a dependency
audit.

**Two of those greens still need reading precisely.** `cargo test --workspace` skips the five
`#[ignore]`d broker tests, so the *local* command asserts nothing about a broker — only the
dedicated CI jobs do. And the **dependency-audit job is `continue-on-error`**, so it reports
green while `cargo audit` exits 1 on four open advisories (B-009); that flag comes off when
B-009 closes. What has and has not been run is tracked in the "Verification reach" section of
[docs/BUGS.md](docs/BUGS.md) — read it before describing this project as tested.

The native libraries are cross-compiled with `cargo-zigbuild` for three targets inside that same
Dockerfile; the `kafka` and `pulsar` features need `cmake`, `libcurl` and `protobuf-compiler`
present. The default build needs none of them (SC-07).

## Code style

- **Rust** — `rustfmt`; keep provider transports behind the uniform transport boundary in
  `crates/messaging` so the JNI surface stays provider-neutral.
- **Java** — Java 6 syntax only (see Hard rules). Explicit types, anonymous inner classes
  instead of lambdas, `StringBuilder` over streams.
- Facade classes are prefixed `Modern*` (new surface) or `Legacy*` (the compatibility API the
  host product already calls).

## Testing

- Rust tests must pass before merge: `cargo test --workspace`.
- Java tests are **standalone `main`-style classes** run from the packaged JAR, not JUnit —
  see `java/src/test/java/com/modernlink/` (11 classes) and
  `java/src/test/java/com/modernlink/messaging/` (4 classes), **15 in total**. As of `dd080b2`
  the workflow enumerates and runs all of them; add new ones the same way and wire them into
  `.github/workflows/test.yml`, because a class the workflow does not name never runs.
- **A local `javac` catches most Java errors — use it before pushing.** Docker is needed for
  the Java 6 *language level* and the runtime, not for type checking. `javac -source 8` over
  `java/src/main/java` then `java/src/test/java` catches unreported checked exceptions,
  missing methods and type errors in seconds. A `ModernPayload` change reached `main` without
  it and broke the CI JAR build with five `unreported exception` errors that this catches
  immediately. Recipe in [docs/CONTRIBUTORS.md](docs/CONTRIBUTORS.md).
- The fixtures under `hacks/` are deterministic contract probes. They are **not** evidence of
  broker-backed behavior. The only broker-backed evidence is
  `crates/messaging/tests/broker_backed.rs`, it covers NATS, JetStream and RabbitMQ only, it is
  `#[ignore]`d so no CI run executes it, and it exercises one happy-path round trip.

## Security

- TLS terminates and verifies in Rust. Custom Java `HostnameVerifier` / `SSLSocketFactory`
  instances are **deliberately rejected** rather than ignored — accepting and discarding them
  would create a misleading security contract.
- TLS floor is 1.2; callers may select 1.2 or 1.3. Unsupported values are rejected before the
  native request starts.
- Native extraction hashes embedded bytes (SHA-256) and renames into a content-addressed path;
  temp files are cleaned up when extraction or loading fails.
- **Panics are contained at the JNI boundary**, not permitted. All 28 `Java_*` entry points run
  inside `jni_guard`; a panic becomes a reported error instead of undefined behaviour in the
  host JVM. `crates/jni` contains **zero `unsafe` blocks** and a test enforces that.
- **Credentials are scrubbed from transport errors** before they can reach a Java exception the
  host logs — broker URLs carry them inline. Build errors with `transport_error`, never from a
  raw provider string; a test enforces that too.
- **Broker connections are NOT TLS-terminated** through `crates/tls` — only the HTTPS path is.
  Kafka cannot do TLS at all in this build and refuses a TLS endpoint rather than connecting in
  plaintext. See [docs/providers.md](docs/providers.md) before assuming a broker link is
  encrypted.
- Never commit credentials or broker endpoints.

## PR / commit conventions

- Conventional commits (`feat:`, `fix:`, `docs:`, `build:`, `style:`, `merge:`).
- No AI attribution in commit messages.
- Run `cargo test --workspace` and `cargo check -p jni-bridge` before proposing a merge, and
  report the results as **facts, not a verdict** — a green gate is a machine result, never
  proof that the Java 6 integration works. Runtime validation against the legacy runtime and
  real brokers has not been done; only the maintainer decides whether something is done.

## Reference docs

- [docs/BACKLOG.md](docs/BACKLOG.md) — messaging compatibility backlog, operating modes, M1/M2 items
- [docs/providers.md](docs/providers.md) — **per-provider delivery guarantees** (MSG-04/DOC-03); read this before selecting a provider
- [docs/jms-compatibility.md](docs/jms-compatibility.md) — the JMS contract boundary
- [docs/jmx-management.md](docs/jmx-management.md) — JMX management model
- [docs/routing-semantics.md](docs/routing-semantics.md) — routing / redirect semantics
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) — component + sequence diagrams
- [docs/ISSUES.md](docs/ISSUES.md) — known limitations, incl. the crate-name collisions
