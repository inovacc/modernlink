# AGENTS.md — ModernLink
<!-- rev:010 (RFC 3339) 2026-08-19T00:00:00Z -->

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

| Crate | Owns |
|---|---|
| `crates/core` | shared request/response, TLS metadata, error types |
| `crates/http` | HTTPS execution (hyper) |
| `crates/tls` | TLS policy boundary (rustls) |
| `crates/messaging` | provider transports: NATS, JetStream, Kafka, Pulsar, RabbitMQ, in-process `LEGACY_JMS` |
| `crates/jni` | JNI entry points; builds the `modernlink` native library |
| `hacks/messaging-demo` | executable cross-application contract fixtures |

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

**All four gates passed in CI** on run
[31782837766](https://github.com/inovacc/modernlink/actions/runs/31782837766) (2026-08-14), which
ran against `d2479bd` — the current tip of `main`. The earlier run
[31781200582](https://github.com/inovacc/modernlink/actions/runs/31781200582) the same day
resolved [docs/BUGS.md](docs/BUGS.md) B-001: the Rust job now reaches its test step instead of
dying in the `rdkafka-sys` build. Both jobs in the workflow succeed.

**Read that green gate precisely — it is narrower than it looks.** `cargo test --workspace`
runs 20 tests and **skips 3 `#[ignore]`d ones**, and those three are exactly the broker-backed
tests (`crates/messaging/tests/broker_backed.rs:117,132,149`). A green CI run therefore asserts
**nothing** about any real broker. The only broker evidence that exists is a hand-run against
local containers on one machine, for NATS, JetStream and RabbitMQ only — see the "Verification
reach" section of [docs/BUGS.md](docs/BUGS.md).

The native libraries are cross-compiled with `cargo-zigbuild` for three targets inside that
same Dockerfile; `crates/messaging` needs `cmake`, `libcurl`, and `protobuf-compiler` present
(rdkafka and pulsar build native code), so a bare host may not build the workspace. The CI job
installs those packages as of `dd080b2`.

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
