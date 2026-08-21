# Verification reach
<!-- rev:007 (RFC 3339) 2026-08-21T20:55:36Z -->

This file records what commands and runtime paths have actually executed. A machine result is
reported as a fact about that command at that revision; it is not a verdict that ModernLink is
correct, production-ready, or compatible with the vendor host.

## Current repository state

- The current branch is `feat/integration-samples-coverage`; CI run
  [32523731422](https://github.com/inovacc/modernlink/actions/runs/32523731422) at `686adaa`
  recorded `success` conclusions for all seven jobs.
- The distributable Dockerfile now compiles all three native targets with
  `--features all-providers`. A local Docker build produced image `b62d47083b6f`; its packaged
  Java 6 (`1.6.0_38`) JAR completed a NATS publish/receive/client-ack probe with exit code 0.
- The 18 no-argument Java test classes in that image completed with exit code 0. The separate,
  parameterized `BrokerBackedMessagingTest` completed with exit code 0 against NATS in Java 6,
  and against NATS, JetStream, RabbitMQ, Kafka, and Pulsar on the host JVM through the
  instrumented Windows native library.
- That run recorded full Rust production-source coverage at 2,814/3,075 lines (91.51%) and the
  thresholded behavior-crate scope at 1,496/1,650 lines (90.67%). JNI ABI glue and demo CLIs
  remain in the full report but outside the narrower threshold denominator.
- JaCoCo on JDK 8, against classes compiled for Java 6 and including the NATS provider probe,
  recorded 802/889 lines (90.21%). The packaged Java 6 runtime probe remained a separate step.

## Recorded machine runs

| Environment | Revision | Raw machine fact | Reach |
|---|---|---|---|
| GitHub Actions run [32523731422](https://github.com/inovacc/modernlink/actions/runs/32523731422) | `686adaa` | All seven jobs reported conclusion `success`; Rust behavior lines 1,496/1,650 (90.67%), full Rust production lines 2,814/3,075 (91.51%), Java production lines 802/889 (90.21%) | Packaged Java 6→JNI→NATS, all five configured broker round trips/fault probes, linux-aarch64 load, thresholded coverage, Rust checks, and non-blocking dependency audit; not the vendor product |
| Build Native JAR run [32523731400](https://github.com/inovacc/modernlink/actions/runs/32523731400) | `686adaa` | The build-container, JAR-extraction, and artifact-upload steps reported conclusion `success` | Distributable construction at that revision; not deployment or vendor-host execution |
| GitHub Actions run [32386474212](https://github.com/inovacc/modernlink/actions/runs/32386474212) | `3b64484` | The Rust, Java 6 JAR, NATS/JetStream/RabbitMQ, Kafka/Pulsar, and linux-aarch64 jobs reported conclusion `success` | One configured happy-path send/receive/ack test per provider; packaged-JAR tests; native load on ARM64/JVM 21 |
| GitHub Actions run [31781200582](https://github.com/inovacc/modernlink/actions/runs/31781200582) | prior revision | The recorded Java job ran the packaged JAR on JVM `1.6.0_38`, including native load, live HTTPS, messaging, and routing probes | Java 6 runtime and linux-x86_64 at that revision; not the vendor product |
| Local Windows record, [2026-08-14-native-runtime.md](evidence/2026-08-14-native-runtime.md) | prior revision | JVM 21 loaded the Windows native, returned HTTP status 200 with four peer certificates, and completed NATS/JetStream/RabbitMQ round trips | Windows JNI/native path and those three brokers on one machine |
| Local dirty tree, 2026-08-21 | pre-`8d18ca5` | Docker build, 18 packaged Java 6 tests, packaged Java 6→JNI→NATS, and host-JVM→JNI round trips for all five provider variants emitted their recorded results | Local source and Docker runtime paths; superseded by the branch run for coverage and Linux execution facts |

The current workflow declares seven jobs, including a 90% production behavior-crate Rust gate
and a 90% Java production-class gate. Every downstream job depends directly or transitively on
the Rust workspace job; linux-aarch64 also waits for the Java JAR artifact. This dependency
graph prevents downstream jobs from starting after an unsuccessful required predecessor.
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
explicitly. Runs 32386474212 and 32523731422 each recorded one configured send/receive/ack path
for NATS core, JetStream, RabbitMQ, Kafka, and Pulsar. Run 32523731422 additionally invoked the
coverage fault probes; those probes target selected unavailable/poisoned state, not full recovery
semantics.

No recorded run exercises durability across restart, reconnect, ordering under load,
concurrency, timeout behavior, failure recovery, rollback, redelivery, or dead-letter behavior.
The provider guarantee table remains authoritative for what the code declares or refuses.

## Still unproven

- The vendor host product and its JMS implementation have never been part of a recorded run.
- `LEGACY_JMS` is an in-process compatibility transport, not a vendor-broker bridge.
- No run establishes production delivery semantics for any provider.
- B-003 delivery-mode enforcement remains open. B-011 records the committed candidate patch for
  the RabbitMQ fixture URI log; a live invocation of that changed fixture remains absent.
- Only the maintainer decides whether the observed behavior satisfies the intended contract.
