# Roadmap
<!-- rev:002 (RFC 3339) 2026-08-14T02:22:19Z -->

Status at HEAD `af02427` on `main`. Phases follow the M1/M2 structure in
[BACKLOG.md](BACKLOG.md); tasks are broken out in
[IMPLEMENTATION_TASKS.md](IMPLEMENTATION_TASKS.md).

**Read the qualifier first:** `[x]` here means *the code exists and compiles*. It does **not**
mean the behavior was validated against the Java 6 host product, the target platforms, or a real
broker. No such validation has been recorded — see [ISSUES.md](ISSUES.md) I-010. `[~]` means
partially delivered, with the gap named inline.

## Definition of Done

Previously this file carried no Definition of Done, which made "how far along are we" answerable
only by counting checkboxes — and checkboxes here mean *compiles*, not *works*. The operative
bar lives in [MILESTONES.md](MILESTONES.md):

> **"done" means the code exists *and* evidence exists** — `MILESTONES.md:8`

Concretely, a phase is done when, in addition to its items:

1. At least one real path in it has been executed end to end and the run recorded, and
2. Every guarantee its docs claim is still true after the last change, and
3. Every finding it produced is in a committed `BUGS.md` / `ISSUES.md` / `BACKLOG.md` entry.

**Ambition tier — unresolved, and the docs disagree.** `README.md:9` says the project *studies* a
compatibility layer and `README.md:75` calls it a design hypothesis, while
[MILESTONES.md](MILESTONES.md) targets "v1.0.0 — Production-usable against the locked product"
and [BACKLOG.md](BACKLOG.md) sets production-grade fail-closed delivery requirements. Research
prototype and production SDK have very different bars; **the maintainer needs to pick one**, and
until then this file is written against the production bar because that is the stricter reading.

## Phase 0 — Native boundary and packaging · `[COMPLETE — unvalidated]`

- [x] Rust workspace split into `core`, `http`, `tls`, `messaging`, `jni`
- [x] 25 `Java_*` JNI entry points exported from `crates/jni`
- [x] Java 6 facade compiling at `-source 1.6 -target 1.6`
- [x] Single-JAR packaging with per-platform native resources
- [x] Cross-compilation to linux-x86_64, linux-aarch64, windows-x86_64 via `cargo-zigbuild`
- [x] SHA-256 content-addressed native extraction with cleanup on failure
- [x] ADR recording embedded-JNI over sidecar ([adr/0001](adr/0001-jni-boundary-over-sidecar.md))
- [ ] Native-load smoke test per platform resource — **VER-05**
- [ ] Java 6 base image that is not deprecated — **VER-06**

## Phase 1 — HTTPS and TLS · `[COMPLETE — unvalidated]`

- [x] `ModernHttpsURLConnection` Java 6 facade
- [x] Methods, properties, buffered output, connect/read timeouts covering the TLS handshake
- [x] Status, reason phrase, headers (incl. indexed access), body streams
- [x] Typed content metadata; redirect policy with `maxRedirects(int)`
- [x] TLS floor 1.2, selectable 1.2/1.3, rejection of unsupported values
- [x] Peer certificate and cipher-suite access
- [x] Capability bitmask for feature discovery
- [x] Custom `HostnameVerifier`/`SSLSocketFactory` rejected rather than ignored
- [ ] Recorded real Java 6 run against a live endpoint — **VER-04**

## Phase 2 — Messaging transports · `[IMPLEMENTED — NO RUNTIME EVIDENCE]`

- [x] Uniform transport boundary fronting all providers
- [x] NATS, NATS JetStream (durable pull + server-side ack), Kafka, Pulsar, RabbitMQ transports
- [x] In-process `LEGACY_JMS` transport for transparent-mode fixtures
- [x] JMS-shaped Java facade: ConnectionFactory / Connection / Session / Producer / Consumer
- [x] Typed delivery receipts and acknowledgement modes across the boundary
- [x] Trace context as a first-class envelope field, preserved across modes
- [~] Routing dispatch with provider-mismatch rejection and auditable receipt — **but the policy
      engine is unreachable from Java.** `crates/jni/src/lib.rs:217-221` constructs every
      `RouteConfig` with `rules: Vec::new()`, and no Java source references `RouteRule`,
      `RouteConfig`, or `dry_run` (0 matches under `java/`). Rule matching, allow/deny,
      predicates, dry-run and `rule_id` receipts exist only inside Rust. See [BUGS.md](BUGS.md)
      B-002 and **MSG-06**.
