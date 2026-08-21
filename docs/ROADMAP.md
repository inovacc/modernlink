# Roadmap
<!-- rev:023 (RFC 3339) 2026-08-21T19:43:45Z -->

Reconciled 2026-08-21 against the current tree. Phases follow the M1/M2 structure in
[BACKLOG.md](BACKLOG.md); tasks are broken out in [IMPLEMENTATION_TASKS.md](IMPLEMENTATION_TASKS.md).
Execution claims are centralized in [VERIFICATION.md](VERIFICATION.md) so this roadmap does not
turn a machine result into a completion verdict.

**Read the qualifier first:** `[x]` here means *the code exists and compiles*. It does **not**
mean the behavior was validated against the Java 6 host product or the vendor's own JMS
implementation — that has never been recorded, see [ISSUES.md](ISSUES.md) I-009/I-011. Recorded
Java, native, and broker runs are scoped in [VERIFICATION.md](VERIFICATION.md). `[~]` means
partially delivered, with the gap named.

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
- [x] 28 `Java_*` JNI entry points exported from `crates/jni` (package `jni-bridge`)
- [x] Java 6 facade compiling at `-source 1.6 -target 1.6`
- [x] Single-JAR packaging with per-platform native resources
- [x] Cross-compilation to linux-x86_64, linux-aarch64, windows-x86_64 via `cargo-zigbuild`
- [x] SHA-256 content-addressed native extraction with cleanup on failure
- [x] ADR recording embedded-JNI over sidecar ([adr/0001](adr/0001-jni-boundary-over-sidecar.md))
- [x] Native-load smoke test per platform resource — **VER-05**. windows-x86_64 (local, JVM 21)
      and linux-x86_64 (CI, JVM 1.6.0_38) both load. A `linux-aarch64 native load` job now runs
      `NativeLoadSmokeTest` on an `ubuntu-24.04-arm` runner against the JAR built by the amd64
      job, and asserts the platform line really reports `aarch64`. Run
      [32386474212](https://github.com/inovacc/modernlink/actions/runs/32386474212) recorded the
      configured load/assert steps at `3b64484`; this is native-load evidence, not vendor-host
      compatibility.
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
- [x] Recorded packaged Java 6 run against a live HTTPS endpoint — **VER-04**. The command and
      revision are in [VERIFICATION.md](VERIFICATION.md); it is not a vendor-host result.

## Phase 2 — Messaging transports · `[IMPLEMENTED — LIMITED HAPPY-PATH EVIDENCE]`

- [x] Uniform transport boundary fronting all providers
- [x] NATS, NATS JetStream (durable pull + server-side ack), Kafka, Pulsar, RabbitMQ transports
- [x] In-process `LEGACY_JMS` transport for transparent-mode fixtures
- [x] JMS-shaped Java facade: ConnectionFactory / Connection / Session / Producer / Consumer
- [x] Typed delivery receipts and acknowledgement modes across the boundary
- [x] Trace context as a first-class envelope field, preserved across modes
- [x] Routing dispatch with policy, provider-mismatch rejection and auditable receipt —
      **reachable from Java** as of `ad4bd2f` (**MSG-06**, closes [BUGS.md](BUGS.md) B-002).
      `nativeOpenRouted` accepts a rule set (`crates/jni/src/lib.rs:354`, parsing at `:379-393`)
      and `nativeDryRun` (`:447`) evaluates policy without publishing; `ModernRouteRule` /
      `ModernRouteDecision` / `ModernConnection.evaluateRoute` expose it, covered by
      `RoutingPolicyTest`. Falsified before acceptance: forcing `rules: Vec::new()` back made the
      test fail, reverting made it pass.
- [x] Read-only JMX metrics MBean, Java 6-compatible
- [x] Contract fixtures under `hacks/` for publisher/consumer across providers
- [x] **Broker fixtures in CI** — **VER-01**. Dedicated jobs start NATS/JetStream/RabbitMQ and
      Kafka/Pulsar, then explicitly invoke the five `#[ignore]`d tests. Run
      [32386474212](https://github.com/inovacc/modernlink/actions/runs/32386474212) at `3b64484`
      recorded `success` conclusions for both broker jobs. The ordinary Rust jobs still execute
      none of these tests.
- [x] **Broker-backed send/receive/ack test per provider** — **VER-02**. Test targets exist for
      all five providers, and run 32386474212 recorded one configured happy-path execution for
      each. Durability, reconnect, ordering under load, concurrency, failure recovery,
      rollback/redelivery, and dead-letter semantics remain unexercised for every provider.
- [x] Per-adapter guarantee declarations — **MSG-04**, **DOC-03**. `Provider::guarantees()`
      returns a three-level table (VERIFIED / DECLARED / UNSUPPORTED) per provider, reachable
      from Java 6 via `ModernMessagingClient.guaranteesFor(...)` **without opening a
      connection**, and documented in [providers.md](providers.md). It exposed
      [BUGS.md](BUGS.md) **B-003**
- [~] Payload categories beyond text — **MSG-05**. **TEXT, BYTES and MAP** cross the boundary;
      the frame carries the category so base64 is never guessed at. **STREAM and OBJECT are
      refused, deliberately**: STREAM needs typed field ordering the frame does not encode, and
      OBJECT would mean deserializing broker-supplied bytes into Java objects, a
      remote-code-execution surface. Both refuse with the reason rather than degrading to BYTES

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
- [x] Dry-run surface is distinct from dispatch and returns denied decisions without publishing
      — **RT-02**, `RouteConfig::dry_run`, `ModernMessagingClient.dryRun`
- [ ] Versioned, auditable policy changes — **RT-03**
- [ ] A failed target never silently acknowledges — **RT-04**
- [ ] Transform envelope: serialization, versioning, idempotency, replay — **RT-05**
- [ ] Duplicate vs redelivery distinguishable — **RT-06**
- [ ] Poison-message quarantine — **RT-07**
- [ ] Migration controls: shadow, dual delivery, cutover, pause/resume, rollback — **RT-08**
- [ ] Cutover and rollback observable through JMX — **RT-09**

## Engineering hygiene · `[PARTIAL]`

- [x] Apache-2.0 LICENSE
- [x] CI declares `cargo test --workspace` + `cargo check -p jni-bridge`; recorded command exits
      and revision scope are in [VERIFICATION.md](VERIFICATION.md). [BUGS.md](BUGS.md) B-001 is resolved.
- [x] CI builds and invokes packaged-JAR probes
- [x] Crate-level `//!` docs on all five crates
- [x] `publish = false` on all six manifests — **SC-01**, `315fe87`
- [x] Toolchain pin + declared MSRV — **SC-02**, **SC-03**; `rust-toolchain.toml` pins
      `1.96.0` and `[workspace.package] rust-version = "1.96"` reaches all six crates. The
      run 32386474212 recorded the configured toolchain/packaging jobs at `3b64484`
- [x] `fmt` and `clippy` are workflow steps — **SC-04**, `dd080b2`
- [x] Java no-argument test execution uses automatic `*Test.class` discovery — **VER-03**.
      There are 19 source files: 18 no-argument probes and one explicitly parameterized broker
      probe.
- [x] Crate-name collisions resolved — **SC-05**, **SC-06**; packages are `jni-bridge` and
      `modernlink-core`, folders unchanged, native artifact still `modernlink`
- [x] Rust behavior-crate and Java production-class 90% line thresholds are wired in the dirty
      workflow.
- [ ] Record terminal current-branch workflow conclusions for both thresholds. Java recorded
      90.33% locally; the Rust gate and both branch workflow results remain pending.

## Test coverage

The current dirty tree records three deliberately distinct coverage scopes:

| Surface | Raw local report | Threshold wired in workflow |
|---|---:|---:|
| Full Rust production source, including JNI ABI glue and demo CLIs (informational) | 2,548/3,071 — **82.97%** before latest redirect/fault additions | none |
| Rust production behavior crates (`core`, `http`, `messaging`, `tls`) | Linux branch result pending | 90% |
| Java production classes under JaCoCo/JDK 8, using Java 6-targeted classes and JNI→NATS | 803/889 — **90.33%** | 90% |

The Rust reports are not Rust-only unit-test figures. `scripts/run_rust_coverage.sh` instruments
`libmodernlink`, loads it from Java, runs unit tests and demo binaries, and exercises
NATS, JetStream, RabbitMQ, Kafka, and Pulsar sequentially, emits a full report, then applies
`--fail-under-lines 90` to the behavior-crate scope. JNI entry points and provider dispatch stay
visible in the full report. The Java job measures the facade independently with JaCoCo.

These are dirty-tree local facts. The workflow code exists, but neither 90% gate has a terminal
GitHub Actions result at this revision yet. Exact commands, runtime reach, and remaining gaps are
maintained in [VERIFICATION.md](VERIFICATION.md); provider durability, reconnect, ordering under
load, and vendor-host compatibility remain outside what either percentage establishes.

## Overall

| Phase | State |
|---|---|
| 0 — Native boundary and packaging | complete, unvalidated |
| 1 — HTTPS and TLS | complete, unvalidated |
| 2 — Messaging transports | implemented, limited happy-path runtime evidence |
| 3 — M1 compatibility scope | not started |
| 4 — M2 routing and migration | not started |

Roughly **two of five phases** are code-complete; none is validated against the vendor product.
The single highest-value next step is a current-branch run of the newly wired Rust and Java line
gates. After that machine result is recorded, B-003 delivery-mode enforcement and vendor-host
JMS compatibility remain the highest-value contract work.
