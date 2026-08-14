# Known Issues and Limitations
<!-- rev:003 (RFC 3339) 2026-08-14T07:27:46Z -->

Constraints accepted on purpose or imposed by the platform. Defects that should be fixed live
in [BUGS.md](BUGS.md); future work lives in [BACKLOG.md](BACKLOG.md).

## Crate naming

### I-001 — the `jni` workspace crate shadows its own dependency

`crates/jni` is named `jni` and depends on the external crate `jni = "0.21"`
(`crates/jni/Cargo.toml:1-12`). `cargo check -p jni` is therefore ambiguous, and every
invocation must disambiguate by version — `.github/workflows/test.yml:19` and
`docker/java6/Dockerfile:14-16` both spell it `-p jni@0.1.0`.

**Workaround:** always use `-p jni@0.1.0`.
**Status:** accepted. Renaming the workspace crate (e.g. to `jni-bridge`) would remove the
ambiguity permanently.

### I-002 — the `core` crate shadows Rust's built-in `core`

`crates/core` is named `core`, which is a Rust built-in crate name. Dependent crates then
write `use core::{Error, Request, Response, TlsInfo};` (`crates/http/src/lib.rs:2`), which
reads as the standard library but resolves to the workspace crate.

**Status:** accepted for now, and it currently compiles — but this is the one case the standing
crate-naming rule carves out: a library crate whose bare name would shadow a Rust built-in
(`core`, `std`, `alloc`, `test`, `proc_macro`) should carry a distinguishing **package** name
while keeping the short folder path (`crates/core` with `name = "modernlink-core"`). The repo
currently has neither, so it does not yet satisfy the exception it relies on. Tracked in
[BACKLOG.md](BACKLOG.md).

## Packaging and platform

### I-003 — no Maven or Gradle build for the Java facade

`java/src` has no `pom.xml`, `build.gradle`, or wrapper. Compilation happens only inside
`docker/java6/Dockerfile` via `find ... | xargs javac -source 1.6 -target 1.6`. There is no
way to build or test the Java side without Docker, and no IDE project model.

**Status:** deliberate for now — it guarantees the Java 6 compiler is the one that judges the
facade. It also means Java changes are invisible to `cargo test`.

### I-004 — the `java:6b38-jdk` base image is deprecated

`docker/java6/Dockerfile:18` pulls `java:6b38-jdk`. The legacy Java image family is deprecated
and may no longer be served by every registry, which makes the JAR build environment-dependent.

**Status:** unresolved. No vendored or self-built Java 6 base image exists.

### I-005 — the workspace may not build on a bare host

`crates/messaging` pulls `rdkafka` with `cmake-build` and `pulsar`, which need `cmake`,
`libcurl`, and `protobuf-compiler`. The Dockerfile installs them
(`docker/java6/Dockerfile:7-11`); a developer machine without them will fail to build the
workspace. A recorded run failed on missing `protoc` with exit 101.

**This limitation is currently also breaking CI**, which is a defect rather than an accepted
constraint — the `Rust workspace` job has no such install step and dies on a missing
`curl/curl.h`. Tracked as [B-001](BUGS.md). An earlier revision of this file claimed the
workspace builds "even though CI is green"; that was wrong, and the claim is withdrawn.

**Workaround:** build through the Dockerfile, or install cmake + libcurl + protoc locally.

## Java 6 language boundary

### I-006 — no lambdas, streams, method references, or try-with-resources

The facade compiles with `-source 1.6 -target 1.6`. Modern Java syntax fails the build, and
`cargo test` will not catch it — only the Docker build does. Anonymous inner classes and
explicit loops are required throughout `java/src`.

**Status:** permanent. This is the reason the project exists.

### I-007 — JSON decode returns normalized text, not an object model

`ModernJson.decode(String)` returns normalized JSON text because Java 6 has no standard JSON
object model and adding one would put a modern dependency on the legacy class path.

## Security contract

### I-008 — custom `HostnameVerifier` / `SSLSocketFactory` are rejected, not honoured

`ModernHttpsURLConnection` explicitly rejects them. TLS is terminated and verified in Rust, so
accepting a Java verifier and then ignoring it would create a misleading security contract.

**Status:** deliberate. Callers needing custom verification must change the Rust TLS policy.

## Messaging scope

### I-009 — `LEGACY_JMS` is an in-process transport, not a vendor JMS bridge

The `LEGACY_JMS` provider is backed by `InMemoryTransport` for transparent-mode contract
fixtures. JNDI, transactions, selectors, rollback/redelivery, and dead-letter behavior are
**not** implemented. The name is about the contract shape, not broker interoperability.

