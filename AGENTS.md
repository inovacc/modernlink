# AGENTS.md — ModernLink
<!-- rev:002 (RFC 3339) 2026-08-14T02:22:19Z -->

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
- **`cargo check -p jni` is ambiguous — always use `-p jni@0.1.0`.** The workspace crate `jni`
  collides with the external `jni` crate it depends on. The same applies to `core`, which
  shadows Rust's built-in `core`. See [docs/ISSUES.md](docs/ISSUES.md).
- **Do not publish to crates.io.** No manifest currently carries `publish = false` — treat
  every crate as non-publishable regardless, and do not run `cargo publish`.

## Build & test commands

No task runner is wired up (no Taskfile/Makefile/justfile). These are the real commands, and
they mirror `.github/workflows/test.yml`.

| Purpose | Command |
|---|---|
| Test the Rust workspace | `cargo test --workspace` |
| Check the JNI crate | `cargo check -p jni@0.1.0` |
| Format | `cargo fmt --all -- --check` |
| Lint | `cargo clippy --workspace --all-targets -- -D warnings` |
| Coverage | `cargo llvm-cov --workspace --summary-only` |
| Build the Java 6 JAR | `docker build -f docker/java6/Dockerfile -t modernlink-java6 .` |
| Run a packaged Java test | `docker run --rm modernlink-java6 sh -c "java -cp /workspace/modernlink.jar com.modernlink.LegacyHttpsTest"` |

**Only `cargo test --workspace` and `cargo check -p jni@0.1.0` are CI-gated** for Rust; `fmt`,
`clippy`, and coverage are available but not enforced by a workflow. The Java side has **no
Maven or Gradle build** — `javac` runs inside `docker/java6/Dockerfile`, which is the only
supported way to compile and package the facade.

> **The Rust CI gate is currently RED and has been for five consecutive pushes.** The
> `Rust workspace` job dies building `rdkafka-sys` on a missing `curl/curl.h` and never reaches
> the tests — see [docs/BUGS.md](docs/BUGS.md) B-001. Do not cite CI as evidence that
> `cargo test --workspace` passes until that is fixed; run it locally and report the real
> result instead. The `Java 6 JAR integration` job in the same workflow does pass.

The native libraries are cross-compiled with `cargo-zigbuild` for three targets inside that
same Dockerfile; `crates/messaging` needs `cmake`, `libcurl`, and `protobuf-compiler` present
(rdkafka and pulsar build native code), so a bare host may not build the workspace — and
neither does the CI runner today, which is exactly B-001.

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
  see `java/src/test/java/com/modernlink/*Test.java`. CI runs three of them; add new ones the
  same way and wire them into `.github/workflows/test.yml`.
- The fixtures under `hacks/` are deterministic contract probes. They are **not** evidence of
  broker-backed behavior — real broker integration remains unproven.

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
- Run `cargo test --workspace` and `cargo check -p jni@0.1.0` before proposing a merge, and
  report the results as **facts, not a verdict** — a green gate is a machine result, never
  proof that the Java 6 integration works. Runtime validation against the legacy runtime and
  real brokers has not been done; only the maintainer decides whether something is done.

## Reference docs

- [docs/BACKLOG.md](docs/BACKLOG.md) — messaging compatibility backlog, operating modes, M1/M2 items
- [docs/jms-compatibility.md](docs/jms-compatibility.md) — the JMS contract boundary
- [docs/jmx-management.md](docs/jmx-management.md) — JMX management model
- [docs/routing-semantics.md](docs/routing-semantics.md) — routing / redirect semantics
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) — component + sequence diagrams
- [docs/ISSUES.md](docs/ISSUES.md) — known limitations, incl. the crate-name collisions
