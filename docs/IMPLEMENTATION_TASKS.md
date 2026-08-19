# Implementation Tasks
<!-- rev:010 (RFC 3339) 2026-08-19T00:00:00Z -->

Granular tasks derived from [BACKLOG.md](BACKLOG.md), [FEATURES.md](FEATURES.md), and
[ISSUES.md](ISSUES.md). Effort: **S** ≤ half a day · **M** ≤ two days · **L** > two days.
Task IDs are referenced from [ROADMAP.md](ROADMAP.md) and [MILESTONES.md](MILESTONES.md).

## Domain: supply chain and hygiene

| ID | What | Files | Deps | Effort |
|---|---|---|---|---|
| ~~SC-01~~ | ~~Add `publish = false` to all six manifests~~ — **DONE `315fe87`** | `crates/{core,http,tls,jni,messaging}/Cargo.toml`, `hacks/messaging-demo/Cargo.toml` | — | S |
| ~~SC-02~~ | ~~Pin the toolchain with `rust-toolchain.toml` so CI and the packaging image agree~~ — **DONE**: `rust-toolchain.toml` pins `1.96.0` with rustfmt+clippy; `[workspace.package] rust-version = "1.96"` propagates to all six crates (`cargo metadata` confirms). CI installs no toolchain of its own and `docker/java6/Dockerfile` COPYs the file in, so all three read one source | `rust-toolchain.toml`, `.github/workflows/test.yml`, `docker/java6/Dockerfile` | — | S |
| ~~SC-03~~ | ~~Declare an MSRV (`rust-version`) in `[workspace.package]`~~ — **DONE**: `rust-version = "1.96"`, inherited by all six crates via `rust-version.workspace = true`. It is the pinned build version, **not a probed floor** — no older toolchain has been tried | `Cargo.toml`, `crates/*/Cargo.toml`, `hacks/messaging-demo/Cargo.toml` | SC-02 | S |
| ~~SC-04~~ | ~~Add `cargo fmt --check` + `cargo clippy -D warnings` jobs to CI~~ — **DONE `dd080b2`**; both executed green on run 31782837766 at `d2479bd` | `.github/workflows/test.yml` | — | S |
| ~~SC-05~~ | ~~Rename `crates/jni` to a non-colliding package name, dropping the `@0.1.0` workaround (I-001)~~ — **DONE**: package `jni-bridge`, folder unchanged, `[lib] name` still `modernlink`. `cargo check -p jni` exits 0 unambiguously | `crates/jni/Cargo.toml`, `.github/workflows/test.yml`, `docker/java6/Dockerfile`, docs | — | M |
| ~~SC-06~~ | ~~Rename `crates/core` so it stops shadowing Rust's built-in `core` (I-002)~~ — **DONE**: package `modernlink-core`, folder unchanged (the carve-out the crate-naming rule allows for built-in-shadowing names). 20 `core::` references rewritten to `modernlink_core::` across http, tls and jni | `crates/core/Cargo.toml`, `crates/{http,tls,jni}` | SC-05 | M |
| ~~SC-07~~ | ~~**Unblock the red CI Rust job (BUGS B-001).** Preferred: feature-gate the providers — `crates/messaging/Cargo.toml` has no `[features]`, so `rdkafka`/`pulsar`/`lapin`/`async-nats` are unconditional and every workspace test/clippy/coverage run must build a native Kafka client; gating them also fixes the Windows `llvm-cov` failure. Fallback: add `libcurl4-openssl-dev` (+ likely `libsasl2-dev`) to the job, per `docker/java6/Dockerfile:8`~~ — **DONE**: `[features]` with `default = []` on `crates/messaging`, forwarded through `crates/jni` and `hacks/messaging-demo`. `cargo tree -p jni-bridge` shows 0 provider deps by default, 5 with `--features all-providers`. A provider compiled out **fails closed** with an explicit error naming the feature, covered by three regression tests in `crates/jni/src/lib.rs` (falsified: making Kafka fall back to LEGACY_JMS makes the test fail) | `crates/messaging/Cargo.toml`, `crates/jni/`, `hacks/messaging-demo/Cargo.toml`, `.github/workflows/test.yml`, `docker/java6/Dockerfile` | — | M |
| ~~SC-08~~ | ~~Commit the ten untracked state docs~~ — **DONE**: `git ls-files` returns all of `AGENTS.md`, `CLAUDE.md`, `docs/{ARCHITECTURE,BUGS,FEATURES,IMPLEMENTATION_TASKS,ISSUES,MILESTONES,ROADMAP}.md` and `docs/adr/`; the working tree is clean | repo root, `docs/` | — | S |

