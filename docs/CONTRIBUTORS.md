# Contributors and Contributing Guide
<!-- rev:005 (RFC 3339) 2026-08-20T00:00:00Z -->

## Maintainers

| Name | Contact | Commits |
|---|---|---|
| Dyam Marcano | dyam.marcano@gmail.com | 59 (all) |

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

**Read a green CI run precisely.** The broker-backed tests are `#[ignore]`d, so no CI run has
now run against real brokers in dedicated CI jobs — all five providers, run 32386474212. What has and has
not actually run is tracked in [BUGS.md](BUGS.md) under "Verification reach" — read it before
describing this project as tested.

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
| Coverage | `cargo llvm-cov --workspace --all-features --summary-only` |
| Build the Java 6 JAR | `docker build -f docker/java6/Dockerfile -t modernlink-java6 .` |
| Run a packaged Java test | `docker run --rm modernlink-java6 sh -c "java -cp /workspace/modernlink.jar com.modernlink.LegacyHttpsTest"` |

Use `-p jni-bridge`, never `-p jni` — the workspace crate shadows its own external dependency
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
- Java tests are standalone `main`-style classes, not JUnit. Add new ones the same way and wire
  them into `.github/workflows/test.yml` — a test the workflow does not invoke never runs.

## Commits and pull requests

- Conventional commits: `feat:`, `fix:`, `docs:`, `build:`, `style:`, `merge:`.
- No AI attribution in commit messages.
- Before proposing a merge, run `cargo test --workspace` and `cargo check -p jni-bridge`.
- **Report gate results as facts, not as a verdict.** A green run is a machine result; it is not
  proof that the Java 6 integration works. Nothing in this repo has been validated against the
  real Java 6 runtime or a real broker. Only the maintainer decides whether work is done.

## Where to start

Open work is tracked in [BACKLOG.md](BACKLOG.md) (M1/M2 items with acceptance criteria) and
broken into tasks in [IMPLEMENTATION_TASKS.md](IMPLEMENTATION_TASKS.md). The highest-value gap
is broker-backed evidence for the messaging transports — see ISSUES I-010.
