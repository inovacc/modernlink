# Bugs
<!-- rev:013 (RFC 3339) 2026-08-19T00:00:00Z -->

Behaviour that is **wrong and should be fixed**. Deliberate constraints belong in
[ISSUES.md](ISSUES.md); planned work belongs in [BACKLOG.md](BACKLOG.md).

| ID | Severity | Status | Summary |
|---|---|---|---|
| B-001 | high | **resolved** `dd080b2` | CI `Rust workspace` job could not build `rdkafka-sys`; the Rust suite never ran in CI |
| B-002 | high | **resolved** `ad4bd2f` | The routing policy engine was unreachable from Java — every `RouteConfig` was built with zero rules |
| B-003 | high | **open** | `MessageEnvelope.delivery_mode` defaults to `Persistent` and is read by no transport; RabbitMQ publishes transient messages to a durable queue |
| B-004 | critical | **open** | No `catch_unwind` on any of the 28 `Java_*` entry points; a Rust panic unwinds into JVM frames (UB) |
| B-005 | high | **open** | The native messaging handle is a raw pointer dereferenced with only a null check |
| B-006 | high | **open** | Broker URLs carry credentials into error strings that cross into Java exception messages |
| B-007 | medium | **resolved** | A contained panic can leave a transport permanently degraded while the client still looks open |

## Open

### B-004 — a Rust panic can unwind across the JNI boundary into the JVM — **CRITICAL**

- **Severity:** critical — this is the failure the product exists to prevent. ModernLink's
  premise is talking to modern brokers *without destabilising a vendor-locked Java 6
  application*; unwinding a Rust panic into JVM frames is undefined behaviour.
- **Observed:** `crates/jni/src/lib.rs` exports **28** `pub extern "system" fn Java_*` entry
  points and contains **0** occurrences of `catch_unwind`. No manifest sets
  `[profile] panic`, so the cdylib builds with the default `panic = "unwind"`.
  `docs/adr/0001-jni-boundary-over-sidecar.md` already lists "A native failure can terminate
  the JVM" as an accepted risk of embedded JNI — accepting a risk is not containing it.
- **Precisely what is and is not claimed:** the *reachable-today* panic surface is thin.
  `crates/http/src/lib.rs:43` (`location.as_ref().unwrap()`) is guarded by an `is_none()`
  early return at `:37`, and the 41 `values[N]` expressions in `crates/jni` index fixed-size
  array literals with statically known length. **The defect is the absence of containment,
  not a specific live crash.** Any panic from `rustls`, `hyper`, `rdkafka`, `lapin`,
  `pulsar`, `serde`, an allocation failure, or a future edit crosses the boundary unguarded.
- **Expected:** every entry point catches unwinding, records the payload through the existing
  `set_error` channel, and returns the type's error sentinel.
- **Reproduction:** `rg -c 'catch_unwind' crates/jni/src/lib.rs` -> 0;
  `rg -c 'pub extern "system" fn Java_' crates/jni/src/lib.rs` -> 28.
- **Seen at commit:** `a32e1dd`.
- **A regression test must fail against the current code:** add an entry point whose body
  panics and assert the sentinel is returned rather than the process dying.

### B-005 — the native messaging handle is dereferenced without validation

- **Severity:** high — a use-after-free inside the host JVM.
- **Observed:** `crates/jni/src/lib.rs:58` — `unsafe fn messaging_client<'a>(handle: jlong)`
  turns a caller-supplied `jlong` into a reference with only a null check.
  `Box::into_raw` at `:501`/`:564`, `Box::from_raw` at `:790`/`:1290`. The Java side guards
  the happy path (`close()` is `synchronized` and zeroes the field; `requireOpen()` rejects
  0), but a stale, copied or fabricated `long` reaches `&*(handle as *const _)` unchecked.