## Domain: crate documentation

| ID | What | Files | Deps | Effort |
|---|---|---|---|---|
| ~~DOC-01~~ | ~~Add `//!` crate-level docs to all five crates~~ — **DONE `210f193`**: `//!` present in all five `crates/*/src/lib.rs`, committed | `crates/*/src/lib.rs` | — | S |
| DOC-02 | Split the five monolithic `lib.rs` files into modules (each crate is currently one file; `crates/messaging` carries six transports) | `crates/*/src/` | DOC-01 | L |
| ~~DOC-03~~ | ~~Document per-provider guarantees~~ — **DONE**: [docs/providers.md](providers.md). TLS/auth and backpressure are **deliberately absent** with the reason stated, rather than shown as empty columns that would imply the analysis was done | `docs/providers.md` | MSG-04 | M |

## Domain: verification (highest value — nothing here is proven)

| ID | What | Files | Deps | Effort |
|---|---|---|---|---|
| ~~MSG-06~~ | ~~Expose the routing policy engine across the JNI boundary~~ — **DONE `ad4bd2f`**, closes BUGS B-002. `nativeOpenRouted` (`crates/jni/src/lib.rs:354`) takes a rule set; `nativeDryRun` (`:447`) evaluates policy without publishing; `ModernRouteRule` / `ModernRouteDecision` / `evaluateRoute` expose it. Falsified before acceptance | `crates/jni/src/lib.rs`, `java/src/main/java/com/modernlink/messaging/` | — | M |
| VER-01 | Stand up broker fixtures in CI for NATS, Kafka, Pulsar, RabbitMQ — **partially done**: a `Broker-backed messaging` job starts `nats:2.10 -js` + `rabbitmq:3.13` via `docker run` (not `services:`, which cannot pass `-js`), waits on broker logs rather than ports, runs the three `#[ignore]`d tests and asserts all three ran. **Kafka and Pulsar are not covered** (no test exists — VER-02). **Never executed** — unproven until a real run | `.github/workflows/test.yml` | — | L |
| ~~VER-08~~ | ~~Add Java-side messaging tests — there are currently zero.~~ — **DONE `5da1ec2` + `ad4bd2f`**: `LegacyJmsMessagingTest` (round trip, CLIENT-ack, JMX register/unregister with no payload leak, async listener) and `RoutingPolicyTest`. Both use in-process `LEGACY_JMS`, so they cover the facade and the JNI boundary, not a broker. *(Renumbered from VER-06, which the base-image task below already owned — an ID collision introduced 2026-08-14 and corrected the same day.)* | `java/src/test/java/com/modernlink/messaging/`, `.github/workflows/test.yml` | — | M |
| ~~VER-07~~ | ~~Put `hacks/java6-messaging/` into a build path~~ — **DONE `76a17f0`**: `docker/java6/Dockerfile` compiles the fixtures into a separate `build/fixtures` tree and `test.yml` runs `LegacyJmsJmxDemo`; green on run 31782837766 | `docker/java6/Dockerfile`, `hacks/java6-messaging/` | — | S |
| VER-02 | Broker-backed send/receive/ack per provider (closes I-010) — **partially DONE**: NATS core, JetStream and RabbitMQ pass against live brokers (`a2419b5`, 2026-08-14, Codex-verified). Kafka and Pulsar now have tests in their own targets (`required-features` is all-or-nothing, so one target for all five would force every broker job to build librdkafka), but **neither has been executed** — written is not proven. Only the happy path is covered for any provider | `crates/messaging/tests/` | VER-01 | L |
| ~~VER-03~~ | ~~Run all ten Java test classes in CI, not the current three~~ — **DONE `dd080b2`**: the workflow enumerates all **13**, and all 13 ran green on runs 31781200582 and 31782837766 | `.github/workflows/test.yml` | — | S |
| ~~VER-04~~ | ~~Record a real **Java 6** run of the packaged JAR against a live HTTPS endpoint~~ — **DONE**: run 31781200582 executed the packaged JAR on JVM `1.6.0_38` (`Linux/amd64`) reaching `tls-protocol=TLSv1_3`. The earlier local record (`docs/evidence/2026-08-14-native-runtime.md`, `status=200`, 4 peer certs) used JVM 21 and covered the JNI boundary only | `docs/evidence/` | — | M |
| VER-05 | Native-load smoke test per platform — **windows-x86_64 DONE `632eaa7`** (`NativeLoadSmokeTest`, local JVM 21) and **linux-x86_64 DONE** (`native-smoke-load=ok` on JVM 1.6.0_38, run 31781200582). **linux-aarch64 has never been loaded on any JVM** — the only platform left | `java/src/test/java/com/modernlink/` | VER-03 | M |
| VER-06 | Self-hosted or vendored Java 6 base image so the JAR build stops depending on a deprecated tag (I-004) | `docker/java6/` | — | M |

