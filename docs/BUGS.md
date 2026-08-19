# Bugs
<!-- rev:009 (RFC 3339) 2026-08-19T00:00:00Z -->

Behaviour that is **wrong and should be fixed**. Deliberate constraints belong in
[ISSUES.md](ISSUES.md); planned work belongs in [BACKLOG.md](BACKLOG.md).

| ID | Severity | Status | Summary |
|---|---|---|---|
| B-001 | high | **resolved** `dd080b2` | CI `Rust workspace` job could not build `rdkafka-sys`; the Rust suite never ran in CI |
| B-002 | high | **resolved** `ad4bd2f` | The routing policy engine was unreachable from Java — every `RouteConfig` was built with zero rules |
| B-003 | high | **open** | `MessageEnvelope.delivery_mode` defaults to `Persistent` and is read by no transport; RabbitMQ publishes transient messages to a durable queue |

## Open

### B-003 — `MessageEnvelope.delivery_mode` is set by every message and read by no transport

- **Severity:** high — this is a silently degraded delivery guarantee, which
  [AGENTS.md](../AGENTS.md) names as non-negotiable: *"Delivery semantics are part of the
  contract, not an implementation detail. A capability gap must be reported explicitly —
  never silently degraded."* A caller is told its message is persistent; nothing makes it so.
- **Observed:** `MessageEnvelope::new` sets `delivery_mode: DeliveryMode::Persistent`
  (`crates/messaging/src/lib.rs:189`), so **every** message defaults to persistent. A grep
  for `delivery_mode` across the crate returns three hits: the field declaration, that
  default, and one unit-test assertion. **No transport reads it.** Kafka, Pulsar, NATS,
  JetStream, RabbitMQ and the in-process transport all publish without consulting it.
- **The RabbitMQ case is the sharpest.** The queue is declared `durable: true`
  (`crates/messaging/src/lib.rs`, `QueueDeclareOptions { durable: true, .. }`), but the
  publisher passes `BasicProperties::default()`, which is AMQP `delivery_mode` 1 —
  transient. A durable queue holding transient messages **loses them on broker restart**.
  The queue looks durable in the management UI while the messages are not, which is worse
  than an obviously non-durable queue because it survives review.
- **Expected:** either the transport honours the requested delivery mode, or requesting a
  mode it cannot honour is refused. Both are acceptable; silently accepting and ignoring is
  not.
- **Reproduction:** `rg -n 'delivery_mode' crates/` → 3 matches, none in a publish path.
  For RabbitMQ specifically, publish with the default envelope, restart the broker, and the
  message is gone despite `DeliveryMode::Persistent`.
- **Seen at commit:** `ba1f7eb`.
- **Not fixed here, deliberately.** MSG-04 added `ProviderGuarantees::require_delivery_mode`,
  which *can* fail closed on this, and it is **not wired into the publish path**. Wiring it
  in would immediately start refusing every default NATS-core publish, because the default
  is `Persistent` and core NATS cannot persist — a change to delivery semantics on the
  default path. **Only the maintainer should make that call**, and making it silently while
  fixing something else is the same class of error as the bug itself.
- **Two candidate fixes, and they are not exclusive:**
  1. *Honour it where possible* — RabbitMQ sets `delivery_mode` 2 when the envelope says
     `Persistent`; Kafka and Pulsar are persistent by construction; JetStream already is.
     This is a small, contained change per transport.
  2. *Fail closed where impossible* — call `require_delivery_mode` before publishing, so
     core NATS refuses a persistent message instead of accepting it. This changes behaviour
     on the default path and needs the default itself reconsidered (`Persistent` is a poor
     default for a layer that fronts a provider which cannot offer it).
- **Recorded in the guarantee table meanwhile:** `Provider::RabbitMq.guarantees().persistence`
  is `UNSUPPORTED`, not `DECLARED`, and `docs/providers.md` says why. The table records the
  behaviour, not the intent — a unit test
  (`rabbitmq_persistence_records_the_behaviour_not_the_intent`) pins that, and will fail if
  the publisher is fixed without updating the table.


## Resolved — B-001

### B-001 — CI `Rust workspace` job failed before reaching the tests — **RESOLVED**

