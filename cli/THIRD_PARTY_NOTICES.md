<!-- rev:001 (RFC 3339) 2026-08-23T20:41:19Z -->

# ModernLink CLI third-party provenance

This ledger applies only to the modernization CLI/plugin domain under `cli/`. It does not alter
the ModernLink runtime library or its distributable artifacts.

## Reversa reference

- Project: `sandeco/reversa`
- Assessed commit: `f3c36892e8aefa44f020f7ad74917089e67ddaa3`
- Upstream license: MIT
- Upstream copyright: Copyright 2026 Sandeco
- Treatment: concepts only; no Reversa source, JavaScript, prompt text, templates, schemas, or
  fixtures were copied or substantially adapted.

Clean ModernLink implementations informed by the assessment are the evidence-first report,
stable content-derived identities, the thin plugin-to-binary binding, and the requirement to
collect characterization evidence before modernization decisions. Resumable lifecycle state,
approval latches, confidence promotion, artifact addenda, and parity workflows remain design
requirements and are not present in the current executable slice.

If future work copies or substantially adapts Reversa material, add the upstream MIT license and
copyright notice to the distribution, identify every affected file here, and record the exact
upstream path and commit before merging.

## Parser dependencies

- `tree-sitter` 0.26.13 — consumed from crates.io under MIT; no source copied into ModernLink.
- `tree-sitter-java` 0.23.5 — consumed from crates.io under MIT; no grammar source copied into
  ModernLink.
- `tree-sitter-graph` — analyzed but not included. Upstream 0.12 targets an older concrete
  Tree-sitter API; any future compatibility copy belongs under `cli/vendor/` with its upstream
  MIT OR Apache-2.0 files and a patch ledger.
