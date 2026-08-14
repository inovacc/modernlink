# Implementation Tasks
<!-- rev:002 (RFC 3339) 2026-08-14T02:22:19Z -->

Granular tasks derived from [BACKLOG.md](BACKLOG.md), [FEATURES.md](FEATURES.md), and
[ISSUES.md](ISSUES.md). Effort: **S** ≤ half a day · **M** ≤ two days · **L** > two days.
Task IDs are referenced from [ROADMAP.md](ROADMAP.md) and [MILESTONES.md](MILESTONES.md).

## Domain: supply chain and hygiene

| ID | What | Files | Deps | Effort |
|---|---|---|---|---|
| SC-01 | Add `publish = false` to all six manifests (ISSUES I-012) | `crates/{core,http,tls,jni,messaging}/Cargo.toml`, `hacks/messaging-demo/Cargo.toml` | — | S |
| SC-02 | Pin the toolchain with `rust-toolchain.toml` so CI and the packaging image agree | `rust-toolchain.toml` | — | S |
| SC-03 | Declare an MSRV (`rust-version`) in `[workspace.package]` | `Cargo.toml` | SC-02 | S |
| SC-04 | Add `cargo fmt --check` + `cargo clippy -D warnings` jobs to CI | `.github/workflows/test.yml` | — | S |
| SC-05 | Rename `crates/jni` → a non-colliding package name, dropping the `@0.1.0` workaround (I-001) | `crates/jni/Cargo.toml`, `Cargo.toml`, `.github/workflows/test.yml`, `docker/java6/Dockerfile` | SC-04 | M |
| SC-06 | Rename `crates/core` so it stops shadowing Rust's built-in `core` (I-002) | `crates/core/Cargo.toml`, all dependents | SC-05 | M |
| SC-07 | **Unblock the red CI Rust job (BUGS B-001).** Preferred: feature-gate the providers — `crates/messaging/Cargo.toml` has no `[features]`, so `rdkafka`/`pulsar`/`lapin`/`async-nats` are unconditional and every workspace test/clippy/coverage run must build a native Kafka client; gating them also fixes the Windows `llvm-cov` failure. Fallback: add `libcurl4-openssl-dev` (+ likely `libsasl2-dev`) to the job, per `docker/java6/Dockerfile:8` | `crates/messaging/Cargo.toml`, `.github/workflows/test.yml` | — | M |
| SC-08 | Commit the ten untracked state docs (`AGENTS.md`, `CLAUDE.md`, `docs/{ARCHITECTURE,BUGS,CONTRIBUTORS,FEATURES,IMPLEMENTATION_TASKS,ISSUES,MILESTONES,ROADMAP}.md`, `docs/adr/`) — until they are tracked, no finding in them survives a `git clean`, and the durable-record rule is unsatisfiable | repo root, `docs/` | — | S |

## Domain: crate documentation

| ID | What | Files | Deps | Effort |
|---|---|---|---|---|
| ~~DOC-01~~ | ~~Add `//!` crate-level docs to all five crates~~ — **DONE** (2026-08-14): `//!` present in all five `crates/*/src/lib.rs`. Note the change is in the working tree and not yet committed (see SC-08) | `crates/*/src/lib.rs` | — | S |
| DOC-02 | Split the five monolithic `lib.rs` files into modules (each crate is currently one file; `crates/messaging` carries six transports) | `crates/*/src/` | DOC-01 | L |
| DOC-03 | Document per-provider guarantees (ordering, persistence, ack, transactions, replay, backpressure, TLS, auth, DLQ) | `docs/routing-semantics.md`, new `docs/providers.md` | MSG-04 | M |

## Domain: verification (highest value — nothing here is proven)

| ID | What | Files | Deps | Effort |
|---|---|---|---|---|
| MSG-06 | **Expose the routing policy engine across the JNI boundary** — let Java supply a rule set instead of the hardcoded `rules: Vec::new()` at `crates/jni/src/lib.rs:217-221`, and surface dry-run + `rule_id` receipts. Closes BUGS B-002 | `crates/jni/src/lib.rs`, `java/src/main/java/com/modernlink/messaging/` | — | M |
| VER-01 | Stand up broker fixtures in CI (testcontainers or compose) for NATS, Kafka, Pulsar, RabbitMQ | `.github/workflows/test.yml`, `docker/` | — | L |
| VER-06 | **Add Java-side messaging tests — there are currently zero.** All ten classes in `java/src/test` cover HTTP/utility only; the ~18-class JMS-shaped facade has no test at all, so VER-03 ("run all ten") would still leave messaging at zero Java coverage | `java/src/test/java/com/modernlink/messaging/`, `.github/workflows/test.yml` | — | M |
| VER-07 | Put `hacks/java6-messaging/` into a build path — `docker/java6/Dockerfile:22,28-31` compiles only `java/src`, so the BACKLOG acceptance criterion "the Java 6 fixture registers a JMX metrics MBean" is not exercised by anything | `docker/java6/Dockerfile`, `hacks/java6-messaging/` | — | S |
| VER-02 | Broker-backed send/receive/ack integration test per provider (closes I-010) | `crates/messaging/tests/` | VER-01 | L |
| VER-03 | Run **all ten** Java test classes in CI, not the current three | `.github/workflows/test.yml` | — | S |
| VER-04 | Record a real Java 6 run of the packaged JAR against a live HTTPS endpoint | `docs/evidence/` | — | M |
| VER-05 | Native-load smoke test per platform resource (linux-x86_64, linux-aarch64, windows-x86_64) | `java/src/test/java/com/modernlink/` | VER-03 | M |
| VER-06 | Self-hosted or vendored Java 6 base image so the JAR build stops depending on a deprecated tag (I-004) | `docker/java6/` | — | M |

## Domain: message domain (BACKLOG M1)

| ID | What | Files | Deps | Effort |
|---|---|---|---|---|
| MSG-01 | Version the envelope schema and record the version in the envelope | `crates/messaging/src/` | DOC-02 | M |
| MSG-02 | Document field-by-field mappings to JMS, Kafka, Pulsar, NATS, RabbitMQ | `docs/jms-compatibility.md`, `docs/providers.md` | MSG-01 | M |
| MSG-03 | Make unsupported mappings fail explicitly at configuration time, not at publish time | `crates/messaging/src/` | MSG-01 | M |
| MSG-04 | Declare each adapter's guarantees in code so capability gaps are queryable before traffic moves | `crates/messaging/src/` | MSG-01 | M |
| MSG-05 | Add map, stream, bytes, and object payload categories (only text is exercised today) | `crates/messaging/src/`, `java/.../messaging/` | MSG-01 | M |

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

1. **SC-01** — one line per manifest, closes a live hard-rule violation.
2. **DOC-01, SC-02, SC-03, SC-04, VER-03** — small, independent, and SC-04/VER-03 make every later change safer.
3. **JMS-01** — blocks the entire JMS domain and is research, not code.
4. **VER-01 → VER-02** — turns the messaging code from claim into evidence. Highest value in the list.
5. **MSG-01 → MSG-02/03/04** — pins the domain the adapters share.
6. **JMS-02 → JMS-03/04**, **JMX-01 → JMX-02/03/04** — parallel once their heads land.
7. **RT-\*** — M2, after the M1 domain is stable.
8. **SC-05/SC-06, DOC-02** — invasive renames and module splits; do them when CI can catch the fallout.
