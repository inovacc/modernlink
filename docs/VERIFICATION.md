# Verification reach
<!-- rev:005 (RFC 3339) 2026-08-21T19:43:45Z -->

This file records what commands and runtime paths have actually executed. A machine result is
reported as a fact about that command at that revision; it is not a verdict that ModernLink is
correct, production-ready, or compatible with the vendor host.

## Current repository state

- The current branch is `feat/integration-samples-coverage` at `e14b71f`; the working tree is
  dirty and has no GitHub Actions result at its current contents.
- The distributable Dockerfile now compiles all three native targets with
  `--features all-providers`. A local Docker build produced image `b62d47083b6f`; its packaged
  Java 6 (`1.6.0_38`) JAR completed a NATS publish/receive/client-ack probe with exit code 0.
- The 18 no-argument Java test classes in that image completed with exit code 0. The separate,
  parameterized `BrokerBackedMessagingTest` completed with exit code 0 against NATS in Java 6,
  and against NATS, JetStream, RabbitMQ, Kafka, and Pulsar on the host JVM through the
  instrumented Windows native library.
- After inline Rust unit bodies were removed from the denominator and profiles were cleaned, the
  full production-source report recorded 2,548/3,071 lines (82.97%) before the latest
  redirect/fault additions. The workflow's 90% Rust gate is explicitly scoped to production
  behavior crates; JNI ABI glue and demo CLIs remain in the full informational report.
- JaCoCo on JDK 8, against classes compiled for Java 6 and including the NATS provider probe,
  recorded 803/889 lines (90.33%). The packaged Java 6 runtime probe remains a separate command.
  These are local dirty-tree measurements, not current-branch workflow conclusions.

## Recorded machine runs

| Environment | Revision | Raw machine fact | Reach |
|---|---|---|---|
| GitHub Actions run [32386474212](https://github.com/inovacc/modernlink/actions/runs/32386474212) | `3b64484` | The Rust, Java 6 JAR, NATS/JetStream/RabbitMQ, Kafka/Pulsar, and linux-aarch64 jobs reported conclusion `success` | One configured happy-path send/receive/ack test per provider; packaged-JAR tests; native load on ARM64/JVM 21 |
| GitHub Actions run [31781200582](https://github.com/inovacc/modernlink/actions/runs/31781200582) | prior revision | The recorded Java job ran the packaged JAR on JVM `1.6.0_38`, including native load, live HTTPS, messaging, and routing probes | Java 6 runtime and linux-x86_64 at that revision; not the vendor product |
| Local Windows record, [2026-08-14-native-runtime.md](evidence/2026-08-14-native-runtime.md) | prior revision | JVM 21 loaded the Windows native, returned HTTP status 200 with four peer certificates, and completed NATS/JetStream/RabbitMQ round trips | Windows JNI/native path and those three brokers on one machine |
| Local dirty tree, 2026-08-21 | uncommitted | Docker build, 18 packaged Java 6 tests, packaged Java 6→JNI→NATS, host-JVM→JNI round trips for all five provider variants, full Rust production 82.97%, and Java 90.33% emitted their recorded results | Current local source and Docker runtime paths; the Rust behavior-crate 90% branch gate still needs a Linux branch result; no vendor host, production semantics, ARM64 rerun, or current-revision CI |

The current dirty workflow declares seven jobs, including a 90% production behavior-crate Rust
gate and a 90% Java production-class gate.
The dependency-audit step is deliberately
`continue-on-error`; its job status must not be read as absence of advisories. B-009 records the
known advisory output.

## Java facade reach

There are currently 19 Java test source files: 13 under `com.modernlink` and six under
`com.modernlink.messaging`. Eighteen are no-argument packaged-JAR probes; the nineteenth accepts
a provider, endpoint, and optional destination prefix and owns no broker lifecycle. The workflow
auto-discovers the first group and invokes the broker probe explicitly against NATS.

Java 6 syntax and runtime behavior are judged only by the Docker build. A local `javac -source 8`
run is a fast type check, not a Java 6 language/runtime result. The Rust coverage harness builds
an instrumented native library and loads it from Java so JNI-only Rust paths contribute profiles.

## Broker reach

The five broker tests are `#[ignore]`d, so neither `cargo test --workspace` nor
`cargo test --workspace --all-features` executes them. Dedicated workflow jobs invoke them
explicitly. Run 32386474212 recorded one send/receive/ack path for NATS core, JetStream,
RabbitMQ, Kafka, and Pulsar.

No recorded run exercises durability across restart, reconnect, ordering under load,
concurrency, timeout behavior, failure recovery, rollback, redelivery, or dead-letter behavior.
The provider guarantee table remains authoritative for what the code declares or refuses.

## Still unproven

- The vendor host product and its JMS implementation have never been part of a recorded run.
- `LEGACY_JMS` is an in-process compatibility transport, not a vendor-broker bridge.
- No run establishes production delivery semantics for any provider.
- The current dirty tree has no ARM64 rerun or GitHub Actions result; the new 90% gates have not
  executed from a clean runner.
- B-003 delivery-mode enforcement and the payload-output hard-rule deviation remain open.
- Only the maintainer decides whether the observed behavior satisfies the intended contract.
