# Roadmap
<!-- rev:013 (RFC 3339) 2026-08-19T00:00:00Z -->

Status at HEAD `d2479bd` on `main` (pushed; in sync with `origin/main`). Phases follow the M1/M2
structure in [BACKLOG.md](BACKLOG.md); tasks are broken out in
[IMPLEMENTATION_TASKS.md](IMPLEMENTATION_TASKS.md).

**Read the qualifier first:** `[x]` here means *the code exists and compiles*. It does **not**
mean the behavior was validated against the Java 6 host product or the vendor's own JMS
implementation — that has never been recorded, see [ISSUES.md](ISSUES.md) I-009/I-011. The Java 6
*runtime* and two of three platforms **are** now exercised, and three of five brokers have one
recorded round trip; the reach of each is stated inline. `[~]` means partially delivered, with
the gap named.

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
- [~] Native-load smoke test per platform resource — **VER-05**. windows-x86_64 (local, JVM 21)
      and linux-x86_64 (CI, JVM 1.6.0_38) both load. A `linux-aarch64 native load` job now runs
      `NativeLoadSmokeTest` on an `ubuntu-24.04-arm` runner against the JAR built by the amd64
      job, and asserts the platform line really reports `aarch64`. **That job has never run**
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
- [x] Routing dispatch with policy, provider-mismatch rejection and auditable receipt —
      **reachable from Java** as of `ad4bd2f` (**MSG-06**, closes [BUGS.md](BUGS.md) B-002).
      `nativeOpenRouted` accepts a rule set (`crates/jni/src/lib.rs:354`, parsing at `:379-393`)
      and `nativeDryRun` (`:447`) evaluates policy without publishing; `ModernRouteRule` /
      `ModernRouteDecision` / `ModernConnection.evaluateRoute` expose it, covered by
      `RoutingPolicyTest`. Falsified before acceptance: forcing `rules: Vec::new()` back made the
      test fail, reverting made it pass.
- [x] Read-only JMX metrics MBean, Java 6-compatible
- [x] Contract fixtures under `hacks/` for publisher/consumer across providers
- [~] **Broker fixtures in CI** — **VER-01**. A `Broker-backed messaging` job in
      `.github/workflows/test.yml` starts `nats:2.10 -js` and `rabbitmq:3.13`, waits on their
      logs, and runs the three `#[ignore]`d tests explicitly, asserting all three actually ran.
      **Kafka and Pulsar are not in it** — they have no broker-backed test (VER-02). **The job
      has never executed**: a workflow edit cannot be verified locally, so this is implemented,
      not proven.
