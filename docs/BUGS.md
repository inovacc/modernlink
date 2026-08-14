# Bugs
<!-- rev:002 (RFC 3339) 2026-08-14T02:22:19Z -->

Behaviour that is **wrong and should be fixed**. Deliberate constraints belong in
[ISSUES.md](ISSUES.md); planned work belongs in [BACKLOG.md](BACKLOG.md).

| ID | Severity | Status | Summary |
|---|---|---|---|
| B-001 | high | open | CI `Rust workspace` job cannot build `rdkafka-sys`; the Rust suite never runs in CI |
| B-002 | high | open | The routing policy engine is unreachable from Java — every `RouteConfig` is built with zero rules |

## Open

### B-001 — CI `Rust workspace` job fails before reaching the tests

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

### B-002 — the routing policy engine cannot be reached from the Java facade

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

This is not a claim that the rest of the code is correct. Beyond B-001 and B-002, **no failing
test, crash, or incorrect-output case has been observed and written down**. The honest state of
verification:

- `cargo test --workspace` is CI-gated on ubuntu, **but that gate is currently failing before the
  tests run (B-001)** — so the Rust suite is *not* being exercised in CI. It has been observed to
  pass locally on Windows; that is a machine result on one platform, not proof of correctness.
- The Java facade is only compiled and run inside `docker/java6/Dockerfile`; CI executes three
  of the ten test classes (`LegacyHttpResponseStructuredTest`, `ModernHttpsURLConnectionTest`,
  `LegacyHttpsTest`).
- **No broker-backed messaging behavior has ever been exercised.** Everything in
  `crates/messaging` beyond `InMemoryTransport` is a source-level claim — a defect there would
  not currently be detected by anything. See ISSUES I-010.
- No run against the real Java 6 host product has been recorded.

A bug found in those unexercised areas belongs here the moment it is observed.

## Resolved

None recorded.

## How to file

Give each entry an ID (`B-001`), a severity (critical / high / medium / low), the observed vs
expected behaviour, the exact reproduction command, and the commit at which it was seen. A
regression test must fail against the pre-fix code before the fix is accepted.
