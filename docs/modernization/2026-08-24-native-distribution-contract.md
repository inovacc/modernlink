# ModernLink Native Distribution Contract

Status: design verified against npm/Bun package-executable behavior; implementation pending.

## Decision

`@inovacc/modernlink` is a tiny npm/Bun distribution wrapper, not the analyzer. It exposes the
`modernlink` executable through `package.json` `bin`; after platform resolution it delegates to
the matching Rust binary without parsing or rewriting its output.

Platform packages are the preferred initial model:

- `@inovacc/modernlink-win32-x64`
- `@inovacc/modernlink-linux-x64`
- `@inovacc/modernlink-linux-arm64`
- `@inovacc/modernlink-darwin-x64`
- `@inovacc/modernlink-darwin-arm64`

They are optional dependencies of the wrapper and carry an exact matching version. The wrapper
must reject a missing or mismatched platform binary with an actionable diagnostic; it must never
fall back to a different architecture or silently download an unverified executable.

## Execution contract

1. npm installs globally through the `bin` mapping; Bun can execute the same npm package with
   `bunx` and forwards arguments after the executable name.
2. The bootstrap selects only its exact OS/architecture platform package and `exec`s the Rust
   binary.
3. The binary owns all analysis, configuration, caching, and terminal output. Node/Bun is not a
   runtime requirement after that handoff.
4. `npx`/`bunx` use their own package caches. `--no-install` / equivalent cache-miss behavior is
   the defined offline path; no implicit GitHub-release download is part of v1.

## Deferred external authority

Publishing, registry credentials, signing keys, release hosting, and corporate-proxy test
environments remain release-operator responsibilities. If download-on-demand is later added, a
signed release manifest with SHA-256 verification, proxy configuration, cache ownership, and
offline failure behavior must be specified before implementation.

## Evidence

- npm `package.json` documents `bin` mappings and optional dependencies.
- Bun documents `bunx` execution of npm package binaries and argument forwarding.
- The current CLI/plugin product contract remains Rust-first; this document does not authorize a
  TypeScript analyzer.
