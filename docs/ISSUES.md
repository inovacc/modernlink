# Known Issues and Limitations
<!-- rev:007 (RFC 3339) 2026-08-19T00:00:00Z -->

Constraints accepted on purpose or imposed by the platform. Defects that should be fixed live
in [BUGS.md](BUGS.md); future work lives in [BACKLOG.md](BACKLOG.md).

## Crate naming

### ~~I-001 — the `jni` workspace crate shadows its own dependency~~ — **RESOLVED (SC-05)**

The package is now **`jni-bridge`** while the folder stays `crates/jni`, so
`cargo check -p jni` is unambiguous again (it resolves to the external crate, and exits 0).
Every invocation spells **`-p jni-bridge`**; the `@0.1.0` workaround is retired.

`[lib] name` is still `modernlink`, so the shipped native artifact is unchanged —
`cargo metadata` confirms `jni-bridge` builds the `modernlink` lib target.

The original report follows.

_`crates/jni` was named `jni` and depends on the external crate `jni = "0.21"`, so
`cargo check -p jni` was ambiguous and every invocation had to disambiguate by version._

### ~~I-002 — the `core` crate shadows Rust's built-in `core`~~ — **RESOLVED (SC-06)**

`crates/core` is named `core`, which is a Rust built-in crate name. Dependent crates then
write `use core::{Error, Request, Response, TlsInfo};` (`crates/http/src/lib.rs:2`), which
reads as the standard library but resolves to the workspace crate.

**Status:** **RESOLVED (SC-06).** The package is now `modernlink-core` while the folder stays
`crates/core`, which is exactly the carve-out the standing crate-naming rule allows: a library
whose bare name would shadow a Rust built-in (`core`, `std`, `alloc`, `test`, `proc_macro`) may
carry a distinguishing **package** name provided the folder path stays short. Dependents now
write `use modernlink_core::{...}`, which no longer reads as the standard library. 20 source
references were rewritten across `crates/{http,tls,jni}`.

## Packaging and platform

### I-003 — no Maven or Gradle build for the Java facade

`java/src` has no `pom.xml`, `build.gradle`, or wrapper. Compilation happens only inside
`docker/java6/Dockerfile` via `find ... | xargs javac -source 1.6 -target 1.6`. There is no
way to build or test the Java side without Docker, and no IDE project model.

**Status:** deliberate for now — it guarantees the Java 6 compiler is the one that judges the
facade. It also means Java changes are invisible to `cargo test`.

**The measurement consequence (H-13), stated so the numbers are not misread.** No Maven or
Gradle means no JaCoCo, so **there is no coverage measurement for `java/src` at all** — not a
low number, no number. Two things follow, and both are easy to get wrong:

1. **`crates/jni` reads 14.24% and is the most-exercised surface in the project.** Its 28
   `Java_*` entry points are driven by the 15 Java test classes running against the packaged
   JAR, which `cargo llvm-cov` cannot observe. The figure counts only what Rust tests reach.
   Treating it as "the JNI boundary is barely tested" is exactly backwards.
2. **The 15 Java classes are themselves unmeasured.** Nothing reports which facade branches
   they miss, so "15 test classes pass" says they pass, not that they cover anything in
   particular.

Closing this needs a Java build system, which is what this issue is about. Until then the
workspace coverage figure describes the Rust half only, and any statement about
facade coverage is an opinion. **Two of the 15 classes have never been compiled or run at
all** — `ProviderGuaranteesTest` and `PayloadCategoriesTest` were added after the last CI
run.

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

**Status:** **resolved in `76a17f0`.** `docker/java6/Dockerfile` compiles the fixtures into a
separate `build/fixtures` tree (kept out of the distributable JAR) and
`.github/workflows/test.yml` runs `LegacyJmsJmxDemo` in transparent `LEGACY_JMS` mode. Both
changes are committed and both ran green on run
[31782837766](https://github.com/inovacc/modernlink/actions/runs/31782837766). Closes **VER-07**.

### I-014 — the project's ambition tier is contradictory across documents

`README.md:9` says the project *studies* a compatibility layer and `README.md:75` calls the
architecture a design hypothesis, while [MILESTONES.md](MILESTONES.md) targets a
"Production-usable" v1.0.0 and [BACKLOG.md](BACKLOG.md) specifies production-grade fail-closed
delivery semantics. A research prototype and a vendor-facing production SDK have different bars
for "done", so this ambiguity makes completion unmeasurable.

**Status:** open decision for the maintainer. [ROADMAP.md](ROADMAP.md) currently writes to the
stricter (production) reading.

## Verification status

The Java 6 *runtime* is now exercised; the **vendor host product** is not, and neither is its
JMS implementation (I-009, I-011). That distinction is the whole of this section — do not
collapse the two.

**Documented reach of the test suites**, so this is not mistaken for coverage:
- `cargo test --workspace` runs **23** tests by default and **37** with `--all-features`. None
  of them reaches a broker: the transport coverage is `InMemoryTransport` only, and the rest
  exercise the domain, the routing policy, the provider guarantee table and the payload
  categories. **5** `#[ignore]`d tests do reach live brokers when run explicitly, and only three
  of those five have ever been executed — see I-010 for exactly how far that goes.
- **15** Java test classes now exist and four of them cover messaging
  (`LegacyJmsMessagingTest`, `RoutingPolicyTest`, `ProviderGuaranteesTest`,
  `PayloadCategoriesTest`), closing **VER-08**. All four use the in-process `LEGACY_JMS`
  transport or static data, so they exercise the facade and the JNI boundary, not a broker.
  **The two newest have never been compiled or run**: that needs the Java 6 image.
- The Rust CI gate executes and passes: run
  [31782837766](https://github.com/inovacc/modernlink/actions/runs/31782837766) at `d2479bd` ran
  `test`, `check`, `fmt` and `clippy` green ([BUGS.md](BUGS.md) B-001 resolved). It proves those
  four commands exit 0 on ubuntu — it does not reach a broker, because the three broker-backed
  tests are `#[ignore]`d.
- **No run against the vendor host product** has ever been recorded.