### I-010 — broker-backed behavior is largely unproven at runtime (narrowed 2026-08-14)

Kafka, Pulsar, NATS, JetStream, and RabbitMQ transports exist in `crates/messaging` and are
selectable through the JNI provider surface.

**What is now proven:** a single send → receive → acknowledge round trip against **live NATS
core, NATS JetStream and RabbitMQ**, asserting that message id, destination, payload, trace
context and delivery state survive the broker
(`crates/messaging/tests/broker_backed.rs`; all three passed 2026-08-14). This is the first
broker-backed evidence the project has ever had.

**What is still a source-level claim:**
- **Kafka and Pulsar have no broker-backed test at all.**
- Durability, reconnect, concurrency, ordering, failure and redelivery semantics are
  unexercised for **every** provider, including the three above. One happy-path round trip is
  not delivery semantics.
- The tests are `#[ignore]`d so a normal `cargo test` does not run them, and they require
  brokers on `127.0.0.1:4222` / `:5672`. They are deliberately not self-skipping: with no
  broker they fail loudly rather than reporting a hollow pass.

The fixtures under `hacks/` remain deterministic contract probes, not integration tests.

### I-011 — no JMS version or vendor has been identified

The exact JMS version and vendor implementation used by the locked product is still an open
decision, so the compatibility surface cannot yet be pinned. See BACKLOG "Open decisions".

## Supply chain

### I-012 — ~~no crate declares `publish = false`~~ **RESOLVED `315fe87`**

All six manifests now set `publish = false` (`crates/{core,http,tls,jni,messaging}/Cargo.toml:6`,
`hacks/messaging-demo/Cargo.toml:6`), so publication is blocked mechanically rather than by
convention. Retained here for history; the original text follows.

~~None of the six manifests sets `publish = false`, so nothing mechanically
prevents a `cargo publish`. Given the crate names are `core`, `http`, `tls`, and `jni`, an
accidental publish attempt is also guaranteed to collide with existing crates.io names.~~

**Status:** fixed in `315fe87`. Still do not run `cargo publish`.

### I-013 — the `hacks/java6-messaging` fixtures are in no build path

`docker/java6/Dockerfile:22,28-31` compiles only `java/src`. Nothing compiles or runs
`hacks/java6-messaging/src/**`, so the BACKLOG acceptance criterion "the Java 6 fixture registers
a JMX metrics MBean" is not exercised by any build. The `java6-classes/` directory in that tree
is stale local output, not build evidence.

**Status:** a fix exists but is **UNCOMMITTED** — `docker/java6/Dockerfile` now compiles the
fixtures into a separate `build/fixtures` tree (kept out of the distributable JAR) and
`.github/workflows/test.yml` runs `LegacyJmsJmxDemo`. Both changes sit in the working tree only,
so this issue is **not** closed and a `git clean` would reopen it. Tracked as **VER-07** in
[IMPLEMENTATION_TASKS.md](IMPLEMENTATION_TASKS.md).

### I-014 — the project's ambition tier is contradictory across documents

`README.md:9` says the project *studies* a compatibility layer and `README.md:75` calls the
architecture a design hypothesis, while [MILESTONES.md](MILESTONES.md) targets a
"Production-usable" v1.0.0 and [BACKLOG.md](BACKLOG.md) specifies production-grade fail-closed
delivery semantics. A research prototype and a vendor-facing production SDK have different bars
for "done", so this ambiguity makes completion unmeasurable.

**Status:** open decision for the maintainer. [ROADMAP.md](ROADMAP.md) currently writes to the
stricter (production) reading.

## Verification status

No claim in this repository has been validated against the real Java 6 runtime, the target
platforms, or real services. The README states this explicitly, and it remains true.

**Documented reach of the test suites**, so this is not mistaken for coverage:
- The 20 Rust tests in the default `cargo test --workspace` run exercise `InMemoryTransport`
  only. Three additional `#[ignore]`d tests do reach live brokers when run explicitly — see
  I-010 for exactly how far that goes.
- **13** Java test classes now exist and two of them cover messaging
  (`LegacyJmsMessagingTest`, `RoutingPolicyTest`), closing **VER-08**. Both use the in-process
  `LEGACY_JMS` transport, so they exercise the facade and the JNI boundary, not a broker.
- The Rust CI gate has never executed — the fixing commit is unpushed ([BUGS.md](BUGS.md) B-001).