## Domain: message domain (BACKLOG M1)

| ID | What | Files | Deps | Effort |
|---|---|---|---|---|
| MSG-01 | Version the envelope schema and record the version in the envelope | `crates/messaging/src/` | DOC-02 | M |
| MSG-02 | Document field-by-field mappings to JMS, Kafka, Pulsar, NATS, RabbitMQ | `docs/jms-compatibility.md`, `docs/providers.md` | MSG-01 | M |
| MSG-03 | Make unsupported mappings fail explicitly at configuration time, not at publish time | `crates/messaging/src/` | MSG-01 | M |
| ~~MSG-04~~ | ~~Declare each adapter's guarantees in code so capability gaps are queryable before traffic moves~~ — **DONE**: `Support` (VERIFIED/DECLARED/UNSUPPORTED), `ProviderGuarantees`, `Provider::guarantees()`, plus `require_delivery_mode`/`require_acknowledgement_mode` fail-closed checks. Exposed as `nativeProviderGuarantees` and `ModernMessagingClient.guaranteesFor(...)`, which needs **no connection**. 7 Rust tests + `ProviderGuaranteesTest`. Found **B-003** | `crates/messaging/src/`, `crates/jni/src/`, `java/.../messaging/` | — | M |
| MSG-05 | Add map, stream, bytes and object payload categories — **partially DONE**: TEXT/BYTES/MAP cross the JNI boundary; the frame gained a category field and the body is base64 for every category, so a BytesMessage is not mangled by a UTF-8 round trip. Map pairs are `base64(k):base64(v)` -- `:` and `,` because `=` is base64 *padding* and split the keys (caught by the delimiter test). **STREAM and OBJECT refuse with a reason**; OBJECT because Java deserialization of broker bytes is an RCE surface. JMS-shaped session wrappers (`createBytesMessage` etc.) remain — that is JMS-03 | `crates/jni/src/`, `java/.../messaging/` | — | M |

## Domain: JMS compatibility (BACKLOG M1)

