# AGENTS.md — ModernLink
<!-- rev:022 (RFC 3339) 2026-08-22T03:15:42Z -->

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

The Rust workspace keeps provider-neutral package names where they are unambiguous, while the
shared core and JNI packages are explicitly namespaced as `modernlink-core` and `jni-bridge`.
`ModernLink` remains the product name:

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
  degraded. Transparent mode must not quietly alter delivery guarantees. This is the required
  contract, not a description of complete enforcement: [docs/BUGS.md](docs/BUGS.md) B-003 is
  the known current deviation because publish paths do not yet enforce requested delivery mode.
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
| Coverage (90% production lines in `core`/`http`/`messaging`/`tls`; full report also includes JNI and demos) | `bash scripts/run_rust_coverage.sh` |
| Build the Java 6 JAR | `docker build -f docker/java6/Dockerfile -t modernlink-java6-https .` |
| Run a packaged Java test | `docker run --rm modernlink-java6-https sh -c "java -cp /workspace/modernlink.jar com.modernlink.LegacyHttpsTest"` |

**Provider transports are feature-gated as of SC-07.** `crates/messaging` declares
`default = []`, and `crates/jni` forwards the selection, so a plain `cargo test --workspace`
compiles **no** broker client — no cmake, no librdkafka, no protoc. Build the distributable and
anything that must talk to a broker with `--features all-providers`; `docker/java6/Dockerfile`
already does. **Asking for a provider that was not compiled in fails closed** with an error
naming the missing cargo feature — it is never rerouted to another transport.

The workflow declares eight jobs in stages: Rust checks, Java 6 JAR, Rust behavior-crate coverage,
the two broker-backed groups plus linux-aarch64 and dependency audit, then release readiness.
Rust is the root; Java waits for Rust, coverage waits for Java, the remaining checks wait for
coverage, and release readiness requires every preceding result to be `success`. The five broker
tests are `#[ignore]`d and run only by the dedicated jobs; dependency advisories are blocking for
release readiness. The dependency set recorded by B-009 is addressed by the current broker-client
upgrades; run [32534452508](https://github.com/inovacc/modernlink/actions/runs/32534452508) recorded
the staged post-change machine results. The published-JAR Docker path is recorded in
[docs/VERIFICATION.md](docs/VERIFICATION.md). **Read
[docs/VERIFICATION.md](docs/VERIFICATION.md)
before citing any test, CI, Java, native, or broker result.** It separates recorded command facts
from runtime behavior that remains unproven.

The Java side has no Maven or Gradle build; the Dockerfile is the supported Java 6 compiler and
packager. It cross-compiles three native targets; Kafka/Pulsar builds additionally need `cmake`,
`libcurl`, and `protobuf-compiler`.

## Code style

- **Rust** — `rustfmt`; keep provider transports behind the uniform transport boundary in
  `crates/messaging` so the JNI surface stays provider-neutral.
- **Java** — Java 6 syntax only (see Hard rules). Explicit types, anonymous inner classes
  instead of lambdas, `StringBuilder` over streams.
- Facade classes are prefixed `Modern*` (new surface) or `Legacy*` (the compatibility API the
  host product already calls).

## Testing

- Rust tests must pass before merge: `cargo test --workspace`.
- Java tests are standalone `main`-style classes, not JUnit. The workflow discovers and runs
  every no-argument `*Test.class`; parameterized broker probes are excluded from that loop and
  invoked explicitly with their provider endpoint.
- **A local `javac` catches most Java errors — use it before pushing.** Docker is needed for
  the Java 6 *language level* and the runtime, not for type checking. `javac -source 8` over
  `java/src/main/java` then `java/src/test/java` catches unreported checked exceptions,
  missing methods and type errors in seconds. A `ModernPayload` change reached `main` without
  it and broke the CI JAR build with five `unreported exception` errors that this catches
  immediately. Recipe in [docs/CONTRIBUTORS.md](docs/CONTRIBUTORS.md).
- The fixtures under `hacks/` are deterministic contract probes, not broker evidence. Dedicated
  broker tests and their exact recorded reach are listed in [docs/VERIFICATION.md](docs/VERIFICATION.md).

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
  report the results as **facts, not a verdict**. Prior Java/broker runs are revision-scoped in
  [docs/VERIFICATION.md](docs/VERIFICATION.md); the vendor host remains unexercised. Only the
  maintainer decides whether something is done.

## Reference docs

- [docs/BACKLOG.md](docs/BACKLOG.md) — messaging compatibility backlog, operating modes, M1/M2 items
- [docs/providers.md](docs/providers.md) — **per-provider delivery guarantees** (MSG-04/DOC-03); read this before selecting a provider
- [docs/jms-compatibility.md](docs/jms-compatibility.md) — the JMS contract boundary
- [docs/jmx-management.md](docs/jmx-management.md) — JMX management model
- [docs/routing-semantics.md](docs/routing-semantics.md) — routing / redirect semantics
- [docs/VERIFICATION.md](docs/VERIFICATION.md) — exact command/runtime reach and unproven paths
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) — component + sequence diagrams
- [docs/ISSUES.md](docs/ISSUES.md) — known limitations, incl. the crate-name collisions
