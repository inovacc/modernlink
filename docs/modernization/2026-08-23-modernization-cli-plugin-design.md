# ModernLink Modernization CLI and Plugin Design

## Status

Proposed for written review on 2026-08-23. The clean-adaptation direction and the
`cli/` / `cli/plugin/` ownership split were approved in conversation; implementation
starts after this document is reviewed.

## Goal

Build a repository modernization product alongside the existing ModernLink runtime.
The product must inspect legacy Java repositories deterministically, preserve the
evidence behind every conclusion, identify architecture and modernization seams, and
coordinate harness-specific skills and agents through a resumable lifecycle.

The product has two distribution surfaces:

1. a native Rust binary that owns analysis, state validation, graph storage, and
   lifecycle transitions; and
2. a plugin bundle that exposes skills, agents, commands, and workflow guidance to AI
   harnesses while treating the Rust binary as the source of observed facts.

The existing Java 6/JNI compatibility runtime remains independent. It may eventually be
recommended as one modernization action, but the analyzer never depends on, loads, or
modifies the runtime crates while inspecting another repository.

## Product boundary

```text
ModernLink Runtime
  Java 6 facade -> JNI -> Rust transports

ModernLink Modernization Product
  npx/bunx bootstrap -> released native binary -> pointer file
                                              -> plugin bundle
  target repository -> deterministic Rust analyzers -> evidence graph
                                                   -> modernization model
  AI harness -> plugin commands/skills/agents -> binary queries/transitions
```

The two products share the ModernLink name and repository but not process boundaries,
dependency graphs, release artifacts, or compatibility promises.

## Non-goals for the first implementation slice

- Do not port Reversa source line by line.
- Do not embed Node.js, Bun, an LLM SDK, or an application-server runtime in the Rust
  binary.
- Do not make model inference authoritative over deterministic evidence.
- Do not modify a target repository outside ModernLink-owned state and output folders.
- Do not perform automatic modernization, cutover, deletion, publication, or network
  deployment.
- Do not claim complete Java semantic resolution in the first slice.
- Do not implement every supported harness at once. The registry is general, while the
  first verified adapters are Codex and Claude Code.
- Do not fold the new crates into the Java 6 JAR or `libmodernlink` native library.

## Repository layout

All new executable Rust code and its package-manager bootstrap live under `cli/`.

```text
cli/
├── Cargo.toml                    # isolated Rust workspace for the CLI product
├── crates/
│   ├── model/                    # evidence graph, claims, capabilities, lifecycle types
│   ├── analyzer/                 # deterministic repository collectors and detectors
│   ├── state/                    # state machine, journal, artifact hashes, reconciliation
│   ├── harness/                  # harness registry and safe plugin installation planning
│   └── modernlink-cli/           # command parsing and orchestration binary
├── bootstrap/
│   └── npm/                      # tiny npm package used by npx and bunx
├── plugin/
│   ├── plugin.toml               # bundle identity, protocol, binary requirements
│   ├── commands/                 # user entry commands and lifecycle routing
│   ├── skills/                   # focused modernization methods
│   ├── agents/                   # harness-neutral agent role descriptors
│   ├── harnesses/                # Codex/Claude materialization descriptors
│   └── schemas/                  # plugin/pointer/state output schemas
└── tests/
    ├── fixtures/                 # deterministic legacy Java repository fixtures
    └── bootstrap/                # platform-selection and pointer integration tests
```

`cli/` is intentionally an isolated Cargo workspace rather than a member of the runtime
workspace at the repository root. This prevents analyzer dependencies, command-line
libraries, and release tooling from affecting the Java 6 runtime build or JAR.

## Distribution and bootstrap

### Package-manager entry

The npm-compatible package is `@inovacc/modernlink`. It supports:

```text
npx @inovacc/modernlink@latest setup
bunx @inovacc/modernlink@latest setup
npm install -g @inovacc/modernlink@latest
modernlink setup
```

