# Contributors and Contributing Guide
<!-- rev:003 (RFC 3339) 2026-08-20T00:00:00Z -->

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

`crates/messaging` builds native code through `rdkafka` (cmake) and `pulsar` (protobuf), and
`crates/messaging/Cargo.toml` declares **no `[features]`** — so every provider is unconditional
and *any* `cargo test/clippy/llvm-cov --workspace` must build a native Kafka client. On a bare
host, install `cmake`, `libcurl`, and `protobuf-compiler` first, or the workspace will not build
— see ISSUES I-005.

**CI does not build it either right now.** The `Rust workspace` job has no install step and has
failed on five consecutive pushes for exactly this reason — see [BUGS.md](BUGS.md) B-001. Do not
treat CI as evidence that `cargo test --workspace` passes; run it locally and report what you
actually saw.

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
| Test the Rust workspace | `cargo test --workspace` |
| Check the JNI crate | `cargo check -p jni-bridge` |
| Format | `cargo fmt --all -- --check` |
| Lint | `cargo clippy --workspace --all-targets -- -D warnings` |
| Coverage | `cargo llvm-cov --workspace --summary-only` |
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
