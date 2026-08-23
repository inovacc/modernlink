# Reversa Reference Integration Report

Date: 2026-08-23
Branch: `feat/reversa-integration`

## Outcome

ModernLink now has one end-to-end preparation path that remains outside the runtime library:

```text
plugin binary pointer -> modernlink analyze -> Tree-sitter Java CST
  -> source evidence -> canonical graph objects -> deterministic JSON report
```

The path was executed against `E:\modernlink`. The machine run reported 62 Java artifacts,
257 evidence items, 69 semantic nodes, 144 relationships, and zero parser-recovery flags. Two
post-deduplication runs at the same repository state produced byte-identical report SHA-256
`242D5F3F2C4CF6D8527AC2E47A25A626E7689B1C9708FD67CD3635239F5FFCA3`.
These are machine observations, not a human verdict that the model is complete or correct.

## Reversa summary

`sandeco/reversa` is an npm-distributed Node.js CLI plus a large prompt-driven agent-skill suite.
Its strongest fit for ModernLink is not its JavaScript implementation but its lifecycle ideas:
harness installation, staged discovery and migration, resumable artifact-based work, explicit
approval points, confidence-aware conclusions, append-only progress, characterization before
change, and parity review.

ModernLink retains those ideas inside a different architecture: a deterministic Rust analyzer
produces evidence first; a thin harness plugin asks coherence questions and delegates to the
binary; later lifecycle policy may promote observed evidence into derived findings or reviewed
hypotheses.

## License implications

- Assessed Reversa commit: `f3c36892e8aefa44f020f7ad74917089e67ddaa3`.
- License: MIT, copyright 2026 Sandeco.
- MIT would permit copying and modification if its license and copyright notice accompanied the
  affected distribution.
- This integration copied no Reversa source, JavaScript, prompt text, templates, schemas, or
  fixtures. Consequently no Reversa notice is embedded as copied-code attribution; the reference
  and future-copy conditions are recorded in `cli/THIRD_PARTY_NOTICES.md`.
- Tree-sitter 0.26.13 and tree-sitter-java 0.23.5 are consumed as MIT-licensed crates; their source
  was not copied into ModernLink.
- tree-sitter-graph 0.12 was assessed but is not included. It targets an older concrete
  Tree-sitter API. A future compatibility copy must live only under `cli/vendor/`, retain upstream
  MIT OR Apache-2.0 materials, and carry a patch ledger.

## Exact reusable components and treatment

| Reference idea or component | ModernLink use | Treatment |
|---|---|---|
| Harness/plugin installation | Codex and Claude manifests plus a portable skill bundle | Clean reimplementation |
| CLI-to-plugin location | Versioned `binary-pointer.json` written by Rust `plugin bind` | Original ModernLink contract |
| Repository evidence before recommendations | Artifact digests, byte spans, collector identity, parse health | Clean reimplementation |
| Evidence relationships | Stable package/type nodes and containment/import edges | Clean reimplementation |
| Confidence-aware conclusions | Observations only in the current report; reviewed claim promotion remains planned | Concept retained, not yet implemented |
| Resumable migration lifecycle | State-machine and append-only journal specified, not executable yet | Concept retained, future work |
| Characterization before change | Plugin asks for white-box, black-box, integration, data, operational, and security evidence | Clean instructional adaptation |
| Reversa JavaScript installer | Not used or ported | Rejected as an implementation source |
| Tree-sitter | Java CST parser feeding the Rust analyzer | Direct dependency, not copied |
| Tree-sitter Graph | Potential declarative rule engine after compatibility work | Assessed only, excluded |
| LSP | Future optional semantic enrichment beside CST collection | Planned, never evidence authority by itself |

## Implemented files

- `cli/Cargo.toml`, `cli/Cargo.lock`, `cli/.cargo/config.toml`, `cli/.gitignore` — isolated Rust
  workspace and build output boundary.
- `cli/crates/analyzer/Cargo.toml`, `cli/crates/analyzer/src/lib.rs` — Java discovery, CST parsing,
  stable SHA-256 identities, evidence spans, graph construction, coalescing, parse health, and
  canonical report serialization.
- `cli/crates/analyzer/tests/java_repository.rs` — repository analysis and shared-node evidence
  aggregation probes.
- `cli/crates/modernlink-cli/Cargo.toml`, `cli/crates/modernlink-cli/src/main.rs` — `analyze` and
  `plugin bind` executable commands with structured diagnostics.
- `cli/crates/modernlink-cli/tests/analyze_command.rs`,
  `cli/crates/modernlink-cli/tests/plugin_bind_command.rs` — process-boundary command probes.
- `cli/plugin/.codex-plugin/plugin.json`, `cli/plugin/.claude-plugin/plugin.json` — initial harness
  manifests.