The npm package contains no analyzer logic and no copied Reversa installer code. Its only
responsibilities are:

1. normalize the operating system and CPU architecture;
2. select an exact release asset from a signed release manifest;
3. download the archive over HTTPS;
4. verify the declared SHA-256 digest before extraction;
5. install into a versioned, user-scoped directory;
6. invoke the installed binary with the original arguments; and
7. return the binary's exit code without rewriting its diagnostic output.

The bootstrap must support Windows x86-64, Linux x86-64, Linux ARM64, and macOS
x86-64/ARM64 when matching release assets exist. Unsupported platforms fail before any
filesystem mutation and name the normalized platform tuple.

### Release manifest

Every CLI release publishes a machine-readable manifest next to the archives:

```json
{
  "schema_version": 1,
  "version": "0.1.0",
  "protocol_version": 1,
  "assets": [
    {
      "target": "x86_64-pc-windows-msvc",
      "archive": "modernlink-cli-x86_64-pc-windows-msvc.zip",
      "executable": "modernlink.exe",
      "sha256": "64-lowercase-hex-characters"
    }
  ]
}
```

The bootstrap rejects a manifest with an unsupported schema, version mismatch, duplicate
target, unsafe archive path, absent digest, or asset outside the configured release origin.
Signature verification is a release-hardening milestone; SHA-256 verification and pinned
HTTPS release origins are mandatory in the first implementation.

### Installation location

The native binary is stored outside target repositories:

```text
Windows: %LOCALAPPDATA%\ModernLink\cli\versions\<version>\modernlink.exe
Linux:   ${XDG_DATA_HOME:-$HOME/.local/share}/modernlink/cli/versions/<version>/modernlink
macOS:   $HOME/Library/Application Support/ModernLink/cli/versions/<version>/modernlink
```

Downloads use a temporary sibling file. Installation is an atomic rename after hash
verification. Existing verified versions are reused. A version directory is never
recursively replaced in place.

## Binary pointer contract

The plugin never guesses a global PATH and never embeds a machine-specific binary path in a
tracked repository file. `modernlink setup` writes a user-local pointer file and installs a
small repository-local locator file that names the pointer, not the executable.

User-local pointer:

```text
Windows: %LOCALAPPDATA%\ModernLink\cli\current.json
Linux:   ${XDG_CONFIG_HOME:-$HOME/.config}/modernlink/cli/current.json
macOS:   $HOME/Library/Application Support/ModernLink/cli/current.json
```

Pointer schema:

```json
{
  "schema_version": 1,
  "binary": "absolute-normalized-path",
  "version": "0.1.0",
  "protocol_version": 1,
  "target": "x86_64-pc-windows-msvc",
  "sha256": "64-lowercase-hex-characters",
  "installed_at": "RFC-3339 UTC timestamp"
}
```

Repository-local locator, owned by the plugin:

```json
{
  "schema_version": 1,
  "pointer": "user-config://modernlink/cli/current.json",
  "required_protocol": 1
}
```

The symbolic `user-config://` URI prevents repository-specific absolute paths from being
committed. Harness adapters resolve the URI using platform rules, parse the pointer, validate
the schema and protocol, contain the resolved path within the ModernLink installation root,
verify that the executable is a regular file, and then execute `modernlink plugin ...`.

The pointer is written atomically and is never trusted as authorization to delete or replace
its parent directory. A broken pointer produces a recovery instruction using
`npx @inovacc/modernlink@latest setup --repair`.

## Rust component design

### `model`

The model crate owns stable, serializable domain types and contains no filesystem scanning or
harness behavior.

Core identities:

- `ArtifactId`: stable hash-derived identity for a file, descriptor, archive, symbol, or
  configuration unit.
- `EvidenceId`: identity for an observed fact with source span and collector version.
- `NodeId` and `EdgeId`: graph identities derived from canonical content.
- `ClaimId`: identity for a conclusion and its evidence set.

