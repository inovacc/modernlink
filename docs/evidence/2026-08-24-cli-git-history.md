# CLI Git-history evidence

## Scope

This record covers only the local deterministic Git-history foundation in the isolated `cli/`
workspace. It does not validate the Java 6 runtime, a vendor application server, enterprise-scale
repository behavior, semantic ownership, or any agent recommendation.

## Recorded machine facts

On 2026-08-24, the following controlled fixture commands exited `0`:

```text
cargo test --manifest-path cli/Cargo.toml -p git
cargo test --manifest-path cli/Cargo.toml -p modernlink-cli --test analyze_command --test history_command
cargo fmt --manifest-path cli/Cargo.toml --all -- --check
cargo clippy --manifest-path cli/Cargo.toml --workspace --all-targets -- -D warnings
```

The local command exercise below also exited `0` against this checkout:

```text
cargo run --manifest-path cli/Cargo.toml -p modernlink-cli -- history . --refs head --max-commits 20 --output <new-temporary-json-path>
```

Its JSON receipt reported 20 collected commits, 106 path changes, 253 bounded co-change pairs,
and `commit-limit-reached (max_commits=20)`. The temporary artifact was 96,535 bytes. This shows
that the command can collect a deliberately limited local sample and state the limit; it does not
establish that the sample represents the full repository or that any inferred support contact,
architecture, domain, seam, or migration recommendation would be correct.

## Current implementation boundary

- Facts are collected through the Rust `gix` API rather than parsing human-oriented `git` output.
- Commit messages are fingerprinted in the artifact; their text is not emitted by default.
- The project-local cache is ignored under `.modernlink/cache/git/`.
- `--mailmap repo` currently emits `mailmap-not-applied`; it is not a claim of normalized identity.
- Existing target report files are refused rather than overwritten silently.
- Nested directory entries are excluded from file-level coupling; a controlled merge-parent
  fixture is still pending.

The corresponding design and remaining work are in
[Git Evolution Intelligence Design](../modernization/2026-08-23-git-evolution-intelligence-design.md)
and its [implementation plan](../modernization/plans/2026-08-23-git-evolution-intelligence-plan.md).