- [~] **Broker-backed send/receive/ack test per provider** — **VER-02**. A test now exists for
      all five. **Proven for three**: NATS core, JetStream and RabbitMQ passed against live
      brokers 2026-08-14 (`tests/broker_backed.rs`, Codex-verified). **Written but never executed
      for two**: Kafka (`tests/broker_backed_kafka.rs`) and Pulsar
      (`tests/broker_backed_pulsar.rs`) — a test nobody has run is not evidence. All five are
      `#[ignore]`d and driven by dedicated CI jobs, neither of which has run yet. One happy-path
      round trip only; durability, reconnect, ordering, concurrency and failure semantics remain
      unexercised for **every** provider.
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
- [x] CI running `cargo test --workspace` + `cargo check -p jni-bridge` — **passing** as of run
      [31781200582](https://github.com/inovacc/modernlink/actions/runs/31781200582) (2026-08-14),
      after five consecutive red pushes. The job now reaches and runs the tests instead of dying
      in the `rdkafka-sys` build. [BUGS.md](BUGS.md) B-001 resolved.
- [x] CI building and exercising the packaged JAR (the `Java 6 JAR integration` job passes)
- [x] Crate-level `//!` docs on all five crates
- [x] `publish = false` on all six manifests — **SC-01**, `315fe87`
- [x] Toolchain pin + declared MSRV — **SC-02**, **SC-03**; `rust-toolchain.toml` pins
      `1.96.0` and `[workspace.package] rust-version = "1.96"` reaches all six crates. The
      CI and Docker halves are **unproven until a run** — neither was executed for this change
- [x] `fmt` and `clippy` enforced in CI — **SC-04**, `dd080b2`; both executed and passed on run
      [31782837766](https://github.com/inovacc/modernlink/actions/runs/31782837766) at `d2479bd`
- [x] All **15** Java test classes enumerated in CI — **VER-03**, `dd080b2` (was three of ten;
      the count changed because VER-08 added two messaging tests and VER-05 added the native
      smoke test)
- [x] Crate-name collisions resolved — **SC-05**, **SC-06**; packages are `jni-bridge` and
      `modernlink-core`, folders unchanged, native artifact still `modernlink`
- [x] Working coverage measurement — 17.07% lines / 15.20% regions, see below. **Not gated**

## Test coverage

**17.07% line coverage / 15.20% region coverage** across the workspace, measured 2026-08-19 with
`cargo llvm-cov --workspace --all-features --summary-only` on Windows, rustc 1.96.0.

This is the first coverage figure the project has ever had. It became measurable because **SC-07**
made the provider clients optional: `combine`, `lapin`, `pulsar` and `async-nats` used to fail
under coverage instrumentation, and llvm-cov could not compile the dependency graph at all.

| Crate | Regions | Lines |
|---|---|---|
| `crates/tls` | 90.48% | 87.10% |
| `crates/core` | 65.07% | 63.37% |
| `crates/messaging` | 23.91% | 25.98% |
| `crates/http` | 16.01% | 16.95% |
| `crates/jni` | 1.28% | 1.67% |
| `hacks/messaging-demo` (7 binaries) | 0.00% | 0.00% |
| **TOTAL** | **15.20%** | **17.07%** |

**Read these numbers with three caveats, or they will mislead:**

1. **`--all-features` is the honest run.** The default (broker-free) build reports
   **27.37% / 30.58%**, and `crates/messaging` alone reports **91.97%** — but only because the
   five transports are compiled out of that build entirely. The uncovered code is excluded
   rather than covered. Always quote the `--all-features` figure.
2. **`crates/jni` at 1.28% is not as bad as it looks, and not as good either.** That crate is
   exercised almost entirely by the 15 Java test classes running against the packaged JAR, which
   llvm-cov cannot see. What the figure does say is that no *Rust* test drives the JNI boundary.
3. **The `messaging` figure is where the real gap is.** 23.91% with the transports included,
   against 91.97% with them excluded, means the domain and routing logic are well covered and the
   five broker transports are close to untested — which is **VER-01/VER-02** restated as a
   measurement.

The Java facade has **no coverage tooling at all**; there is no Maven or Gradle build, so no
JaCoCo. Coverage is measured but **not gated** in CI — no threshold is enforced on any push.

Other verification facts:
- `cargo test --workspace` now runs and passes in CI on ubuntu-latest (run 31781200582), plus
  `check`, `fmt` and `clippy`. Also exit 0 locally on Windows. Two platforms, still machine
  results rather than a coverage figure.
- All 13 Java test classes ran and passed in CI from the packaged JAR on a **Java 6** JVM
  (`1.6.0_38`, `Linux/amd64`), including the native load, live HTTPS at TLSv1_3, the messaging
  facade and the routing policy.
- The Java facade has no coverage tooling at all — no Maven/Gradle means no JaCoCo.

Raising the number, and gating it, remain open — tracked as P2 in [BACKLOG.md](BACKLOG.md).

## Overall

| Phase | State |
|---|---|
| 0 — Native boundary and packaging | complete, unvalidated |
| 1 — HTTPS and TLS | complete, unvalidated |
| 2 — Messaging transports | implemented, no runtime evidence |
| 3 — M1 compatibility scope | not started |
| 4 — M2 routing and migration | not started |

Roughly **two of five phases** are code-complete; none is validated against the vendor product.
The single highest-value next step is **VER-01 → VER-02**: broker fixtures in CI, which convert
Phase 2 from a manual one-machine run into reproducible evidence. The cheapest remaining items
are **SC-02** and **SC-03** — a toolchain pin and a declared MSRV.