Core records:

- `Artifact`: repository-relative path, language, kind, digest, and classification.
- `SourceSpan`: repository-relative path plus byte and line boundaries.
- `Evidence`: collector, observation kind, normalized value, source span, and artifact digest.
- `Node`: repository, module, package, type, method, endpoint, table, queue, deployment unit,
  vendor API, or inferred domain concept.
- `Edge`: contains, imports, calls, reads, writes, publishes, consumes, deploys, configures,
  implements, extends, or contradicts.
- `Claim`: statement, epistemic state, confidence, supporting and contradicting evidence,
  derivation rule, and reviewer decision.

Epistemic state is explicit:

```text
Observed -> Derived -> Hypothesis -> Confirmed
                          \-------> Rejected
Observed/Derived --------> Contradicted
```

Only deterministic collectors may create `Observed` facts. Deterministic rules may create
`Derived` claims when every input is recorded. Plugin agents may create `Hypothesis` claims.
Only an explicit review transition may create `Confirmed` or `Rejected` claims.

Graph validation fails when an observed fact lacks a source span, a derived claim lacks a
derivation rule or evidence, an edge references a missing node, or a confirmed claim lacks a
review record.

### `analyzer`

The analyzer runs ordered, independently testable collectors:

1. repository inventory and authored/generated/vendored/build-output classification;
2. build-system and module discovery for Maven, Gradle, Ant, and raw Java layouts;
3. Java package, type, method, annotation, import, inheritance, and call-site extraction;
4. deployment descriptor extraction from EAR/WAR, `web.xml`, EJB, application-server, and
   persistence configuration;
5. vendor-lock detection for JBoss/WildFly, WebLogic, WebSphere, Tomcat, proprietary JNDI,
   JMS, EJB, JTA, JCA, JAX-WS, JAXB, and server-specific APIs;
6. persistence, messaging, HTTP/SOAP, scheduled-work, and transaction relationship
   extraction;
7. graph metrics including cycles, fan-in, fan-out, centrality, unstable dependencies, and
   vendor-edge concentration; and
8. rule-based modernization seam candidates with complete evidence paths.

Collectors produce evidence and graph deltas, never prose reports. Language-aware parsing is
preferred; bounded lexical detection may be used only when the evidence records its reduced
precision and the claim remains unconfirmed.

### Domain and bounded-context inference

Bounded contexts are hypotheses, not observed repository facts. Deterministic features supply
the input:

- package and module cohesion;
- transaction boundaries;
- shared database tables and schemas;
- synchronous and asynchronous message boundaries;
- public API ownership;
- domain vocabulary concentration;
- dependency cycles and change coupling from Git history; and
- deployment-unit ownership.

The plugin's domain analyst groups these signals using invariants, transaction boundaries,
change cadence, and team ownership when available. Every proposed context carries supporting
and contradicting evidence, a confidence score, and an explicit human validation state.

### Modernization seams

A seam candidate is a graph cut with an evidence-backed intervention proposal. The score is a
transparent tuple rather than one opaque number:

```text
vendor_lock, coupling, blast_radius, behavioral_coverage,
data_ownership, runtime_criticality, reversibility, migration_value
```

Candidates identify the affected nodes and edges, required characterization tests, contract
to preserve, capability gaps, possible strategies, rollback boundary, and proof needed before
cutover. Strategies include strangler, branch by abstraction, parallel run, adapter/anti-
corruption layer, data replication, and retained legacy operation. The tool may recommend but
does not execute a strategy without an approved lifecycle transition.

### `state`

The state crate owns a deterministic modernization state machine:

```text
setup -> discover -> understand -> model -> assess -> design -> plan
      -> prepare -> modernize -> migrate -> verify -> cutover
      -> detach -> cleanup
```