**Resolved by `dd080b2`, confirmed by run
[31781200582](https://github.com/inovacc/modernlink/actions/runs/31781200582) on 2026-08-14** —
the first execution of the fixed workflow, after the branch was pushed. Both jobs succeeded, and
the per-step detail answers the question the bug actually asked — *did it reach the tests?*

```
Rust workspace:
  4. Install native build dependencies -> success
  5. Test workspace                    -> success   <- previously died before this step
  6. Check JNI crate                   -> success
  7. Format                            -> success
  8. Lint                              -> success
Java 6 JAR integration                 -> success
```

The fix was the symptom-level one: install `libcurl4-openssl-dev`, `libsasl2-dev`, `cmake` and
`protobuf-compiler` in the job. **The root layer is now fixed too (SC-07):** `crates/messaging`
declares `[features]` with `default = []`, so a broker-free build compiles no provider client at
all. `cargo tree -p jni-bridge` shows **0** matches for rdkafka/pulsar/lapin/async-nats by default
and **5** with `--features all-providers`. The apt packages are still installed, but only the
`--all-features` half of the job needs them.

The original report follows.

### The original report

- **Severity:** high — this is a broken verification gate, not just a red build. Several docs
  cited this job as proof the Rust suite is exercised, so the failure was manufacturing false
  confidence rather than removing it.
- **Observed:** every push to `main` fails the `CI` workflow's `Rust workspace` job.
  `cargo test --workspace` builds `rdkafka-sys 4.10.0+2.12.1`, which compiles librdkafka from
  source via cmake, and dies at
  `librdkafka/src/rdkafka_conf.c:60:10: fatal error: curl/curl.h: No such file or directory`
  → cmake exit 2 → build-script panic → `##[error]Process completed with exit code 101`.
  **The test suite is never reached.** The log also reports
  `Package 'libsasl2', required by 'virtual:world', not found`, which may be a second missing
  package behind the first (unconfirmed — the curl failure aborts first).
- **Expected:** the job builds the workspace and runs the tests.
- **Reproduction:** push to `main`, or `gh run view 31758767603 --log-failed`. Seen on five
  consecutive runs — 31758767603, 31757479422, 31757440099, 31755795828, 31754951217.
- **Seen at commit:** `af02427` (and the four before it).
- **Not reproducible locally:** on Windows with `cmake` and `protoc` present, the same command
  exits 0 (20 tests passed, 15m55s). This is an environment gap on the runner, not a code defect.
- **Two candidate fixes, neither verified. Prefer the second:**
  1. *Symptom fix* — `.github/workflows/test.yml:10-19` installs no system packages, while
     `docker/java6/Dockerfile:8` already installs `cmake libcurl4-openssl-dev
     protobuf-compiler`. Copying that into the CI job would likely make it build.
  2. *Root-layer fix* — `crates/messaging/Cargo.toml` declares **no `[features]`**, so
     `rdkafka` (`cmake-build`), `pulsar`, `lapin` and `async-nats` are all unconditional
     dependencies. That means every `cargo test`, `cargo clippy`, and `cargo llvm-cov` over the
     workspace must compile a native Kafka client, for tests that touch no broker at all.
     Feature-gating the providers fixes this CI failure **and** the Windows `llvm-cov` failure
     recorded in ROADMAP's coverage section, in one change. The apt fix addresses only the first.
- **Decide the root layer before patching the symptom.** A broker-free unit-test run should not
  require a native Kafka toolchain on every machine that touches the repo.

This is not a claim that the rest of the code is correct. Beyond B-001, **no failing test,
crash, or incorrect-output case has been observed and written down**. The honest state of
verification is recorded under "Verification reach" below.

## Resolved

### B-002 — the routing policy engine could not be reached from the Java facade

**Resolved in `ad4bd2f`.** `crates/jni/src/lib.rs:354` adds `nativeOpenRouted`, which parses a
rule set at `:379-393`; `:447` adds `nativeDryRun` for policy evaluation without publishing.
The Java side gained `ModernRouteRule`, `ModernRouteDecision`, factory/client overloads and
`ModernConnection.evaluateRoute`. `RoutingPolicyTest` covers default fallthrough, exact and
prefix matching, tenant constraints, denial-by-rule, first-match-wins and the rejection paths.

**Falsified before acceptance:** forcing `rules` back to `Vec::new()` made the test fail with
`expected exact-hit, got null`; reverting made it pass. The regression test does fail against
the pre-fix code, so it is real evidence rather than a test that cannot fail.

The original report follows, unedited, because a resolved bug's history is the useful part.

- **Severity:** high — a documented capability is not wired to its only caller. This is a
  partial-delivery defect: the engine was built, the surface that exposes it was not.
- **Observed:** `crates/jni/src/lib.rs:217-221` constructs every `RouteConfig` as
  `{ default_mode, default_provider, rules: Vec::new() }` — the rule list is hardcoded empty on
  every connection. A grep for `RouteRule`, `RouteConfig`, `dry_run` across `java/` returns
  **0 matches in 0 files**. So rule matching, allow/deny predicates, dry-run evaluation and
  `rule_id` receipts exist in `crates/messaging` but no Java 6 caller can configure or observe
  any of them.
- **Expected:** either the JNI boundary accepts a rule set from Java, or the docs stop claiming
  routing "with policy" as delivered.
- **Reproduction:** read `crates/jni/src/lib.rs:217-221`; run
  `rg -c 'RouteRule|RouteConfig|dry_run' java/` → no matches.
- **Seen at commit:** `af02427`.
- **Doc impact:** `docs/ROADMAP.md` previously carried this as `[x] Routing dispatch with
  policy…`; corrected to `[~]` in the same revision that filed this bug. Follow-on work is
  tracked as **MSG-06** in [IMPLEMENTATION_TASKS.md](IMPLEMENTATION_TASKS.md).

## Verification reach

What the suites actually cover, so absence of bugs is not mistaken for evidence of correctness:

- `cargo test --workspace`, `cargo check -p jni-bridge`, `cargo fmt --all -- --check` and
  `cargo clippy … -D warnings` **all passed in CI** on ubuntu, run
  [31781200582](https://github.com/inovacc/modernlink/actions/runs/31781200582), 2026-08-14 —
  and the Rust job reached its test step rather than dying in a build step. Also observed exit 0
  locally on Windows (Codex-run).
- **The Java 6 runtime is now exercised.** In the same run, all **13** test classes ran from the
  packaged JAR on a real Java 6 JVM — `native-smoke-jvm=1.6.0_38`, `Linux/amd64` — including
  `native-smoke-load=ok` (so the **linux-x86_64** native loads under Java 6),
  `tls-protocol=TLSv1_3` for live HTTPS, `legacy-jms-messaging=PASS` and `routing-policy=PASS`.
  That also demonstrates the new test classes are genuinely Java 6-compatible: they compiled
  under `javac -source 1.6 -target 1.6`.
- **Still not exercised:** `linux-aarch64` has never been loaded on any JVM. A CI job now
  targets it on an arm64 runner; that job has not run, so the native is still unproven. **The Kafka and
  Pulsar broker-backed tests have never been executed** — they exist
  (`crates/messaging/tests/broker_backed_{kafka,pulsar}.rs`) and a CI job invokes them, but no
  run has been recorded, so they are code, not evidence.
- **Broker-backed messaging has now been exercised, for three providers — but only by hand.**
  All three tests are `#[ignore]`d (`crates/messaging/tests/broker_backed.rs:117,132,149`), so
  **no CI run has ever executed one**; the evidence below is an operator's manual run against
  local containers, which is VER-01. On 2026-08-14
  `cargo test -p messaging --test broker_backed -- --ignored` exited 0 with
  `nats_core_send_receive_ack`, `nats_jetstream_send_receive_ack` and `rabbitmq_send_receive_ack`
  all passing against live NATS/JetStream/RabbitMQ. That retires the blanket "no broker-backed
  behavior has ever been exercised" claim — **but only for send/receive/ack on those three.**
  Kafka and Pulsar still have no broker-backed test, and durability, reconnect, ordering,
  concurrency, failure and redelivery remain unexercised everywhere. See ISSUES I-010.
- The Java facade is compiled and run only inside `docker/java6/Dockerfile`. All **13** test
  classes are enumerated and, as of run 31781200582, all 13 executed and passed.
- `cargo fmt --all -- --check` → **exit 0**, both on the runner (run 31782837766) and locally
  (2026-08-19, verified independently by Codex). The earlier three diffs in
  `crates/messaging/tests/broker_backed.rs` were fixed before that file was committed in
  `a2419b5`, so the gate now sees the file and passes on it.
- **No run against the real vendor host product** has been recorded. Java 6 the *runtime* is now
  exercised; the locked *product* the layer exists to serve is not, and neither is its JMS
  implementation (I-009, I-011).

A bug found in the unexercised areas belongs here the moment it is observed.

## How to file

Give each entry an ID (`B-001`), a severity (critical / high / medium / low), the observed vs
expected behaviour, the exact reproduction command, and the commit at which it was seen. A
regression test must fail against the pre-fix code before the fix is accepted.