- `cli/plugin/skills/analyze-repository/SKILL.md` — deterministic delegation and migration
  readiness interview procedure.
- `cli/plugin/config/binary-pointer.schema.json`, `cli/plugin/config/.gitignore` — versioned pointer
  contract and machine-local pointer exclusion. Its identifier is the non-network URN
  `urn:modernlink:schema:binary-pointer:v1`; no `modernlink.dev` ownership is assumed.
- `cli/THIRD_PARTY_NOTICES.md` — provenance and future-copy obligations.
- `docs/modernization/2026-08-23-modernization-cli-plugin-design.md` — coherent CLI/plugin design,
  parser decision, lifecycle, readiness, docs pipeline, and Reversa assessment.
- `docs/modernization/plans/2026-08-23-cli-foundation-plan.md` — next foundation extraction plan.

The existing user change to root `.gitignore` was neither staged nor committed.

## Commits

- `792dafe` — `docs: design modernization CLI and plugin`
- `3090e57` — `docs: add migration readiness and parser architecture`
- `bd63dd7` — `docs: plan deterministic modernization CLI foundation`
- `72d17b3` — `feat: add deterministic Java modernization analyzer`
- `add913b` — `style: normalize CLI file endings`
- `a2364ef` — `fix: remove unowned schema domain`
- `d05fc17` — `feat: detect legacy Java platform signals`

## Machine probes and remaining uncertainty

- `cargo test --workspace` under `cli/` exited 0 with four integration tests executed.
- `cargo clippy --workspace --all-targets -- -D warnings` under `cli/` exited 0.
- Codex plugin and skill validators exited 0 after manifest/frontmatter corrections.
- Cargo metadata reported six runtime packages and two CLI packages, with zero cross-workspace
  package paths in either direction.
- The bound binary's SHA-256 matched its pointer, and pointer-based analysis emitted the expected
  analysis schema.
- The root runtime workspace baseline did not execute because its existing `.cargo/config.toml`
  directs output to `C:\temp\jni\target`, where this environment received access denied. No
  runtime-library file was changed to bypass that independent issue.
- Real WebLogic, WebSphere, Gradle, Ant, generated-source, malformed Java, and very large repository
  inputs remain unexercised. Descriptor-path and import evidence alone does not prove bounded
  contexts, runtime calls, ownership, deployment topology, or safe seams.

## Continuation: legacy-platform signal collection

The analyzer now recognizes Maven, Gradle, and Ant build descriptors; Java EE web, EAR, and EJB
deployment descriptors; JBoss, WebLogic, and WebSphere vendor descriptors; and JBoss, WebLogic,
and WebSphere Java import prefixes. Each result is a `derived` technology signal carrying a stable
rule ID and one or more observed evidence IDs. Repeated matches coalesce without losing their
individual evidence references.

A shallow checkout of `wildfly/quickstart` was analyzed as a real positive input. The machine run
reported 369 Java files, 105 recognized configuration files, 3,545 evidence items, and six
coalesced technology-signal classes. Those classes cited 85 Maven descriptors, seven JBoss
deployment descriptors, 154 JBoss vendor imports, 12 Java EE web descriptors, and one EJB
descriptor. Two runs produced byte-identical report SHA-256
`635C4D734E8FDD16F874D37E7D8A153FCC67083C1F04387DFD892A8F6DAB4D0E`.

This does not establish that every quickstart deploys to a specific server, that imports represent
executed paths, or that the rules cover all vendor descriptor variants. WebLogic and WebSphere
positive paths currently have controlled repository probes but no independently sourced real
repository run in this slice.

## Recommended next implementation steps

1. Parse descriptor contents and dependency coordinates for Maven, Gradle, Ant, EAR/WAR,
   JBoss/WildFly, WebLogic, WebSphere, JNDI, EJB, JMS, JAX-WS, JAXB, JDBC, and Hibernate instead of
   relying only on file paths and vendor import prefixes.
2. Extract stable report types into the planned model crate and add claim authority
   (`Observed`, `Derived`, `Hypothesis`, `Confirmed`, `Rejected`) without changing the current JSON
   contract silently.
3. Implement component-scoped migration-readiness records and the nine-round coherence interview,
   including test coverage comparability and owned risk expiry.
4. Add append-only lifecycle state and crash recovery before implementing modernization actions.
5. Run the tree-sitter-graph compatibility spike against Tree-sitter 0.26.13; vendor it only if
   upstream and ModernLink parity probes justify the maintenance cost.
6. Add optional LSP enrichment as a parallel collector whose output remains linked to source
   evidence and is never silently promoted to observed fact.
7. After the analyzer handles representative real repositories, implement the minimal npx/bunx
   downloader for signed/checksummed release binaries and reuse Rust `plugin bind` for installation.