- **Expected:** a handle that is not live is refused, not dereferenced.
- **Reproduction:** call any native method with an arbitrary non-zero `long`.
- **Seen at commit:** `a32e1dd`.
- **Candidate fix:** a registry mapping opaque ids to boxed clients, which also makes a
  leaked client (B-005's sibling, an unclosed handle) enumerable. A magic/generation word is
  the cheaper alternative.

### B-006 — broker credentials are not redacted from errors that reach Java

- **Severity:** high — AGENTS.md: *"Never put credentials, payloads, or message bodies in
  JMX attributes or logs."*
- **Observed:** the documented RabbitMQ endpoint form is
  `amqp://guest:guest@127.0.0.1:5672/%2f` (`crates/messaging/tests/broker_backed.rs:59`).
  `RabbitMqTransport::connect` passes it to `Connection::connect` and maps every failure with
  `.map_err(|error| DomainError::Transport(error.to_string()))`
  (`crates/messaging/src/lib.rs:547,551,555`). That string crosses the JNI boundary and
  becomes a `LegacyHttpException` message the host application will log. Nothing on that path
  redacts. The same pattern applies to all five transports.
- **Not yet confirmed:** whether `lapin`'s error `Display` actually embeds the URI. That
  needs a real failed connect against a broker. What *is* confirmed is that the path is
  unredacted by construction.
- **Expected:** userinfo is stripped (`scheme://***@host:port`) from any error built from a
  connection failure.
- **Reproduction:** trigger a failed connect with a credentialed URI and inspect the message.
- **Seen at commit:** `a32e1dd`.
- **Regression test:** assert a URI containing `user:password@` never appears in the
  `Display` of the resulting error.

### B-007 — a contained panic leaves the client alive and permanently broken

- **Severity:** medium — strictly better than the undefined behaviour it replaces, and still
  wrong. Found by Codex while adversarially reviewing the B-004 fix, and verified directly.
- **Observed:** `NatsTransport::receive` (`crates/messaging/src/lib.rs:620-642`) `.take()`s
  the subscription out of its `Mutex`, awaits `subscription.next()` inside
  `runtime.block_on`, and only then `.replace()`s it. If anything unwinds between the take
  and the replace, the subscription is dropped and the `Mutex` is left holding `None`
  **for the life of the client**. Every later `receive` then fails with "NATS subscription
  is unavailable" on a handle that still reports itself open. The `Mutex` is *not* poisoned,
  because the guard is released before the await — so Rust's own protection does not fire.
- **Why it appears now:** before B-004's fix that panic was undefined behaviour crossing into
  the JVM, i.e. usually a crash. Containing it converts a crash into a silent permanent
  degradation. That is an improvement and a new failure mode, and both should be said out
  loud.
- **Expected:** either the operation is panic-safe (restore on unwind rather than after the
  happy path), or the client fails closed on reuse after a contained panic instead of
  reporting a misleading "unavailable".
- **Reproduction:** read `crates/messaging/src/lib.rs:620-642`; the take at `:621-628`, the
  await at `:633-638`, the replace at `:639-642`.
- **Seen at commit:** the B-004 fix on `harden/h-01-catch-unwind`.
- **RESOLVED** by `RestoreOnDrop` (`crates/messaging/src/lib.rs`), fix (a) below. Both the
  core NATS and the JetStream receive paths now restore through `Drop`, so the happy path and
  the unwinding path take the same route. **The JetStream path had the same defect** — B-007
  suspected it and it was confirmed while fixing. Falsified: making `Drop` skip the restore
  makes `a_panic_between_take_and_replace_still_restores_the_value` fail. A structural test
  fails if any receive path goes back to restoring by hand.
- Kafka, Pulsar and RabbitMQ were checked: their pending-acknowledgement maps are mutated
  under a lock without an await in between, so they do not have this window.
- **Candidate fixes:** (a) hold the restore in a guard whose `Drop` puts the subscription
  back, so an unwind cannot skip it — contained and local; (b) mark the client poisoned in
  `jni_guard` and refuse later calls, which needs the handle registry from **B-005** to do
  properly. (a) is the cheaper fix and does not wait on B-005.
- **The same shape may exist in the other transports** — JetStream, RabbitMQ, Kafka and
  Pulsar all follow a take/operate/replace pattern around pending acknowledgements. Not
  audited yet; do that with the fix.

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
- The Java facade is compiled and run only inside `docker/java6/Dockerfile`. All **15** test
  classes are enumerated in the workflow. 13 of them executed and passed on run 31781200582;
  `ProviderGuaranteesTest` and `PayloadCategoriesTest` were added afterwards and **have never
  been compiled by `javac -source 1.6` or executed**.
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
