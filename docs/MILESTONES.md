# Milestones
<!-- rev:013 (RFC 3339) 2026-08-19T00:00:00Z -->

Version milestones for ModernLink. **No git tags exist yet** — nothing has been released, so
every version below is a target, not a shipped artifact. Phases are detailed in
[ROADMAP.md](ROADMAP.md); tasks in [IMPLEMENTATION_TASKS.md](IMPLEMENTATION_TASKS.md).

Throughout: "done" means the code exists **and** evidence exists. Phase 0-1 now have runtime
evidence on a real Java 6 JVM; Phase 2 has one hand-run round trip for three of five providers
and nothing reproducible in CI. v0.1.0 is close but not reached — a toolchain pin, a declared
MSRV and any coverage number are still missing.

## v0.1.0 — Provable native boundary `[IN PROGRESS]`

The first version that can honestly claim anything. Goal: the HTTPS path is *demonstrated*, not
just implemented.

- [x] Rust workspace with `core` / `http` / `tls` / `messaging` / `jni`
- [x] Java 6 facade at `-source 1.6 -target 1.6`
- [x] Single-JAR packaging with three platform natives
- [x] HTTPS facade with TLS 1.2/1.3, redirects, certificates, capability bitmask
- [x] Crate-level `//!` documentation
- [x] CI: `cargo test --workspace`, `cargo check -p jni-bridge`, `fmt`, `clippy`, JAR build + all
      13 Java tests — **all green** on run
      [31781200582](https://github.com/inovacc/modernlink/actions/runs/31781200582) (2026-08-14),
      which resolved [BUGS.md](BUGS.md) B-001.
- [x] `publish = false` on all six manifests — **SC-01**, `315fe87`
- [x] Toolchain pin and declared MSRV — **SC-02**, **SC-03**. `rust-toolchain.toml` +
      `rust-version = "1.96"`; verified locally by `rustup show active-toolchain` and
      `cargo metadata`, not yet by a CI run or a Docker build
- [x] `fmt` + `clippy` enforced in CI — **SC-04**, `dd080b2`; both ran and passed on run
      [31782837766](https://github.com/inovacc/modernlink/actions/runs/31782837766) at `d2479bd`
- [x] All **15** Java test classes enumerated in CI — **VER-03**, `dd080b2`; the 13 that existed
      at the time executed and
      passed on run 31781200582.
- [x] A recorded **Java 6** run against a live HTTPS endpoint — **VER-04**. Run 31781200582
      executed the suite on a real Java 6 JVM (`native-smoke-jvm=1.6.0_38`, `Linux/amd64`) from
      the packaged JAR, reaching `tls-protocol=TLSv1_3` against a live endpoint. The earlier local
      record ([docs/evidence/2026-08-14-native-runtime.md](evidence/2026-08-14-native-runtime.md),
      `status=200`, 4 peer certs) used JVM 21 and covered the JNI boundary only; CI supplies the
      Java 6 half.
- [~] Native-load smoke test per platform — **VER-05**. **windows-x86_64** (local, JVM 21) and
      **linux-x86_64** (CI, `native-smoke-load=ok` on JVM 1.6.0_38) both load. A CI job now
      targets **linux-aarch64** on an arm64 runner, but **it has never executed**, so that
      native remains unloaded on any JVM until it does.
- [x] A working coverage measurement — **15.20% regions / 17.07% lines**
      (`cargo llvm-cov --workspace --all-features`, 2026-08-19). Unblocked by SC-07; see
      [ROADMAP.md](ROADMAP.md). Measured, **not gated**

**Coverage target:** establish a baseline at all — **met**: 17.07% lines. That is a starting
line, not a passing grade, and nothing enforces it yet.

## v0.2.0 — Messaging with evidence `[BLOCKED on v0.1.0]`

Converts Phase 2 from claim to fact. This is the milestone that matters most: the transports are
already written, so the entire value of this release is proof.

- [~] Broker fixtures in CI for NATS, Kafka, Pulsar, RabbitMQ — **VER-01**. A `Broker-backed
      messaging` job now stands up NATS (JetStream) and RabbitMQ and runs the three `#[ignore]`d
      tests; **Kafka and Pulsar are absent** because they have no test yet, and **the job has
      never run** — unproven until it does
- [~] Broker-backed send / receive / acknowledge test per provider — **VER-02**. NATS core,
      JetStream and RabbitMQ pass (2026-08-14, verified by Codex). Kafka and Pulsar now have
      tests too, but **neither has ever been executed** against a live broker. Only the happy
      path is covered for any provider.
- [x] Per-adapter guarantee declarations, queryable before traffic moves — **MSG-04**
- [x] Documented per-provider guarantees — **DOC-03**, [providers.md](providers.md)
- [~] Payload categories beyond text — **MSG-05**. TEXT, BYTES and MAP carried; STREAM and
      OBJECT deliberately refused (see [providers.md](providers.md) and ROADMAP)
- [ ] Versioned envelope schema — **MSG-01**
- [ ] Documented field mappings across all five providers — **MSG-02**
- [ ] Unsupported mappings fail at configuration time — **MSG-03**

**Coverage target:** ≥ 60% on `crates/messaging`, the crate carrying six transports in one file.

## v0.3.0 — JMS compatibility surface `[BLOCKED on JMS-01]`

Cannot start until the vendor's actual JMS version and interfaces are known
([ISSUES.md](ISSUES.md) I-011). That inventory is research, not code, and it gates everything.

- [ ] Vendor JMS version + interface inventory — **JMS-01**
- [ ] Façade strategy ADR: `javax.jms` vs `com.modernlink.jms` vs vendor adapter — **JMS-02**
- [ ] API compatibility matrix covering every supported method and semantic — **JMS-03**
- [ ] Java 6 application-server class-loading and packaging model — **JMS-04**
- [ ] JNDI lookup compatibility — **JMS-05**
- [ ] Transactions, selectors, rollback / redelivery, dead-letter — **JMS-06**
- [ ] Broker-backed transparent pass-through prototype — **JMS-07**
- [ ] Full JMX management model with read-only / mutating separation — **JMX-01**…**JMX-04**

**Coverage target:** ≥ 70% workspace.

## v0.4.0 — Routing, transform, migration `[NOT STARTED]`

The M2 scope. Only meaningful once transparent mode is real.

- [ ] Routing policy configuration — **RT-01**
- [ ] Dry-run proven distinct from apply — **RT-02**
- [ ] Versioned auditable policy — **RT-03**
- [ ] No silent acknowledgement on target failure — **RT-04**
- [ ] Transform envelope with replay controls — **RT-05**…**RT-07**
- [ ] Migration and rollback controls, observable via JMX — **RT-08**, **RT-09**

**Coverage target:** ≥ 75% workspace.

## v1.0.0 — Production-usable against the locked product `[NOT STARTED]`

The bar is the one the README already sets and nothing has yet met: validation with the Java 6
runtime, the target platforms, and real services.

- [ ] The host product runs against ModernLink in a representative environment
- [ ] Every provider adapter declares and demonstrates its guarantees
- [ ] Transparent mode proven not to alter delivery semantics
- [ ] Cutover and rollback rehearsed end to end
- [x] Crate-name collisions resolved — **SC-05**, **SC-06**
- [ ] Non-deprecated Java 6 build image — **VER-06**
- [ ] Versioning policy for the Java API and native contract

**Coverage target:** ≥ 80% workspace, the standing target.

## Release status

| Version | Tag | State |
|---|---|---|
| v0.1.0 | — | in progress |
| v0.2.0 | — | blocked on v0.1.0 |
| v0.3.0 | — | blocked on JMS-01 |
| v0.4.0 | — | not started |
| v1.0.0 | — | not started |

`.github/workflows/` carries release-drafter, release-note, snapshot, and release workflows, so
the release machinery exists ahead of anything to release.
