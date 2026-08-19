# CLAUDE.md
<!-- rev:003 (RFC 3339) 2026-08-19T00:00:00Z -->

Claude Code entry point for the ModernLink repo. Canonical cross-tool agent instructions
live in **AGENTS.md** (imported below) — keep shared rules there, not here.

@AGENTS.md

## Claude-Code-only

- **The Java 6 boundary is the easiest rule to break by reflex.** When editing anything under
  `java/src`, no lambdas, method references, streams, diamond operator, or try-with-resources —
  `javac -source 1.6 -target 1.6` in `docker/java6/Dockerfile` is the only compiler that
  judges it, and it is not run by `cargo test`.
- **`-p jni-bridge`** is the JNI crate (folder `crates/jni`, lib `modernlink`). The old
  `-p jni@0.1.0` workaround is gone: SC-05 renamed the package, so `-p jni` is no longer
  ambiguous. `crates/core` is the package `modernlink-core` (`use modernlink_core::...`).
- Before proposing a merge, run `cargo test --workspace` and `cargo check -p jni-bridge` and
  report the real result. Neither proves the Java 6 integration works. The Java 6 **runtime** is
  now exercised in CI (JVM `1.6.0_38`, run 31782837766); the **vendor host product** and its JMS
  implementation are not, and the broker-backed tests are `#[ignore]`d so no CI run reaches a
  broker.