Each transition declares required inputs, produced artifacts, verification gates, allowed
rollback transitions, and whether human approval is mandatory. State updates use:

- an immutable run identifier;
- append-only JSON Lines events;
- content hashes for every referenced artifact;
- an atomic current-state snapshot derived from the event log;
- separate approval-latch records; and
- reconciliation against physical artifacts after interruption.

Physical-artifact detection repairs or challenges state; it does not silently advance the
lifecycle. A mismatch becomes a `Contradicted` claim and blocks dependent transitions.

### `harness`

Harness support is descriptor-driven. Each descriptor declares:

- harness identity and version constraints;
- detected marker files;
- plugin installation roots;
- supported capabilities: commands, skills, subagents, hooks, MCP, and file references;
- invocation template for the Rust binary;
- ownership boundaries; and
- merge policy for pre-existing instruction files.

The installer first creates a plan, displays all paths and conflicts, and applies only after
approval unless `--yes` was explicitly supplied. Every installed file has its own manifest
entry containing original hash, installed hash, current ownership, and merge disposition.
Directory ownership never authorizes recursive deletion. Update and uninstall operate only on
individually manifested files after containment checks.

## Plugin bundle

The plugin is a control and interpretation layer, not a second analyzer.

Initial commands:

```text
modernlink setup
modernlink analyze
modernlink status
modernlink architecture
modernlink domains
modernlink seams
modernlink plan
modernlink prepare
modernlink verify
modernlink migrate
modernlink cutover
modernlink detach
modernlink cleanup
```

Initial skills:

- repository-reconnaissance
- architecture-discovery
- vendor-lock-analysis
- domain-hypothesis
- bounded-context-review
- modernization-seams
- migration-strategy
- characterization-testing
- parity-verification
- application-server-exit
- cutover-and-rollback
- provenance-and-evidence-review

Initial agent roles:

- orchestrator
- repository-archaeologist
- Java/application-server analyst
- dependency and architecture analyst
- domain analyst
- modernization architect
- migration planner
- verification auditor
- implementation reviewer

Each skill declares which binary query it consumes and which claim types it may emit. Skills
cannot write `Observed` or `Derived` claims. Agents that propose changes must reference seam and
claim identifiers instead of restating unsupported repository facts.

## Target repository state

ModernLink-owned repository state lives under `.modernlink/`:

```text
.modernlink/
├── plugin-locator.json
├── config.toml
├── state.json
├── events.jsonl
├── manifests/
├── graphs/
├── evidence/
├── claims/
├── decisions/
└── reports/
```

Analysis is read-only outside `.modernlink/`. Generated human-facing modernization documents
are written under `docs/modernlink/` only when the user enables tracked reports. The default
keeps all generated state local and ignored.

## Reversa reference and provenance

The design was informed by `sandeco/reversa` at commit
`f3c36892e8aefa44f020f7ad74917089e67ddaa3`, licensed under MIT with copyright
2026 Sandeco.

Concepts retained through clean reimplementation:

- harness detection and bundled agent installation;
- resumable checkpoints and physical-artifact awareness;
- separate approval latches;
- confidence-aware claims;
- append-only progress history;
- immutable originals plus addenda;
- staged discovery, migration, refactoring, and parity review; and
- characterization tests before reversible changes.

No Reversa source code, templates, skill text, or schemas are copied in this design. Its
JavaScript installer is not translated because static review found ownership, recursive
deletion, containment, manifest-rebuild, and Windows path concerns. ModernLink reimplements
the required behavior against its own safety contracts.

If future work copies or substantially adapts Reversa material, that change must add the MIT
license text and copyright notice to a tracked third-party notice, identify every affected
file, and record the upstream commit. Materials with unclear individual provenance remain
concept-only.

## Error and safety model

- Unsupported platforms, schemas, protocols, capabilities, or analyzers fail closed.
- Paths are canonicalized and containment-checked before every write, update, or removal.
- Symlinks and junctions are never followed during plugin cleanup.
- Archive entries with absolute paths, parent traversal, links, or duplicate normalized paths
  are rejected.