| ID | What | Files | Deps | Effort |
|---|---|---|---|---|
| JMS-01 | Inventory the vendor product's exact JMS version and interfaces (blocks the rest — I-011) | `docs/jms-compatibility.md` | — | M |
| JMS-02 | Decide: binary-compatible `javax.jms` façade vs source-compatible `com.modernlink.jms` vs vendor adapter | `docs/adr/0002-*.md` | JMS-01 | M |
| JMS-03 | Publish the API compatibility matrix — every supported method and semantic | `docs/jms-compatibility.md` | JMS-02 | L |
| JMS-04 | Define class-loading and packaging behavior for Java 6 application servers | `docs/`, `docker/java6/` | JMS-02 | L |
| JMS-05 | JNDI lookup compatibility | `java/.../messaging/` | JMS-02 | L |
| JMS-06 | Transactions, selectors, rollback / redelivery, dead-letter behavior | `crates/messaging/`, `java/.../messaging/` | JMS-03 | L |
| JMS-07 | Broker-backed transparent pass-through prototype against one real provider | `crates/messaging/`, `hacks/` | JMS-06, VER-01 | L |

## Domain: JMX management (BACKLOG M1)

| ID | What | Files | Deps | Effort |
|---|---|---|---|---|
| JMX-01 | Specify MBeans for health, connection/session state, route decisions, queue metrics, retries, dead letters, inflight | `docs/jmx-management.md` | — | M |
| JMX-02 | Separate read-only operational metrics from mutating controls | `java/.../messaging/` | JMX-01 | M |
| JMX-03 | Stable object names and attribute meanings held constant across providers | `java/.../messaging/` | JMX-01 | M |
| JMX-04 | Assert no credential, payload, or message body can reach a JMX attribute or log | `crates/messaging/`, `java/.../messaging/` | JMX-02 | S |

## Domain: routing and migration (BACKLOG M2)

| ID | What | Files | Deps | Effort |
|---|---|---|---|---|
| RT-01 | Routing config: exact + pattern destination mappings, tenant rules, header predicates, priority, fallback | `crates/messaging/src/` | MSG-04 | L |
| RT-02 | Make dry-run evaluation observable and prove it differs from apply (unverified today — I-010) | `crates/messaging/src/` | RT-01 | M |
| RT-03 | Version and audit policy changes | `crates/messaging/src/` | RT-01 | M |
| RT-04 | Guarantee a failed target never silently acknowledges the legacy message | `crates/messaging/src/` | RT-01 | M |
| RT-05 | Transform envelope: serialization, schema versioning, idempotency keys, retry classification, replay | `crates/messaging/src/` | MSG-01 | L |
| RT-06 | Distinguish duplicate delivery from redelivery | `crates/messaging/src/` | RT-05 | M |
| RT-07 | Quarantine poison messages without blocking unrelated traffic | `crates/messaging/src/` | RT-05 | M |
| RT-08 | Migration controls: shadow publish, sampled dual delivery, cutover by destination/tenant, pause/resume, rollback | `crates/messaging/src/` | RT-01 | L |
| RT-09 | Surface cutover and rollback through JMX | `java/.../messaging/` | RT-08, JMX-02 | M |

## Suggested order

Dependency-respecting, cheapest-unblocking-first:

1. ~~**SC-01**~~, ~~**DOC-01**~~, ~~**SC-04**~~, ~~**VER-03**~~, ~~**VER-07**~~, ~~**VER-08**~~, ~~**SC-08**~~, ~~**MSG-06**~~ — all landed; see the rows above.
2. ~~**SC-02**, **SC-03**~~ — done; the toolchain is pinned and the MSRV declared. **SC-07** is next, so a broker-free run stops building a native Kafka client.
3. **JMS-01** — blocks the entire JMS domain and is research, not code.
4. **VER-01 → VER-02** — turns the messaging code from claim into evidence. Highest value in the list.
5. **MSG-01 → MSG-02/03/04** — pins the domain the adapters share.
6. **JMS-02 → JMS-03/04**, **JMX-01 → JMX-02/03/04** — parallel once their heads land.
7. **RT-\*** — M2, after the M1 domain is stable.
8. **SC-05/SC-06, DOC-02** — invasive renames and module splits; do them when CI can catch the fallout.