- [x] Read-only JMX metrics MBean, Java 6-compatible
- [x] Contract fixtures under `hacks/` for publisher/consumer across providers
- [ ] **Broker fixtures in CI** — **VER-01** ← the gap that makes everything above a claim
- [ ] **Broker-backed send/receive/ack test per provider** — **VER-02**
- [ ] Per-adapter guarantee declarations — **MSG-04**, **DOC-03**
- [ ] Payload categories beyond text (map, stream, bytes, object) — **MSG-05**

## Phase 3 — M1 compatibility scope · `[NOT STARTED]`

- [ ] Versioned envelope schema — **MSG-01**
- [ ] Documented field mappings to JMS/Kafka/Pulsar/NATS/RabbitMQ — **MSG-02**
- [ ] Unsupported mappings fail at configuration time — **MSG-03**
- [ ] Vendor JMS version and interface inventory — **JMS-01** ← blocks the rest of this phase
- [ ] Façade strategy decision (`javax.jms` vs `com.modernlink.jms` vs adapter) — **JMS-02**
- [ ] API compatibility matrix — **JMS-03**
- [ ] Java 6 application-server class-loading model — **JMS-04**
- [ ] JNDI lookup compatibility — **JMS-05**
- [ ] Transactions, selectors, rollback/redelivery, dead-letter — **JMS-06**
- [ ] Broker-backed transparent pass-through prototype — **JMS-07**
- [ ] Full JMX management model — **JMX-01**…**JMX-04**

## Phase 4 — M2 routing, transform, migration · `[NOT STARTED]`

- [ ] Routing policy config: patterns, tenants, predicates, priority, fallback — **RT-01**
- [ ] Dry-run proven distinct from apply — **RT-02**
- [ ] Versioned, auditable policy changes — **RT-03**
- [ ] A failed target never silently acknowledges — **RT-04**
- [ ] Transform envelope: serialization, versioning, idempotency, replay — **RT-05**
- [ ] Duplicate vs redelivery distinguishable — **RT-06**
- [ ] Poison-message quarantine — **RT-07**
- [ ] Migration controls: shadow, dual delivery, cutover, pause/resume, rollback — **RT-08**
- [ ] Cutover and rollback observable through JMX — **RT-09**

## Engineering hygiene · `[PARTIAL]`

- [x] Apache-2.0 LICENSE
- [ ] CI running `cargo test --workspace` + `cargo check -p jni@0.1.0` — **configured, but RED
      for five consecutive pushes**; the job dies building `rdkafka-sys` and never reaches the
      tests. See [BUGS.md](BUGS.md) B-001. Configured is not passing.
- [x] CI building and exercising the packaged JAR (the `Java 6 JAR integration` job passes)
- [x] Crate-level `//!` docs on all five crates
- [ ] `publish = false` on all six manifests — **SC-01** ← live hard-rule violation
- [ ] Toolchain pin + declared MSRV — **SC-02**, **SC-03**
- [ ] `fmt` and `clippy` enforced in CI — **SC-04**
- [ ] All ten Java test classes run in CI (three run today) — **VER-03**
- [ ] Crate-name collisions resolved — **SC-05**, **SC-06**
- [ ] Working coverage measurement — see below

## Test coverage

**N/A — no working coverage measurement.**

`cargo-llvm-cov` is installed and was run (`cargo llvm-cov --workspace --summary-only`), but it
**fails to compile the dependency graph on Windows**: `combine`, `lapin`, `pulsar`, and
`async-nats` all fail under coverage instrumentation, and they are untouched third-party crates.
No percentage is reported here rather than an estimated one.

What is known instead:
- `cargo test --workspace` is CI-gated on ubuntu-latest, **but the gate is failing before the
  tests run** ([BUGS.md](BUGS.md) B-001), so the Rust suite is not being exercised in CI. It has
  been observed to exit 0 locally on Windows (20 tests passed, 2026-08-14) — one platform, one
  machine result, not a coverage figure and not proof of correctness.
- Three of the ten Java test classes execute in CI.
- The Java facade has no coverage tooling at all — no Maven/Gradle means no JaCoCo.

Wiring a coverage measurement that works is tracked as **SC-04** / P2 in
[BACKLOG.md](BACKLOG.md). Running llvm-cov inside the Linux container is the likely fix.

## Overall

| Phase | State |
|---|---|
| 0 — Native boundary and packaging | complete, unvalidated |
| 1 — HTTPS and TLS | complete, unvalidated |
| 2 — Messaging transports | implemented, no runtime evidence |
| 3 — M1 compatibility scope | not started |
| 4 — M2 routing and migration | not started |

Roughly **two of five phases** are code-complete and none is validated. The single highest-value
next step is **VER-01 → VER-02**: broker fixtures in CI, which convert Phase 2 from a set of
claims into evidence. The cheapest is **SC-01**, one line in six manifests.