- Existing project instructions are never overwritten. Merge is explicit and ownership of
  pre-existing content is never claimed.
- Modified installed files are preserved and surfaced as conflicts.
- Target repository files are read only during analysis.
- Credentials, file contents classified as secrets, payloads, and database row values never
  enter reports, claims, logs, or prompts.
- Network access is disabled during repository analysis unless a command explicitly declares
  and receives approval for an external research capability.
- Plugin execution uses argument arrays, not shell-built command strings.

## Verification strategy

### Rust model and state

- Round-trip serialization tests for every public schema.
- Property tests for stable identities and graph referential integrity.
- State-machine transition tables covering every allowed and denied transition.
- Crash/recovery tests that rebuild snapshots from append-only events.
- Tests proving hypotheses cannot become observed facts.

### Analyzer

- Small, licensed-in-repository fixtures for Maven, Gradle, Ant, EAR/WAR, JBoss/WildFly,
  WebLogic, WebSphere, Tomcat, EJB, JMS/JNDI/JTA, SOAP/JAX-WS, JAXB, and JDBC/Hibernate.
- Golden evidence graphs for each fixture.
- False-positive fixtures containing vendor names only in comments or documentation.
- Deterministic-output tests across repeated runs and path separators.
- Graph metric and seam-ranking tests with hand-computed expected results.

### Bootstrap and installation

- Platform tuple and asset-selection table tests.
- Hash mismatch, truncated archive, traversal, link, and duplicate-entry rejection tests.
- Atomic install interruption and repair tests.
- Pointer resolution tests on Windows, Linux, and macOS path rules.
- Manifested-file update/uninstall tests that preserve modified and unowned files.
- Protocol mismatch tests between plugin, pointer, and binary.

### Plugin

- Schema validation for every descriptor, skill, command, and agent role.
- Capability degradation tests for harnesses without subagents or slash commands.
- Contract tests proving emitted claims reference existing graph evidence.
- End-to-end fixture flow from bootstrap pointer through `analyze`, `seams`, and `status`.

## First implementation slice

The first slice is deliberately narrow but executable:

1. create the isolated `cli/` Cargo workspace;
2. implement the `model` crate with evidence, graph, claim, capability, and lifecycle types;
3. implement graph validation and JSON round trips with tests;
4. implement the `state` crate's lifecycle transition table and append-only event model;
5. implement `modernlink-cli` commands `version`, `pointer validate`, `plugin locate`, and
   `status`;
6. define `plugin.toml`, pointer, locator, and harness descriptor schemas;
7. add a Codex descriptor and a Claude Code descriptor without installing either yet;
8. implement the npm bootstrap's platform selection, release-manifest validation, download,
   SHA-256 verification, atomic installation, pointer writing, and binary delegation;
9. test bootstrap behavior against a local fake release server and fixture archives; and
10. record Reversa provenance and the explicit no-copy decision in tracked documentation.

Repository scanning, Java parsing, application-server detection, seam scoring, and lifecycle
mutation commands follow as independently testable slices after this foundation is verified.

## Acceptance criteria

The first slice is acceptable when:

- the root runtime workspace builds without resolving any CLI dependency;
- the isolated CLI workspace formats, lints, and tests cleanly;
- model validation rejects unsupported epistemic transitions and orphaned evidence links;
- state recovery reproduces an identical snapshot from the event log;
- the npm bootstrap selects only an exact supported release asset and rejects a bad digest;
- the pointer contains no target-repository path and resolves only inside the ModernLink user
  installation root;
- Codex and Claude descriptors pass schema validation;
- no installer command recursively deletes a directory;
- no Reversa source or template appears in the diff; and
- tracked provenance names the upstream repository, pinned commit, MIT license, reused
  concepts, and implementation exclusions.
