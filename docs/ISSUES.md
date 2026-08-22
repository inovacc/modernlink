# Known Issues and Limitations
<!-- rev:014 (RFC 3339) 2026-08-22T03:15:42Z -->

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

**Measurement consequence (H-13).** Maven/Gradle remains absent, but coverage no longer depends
on either tool. Run 32523731422 used JaCoCo on JDK 8 against Java 6-targeted classes and recorded
802/889 production lines (90.21%). The Rust harness instruments `libmodernlink` and loads it from
Java, so Java-driven JNI paths contribute to llvm-cov; the same run recorded 1,496/1,650 lines
(90.67%) in the enforced four behavior crates and 2,814/3,075 full production lines (91.51%).
These machine percentages do not establish vendor-host compatibility.

### I-004 — the `java:6b38-jdk` base image is deprecated

`docker/java6/Dockerfile:18` pulls `java:6b38-jdk`. The legacy Java image family is deprecated
and may no longer be served by every registry, which makes the JAR build environment-dependent.

**Status:** unresolved. No vendored or self-built Java 6 base image exists.

### I-005 — the workspace may not build on a bare host

Provider features pull `rdkafka` with `cmake-build` and `pulsar`, which need `cmake`, `libcurl`,
and `protobuf-compiler`. The default workspace build compiles no broker client. The Dockerfile
and all-provider workflow jobs install the native dependencies; B-001 records the earlier CI
failure and its committed remediation. The merged mainline has recorded all-provider CI runs; a
bare host still needs those packages for local all-feature builds.

**Workaround for local all-provider builds:** build through the Dockerfile, or install cmake +
libcurl + protoc locally.

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

**Recorded machine reach:** run 32386474212 at `3b64484` reported `success` conclusions for the
dedicated NATS/JetStream/RabbitMQ and Kafka/Pulsar jobs, each configured around a single
send → receive → acknowledge path.

**What is still a source-level claim:**
- Durability, reconnect, concurrency, ordering, failure and redelivery semantics are
  unexercised for **every** provider. One happy-path round trip is
  not delivery semantics.
- The tests are `#[ignore]`d so a normal `cargo test` does not run them, and they require
  explicit broker setup. Dedicated workflow jobs invoke them; ordinary Rust jobs do not.

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
`.github/workflows/test.yml` invokes `LegacyJmsJmxDemo` in transparent `LEGACY_JMS` mode. Run
[31782837766](https://github.com/inovacc/modernlink/actions/runs/31782837766) recorded that
configured fixture step. Closes **VER-07** for build-path reach only.

### I-014 — the project's ambition tier was contradictory across documents

The README now describes ModernLink as a compatibility layer, marks it as actively under
development, and records the published-JAR Docker reach. [MILESTONES.md](MILESTONES.md) targets a
"Production-usable" v1.0.0 and [BACKLOG.md](BACKLOG.md) specifies production-grade fail-closed
delivery semantics. The vendor host and direct Windows release DLL remain explicit unproven bars.

**Status:** clarified in the documentation; the production bar remains the roadmap reading.

## Verification status

The exact revision-scoped execution ledger is [VERIFICATION.md](VERIFICATION.md). The central
limitation remains unchanged: the Java 6 runtime has recorded probes, but the vendor host
product and its JMS implementation have never been part of a recorded run (I-009, I-011).
