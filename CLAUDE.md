# CLAUDE.md
<!-- rev:001 (RFC 3339) 2026-08-14T01:13:05Z -->

Claude Code entry point for the ModernLink repo. Canonical cross-tool agent instructions
live in **AGENTS.md** (imported below) — keep shared rules there, not here.

@AGENTS.md

## Claude-Code-only

- **The Java 6 boundary is the easiest rule to break by reflex.** When editing anything under
  `java/src`, no lambdas, method references, streams, diamond operator, or try-with-resources —
  `javac -source 1.6 -target 1.6` in `docker/java6/Dockerfile` is the only compiler that
  judges it, and it is not run by `cargo test`.
- **`-p jni@0.1.0`, never `-p jni`** — the workspace crate shadows its own external dependency.
- Before proposing a merge, run `cargo test --workspace` and `cargo check -p jni@0.1.0` and
  report the real result. Neither proves the Java 6 integration works; that needs the Java 6
  runtime and real brokers, which has not been done.
