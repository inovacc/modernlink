# Contributors and Contributing Guide
<!-- rev:008 (RFC 3339) 2026-08-21T19:20:00Z -->

## Maintainers

| Name | Contact | Role |
|---|---|---|
| Dyam Marcano | dyam.marcano@gmail.com | Maintainer |

Repository: <https://github.com/inovacc/modernlink> · License: Apache-2.0

## Toolchain

There is **no** `rust-toolchain.toml` pin. CI resolves `dtolnay/rust-toolchain@stable`, and the
packaging image pins `rust:1.96-bookworm`. All crates are `edition = "2021"` with no declared
`rust-version` (MSRV).

| Component | Version |
|---|---|
| Rust (CI) | `stable` — unpinned |
| Rust (packaging image) | 1.96 |
| Java (facade target) | 6 — `-source 1.6 -target 1.6` |
| Java (build image) | `java:6b38-jdk` (deprecated, see [ISSUES.md](ISSUES.md) I-004) |

## Getting a working tree

```bash
git clone https://github.com/inovacc/modernlink.git
cd modernlink
cargo test --workspace
```

**The default build needs no native toolchain.** `crates/messaging` gates every provider
behind a cargo feature and defaults to none (SC-07), so a plain `cargo test --workspace`
compiles no broker client — no cmake, no librdkafka, no protoc. That is the command to run
while developing.

Building with providers is what needs the toolchain: `--features kafka` builds librdkafka from
C via cmake, and `--features pulsar` needs protoc. Install `cmake`, `libcurl` and
`protobuf-compiler` before using `--all-features` or building the distributable.

**Read a CI result precisely.** The broker-backed tests are `#[ignore]`d, so ordinary Rust jobs
execute none of them. Dedicated jobs invoke all five; run 32386474212 recorded those configured
jobs at `3b64484`. Read [VERIFICATION.md](VERIFICATION.md) before describing runtime reach.

## Checking the Java facade without Docker

Docker is required to compile at `-source 1.6` and to run anything, but **type errors do not
need it**:

```bash
find java/src/main/java -name '*.java' > /tmp/main.txt
javac -source 8 -target 8 -nowarn -Xlint:-options -d /tmp/classes @/tmp/main.txt
find java/src/test/java -name '*.java' > /tmp/test.txt
javac -source 8 -target 8 -nowarn -Xlint:-options -cp /tmp/classes -d /tmp/classes @/tmp/test.txt
```

Java 6 syntax is a subset of 8, so anything that compiles at `-source 1.6` compiles here. The
reverse is **not** true — this accepts lambdas and diamonds the real build rejects — so it is a
fast pre-check, not a substitute for the Docker build.

Worth doing: a `ModernPayload` change reached `main` without it and broke the CI JAR build with
five `unreported exception` errors this catches in seconds.

## Commands

| Purpose | Command |
|---|---|
| Test the Rust workspace (no broker client compiled) | `cargo test --workspace` |
| Test with every provider transport | `cargo test --workspace --all-features` |
| Check the JNI crate | `cargo check -p jni-bridge` |
| Format | `cargo fmt --all -- --check` |
| Lint | `cargo clippy --workspace --all-targets -- -D warnings` |
| Rust behavior-crate coverage gate plus full JNI/demo report (Linux + Docker) | `bash scripts/run_rust_coverage.sh` |
| Build the Java 6 JAR | `docker build -f docker/java6/Dockerfile -t modernlink-java6-https .` |
| Run a packaged Java test | `docker run --rm modernlink-java6-https sh -c "java -cp /workspace/modernlink.jar com.modernlink.LegacyHttpsTest"` |

Use `-p jni-bridge` for the workspace JNI crate. The old package-name collision with the
external `jni` crate is resolved, but `jni-bridge` remains the package's canonical name
(ISSUES I-001).

## Code standards

**Rust**
- `rustfmt` clean. Keep provider transports behind the uniform transport boundary in
  `crates/messaging` so the JNI surface stays provider-neutral.
- Fail closed: an unsupported guarantee is an explicit error, never a silent degradation.

**Java — the rule that bites**
- Everything under `java/src` compiles as Java 6. **No lambdas, method references, Streams,
  diamond operator, or try-with-resources.** Use anonymous inner classes and explicit loops.
- `cargo test` will not catch a violation. Only the Docker build does — run it before pushing
  Java changes.
- Naming: `Modern*` for new surface, `Legacy*` for the compatibility API the host already calls.

**Tests**
- Java tests are standalone `main`-style classes, not JUnit. The workflow discovers compiled
  `*Test.class` files; preserve that discovery when adding tests so a fixed list cannot drift.

## Commits and pull requests

- Conventional commits: `feat:`, `fix:`, `docs:`, `build:`, `style:`, `merge:`.
- No AI attribution in commit messages.
- Before proposing a merge, run `cargo test --workspace` and `cargo check -p jni-bridge`.
- **Report gate results as facts, not as a verdict.** A green run is a machine result; it is not
  proof that the Java 6 integration satisfies the intended contract. Recorded Java/broker runs
  and their limits are in [VERIFICATION.md](VERIFICATION.md). Only the maintainer decides whether
  work is done.

## Where to start

Open work is tracked in [BACKLOG.md](BACKLOG.md) (M1/M2 items with acceptance criteria) and
broken into tasks in [IMPLEMENTATION_TASKS.md](IMPLEMENTATION_TASKS.md). The highest-value gaps
are the vendor JMS inventory/compatibility boundary and delivery-mode enforcement — see
ISSUES I-010 and BUGS B-003.
