# ModernLink Modernization CLI and Plugin Design

## Status

Revised for written review on 2026-08-23. The clean-adaptation direction and the
`cli/` / `cli/plugin/` ownership split were approved in conversation. This revision adds
the hard library/CLI separation, component migration-readiness gates, coherence interviews,
the documentation lifecycle, and the Tree-sitter extraction architecture. Implementation is
underway in controlled slices: the analyzer, plugin-pointer contract, runtime-observation
preparation, Git-evolution artifact, shared evidence-model kernel, and lifecycle transition/replay
kernel exist. The analyzer and Git artifacts now adapt into that shared graph through
`modernlink inspect [--history]`; `modernlink architecture` produces deliberately labeled layer
inferences and candidate-context hypotheses from it. The analyzer also emits deterministic
import-derived infrastructure signals (JMS, JNDI, JDBC/persistence, EJB, JTA, JAX-WS, JAXB,
JMX, Servlet, RMI, Spring, and server APIs). `modernlink seams` currently scores vendor, JMS,
JNDI, database, and SOAP import boundaries with decomposed components and explicit migration
modes; call/data-flow-specific transaction, messaging, data, and runtime seam families remain
pending. `modernlink compatibility --target <major>` emits evidence-linked review findings for
observed internal-JDK, Java EE, and vendor-server imports; it intentionally does not claim
readiness or infer resolved dependencies and runtime behavior. The state kernel now persists/replays append-only JSONL events but has no workspace snapshot,
approval-latch records, or physical-artifact reconciliation yet. A descriptor registry exposes conservative
Codex and Claude capability records but deliberately has no installation paths; setup,
distribution, verified harness adapters, and the full lifecycle remain planned. `modernlink setup`
now creates only ModernLink-owned `.modernlink/` local metadata/cache directories and records
explicit harness selections; it does not modify harness instructions or claim adapter installation.
The canonical `cli/plugin/` bundle now contains the evidence/approval lifecycle rules, core
analyze/plan/modernize/verify/migrate/status command contracts, focused skills, and bounded
orchestrator/archaeologist/verification agent roles. Its Codex and Claude descriptors still
state `pending-ownership-contract`: no user-managed harness file is materialized yet.

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

### Hard domain separation

The runtime library and modernization CLI are separate domains with a one-way product
relationship:

- The **library** is the deliverable used by customer projects. Its Java 6/JNI/Rust runtime
  remains buildable, testable, publishable, installable, and operable without the CLI,
  plugin, Node.js, Tree-sitter, an LSP server, or any generated modernization state.
- The **CLI/plugin** is an external preparation ecosystem. It analyzes a checkout, asks for
  missing decisions, proves migration readiness, and may recommend adoption of the library.
  It is never loaded into the customer JVM and is never required after preparation.

The separation is enforced mechanically:

- the root Cargo workspace and `cli/` Cargo workspace have no path or package dependencies
  on each other;
- neither workspace imports source, generated bindings, tests, schemas, or build artifacts
  from the other;
- the root release never contains CLI/plugin assets and the CLI release never contains the
  Java facade, JNI library, or runtime provider transports;
- CI has independent dependency, build, test, coverage, security, and release jobs;
- the CLI cannot invoke the runtime library or treat its internal Rust crates as an analyzer
  SDK; and
- any recommendation to adopt ModernLink runtime is a documented migration option with
  versioned public contract references, not a code dependency.

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
├── rules/
│   └── tree-sitter/              # versioned per-language extraction/query/graph rules
├── vendor/
│   └── tree-sitter-graph/        # only if the licensed compatibility upgrade is accepted
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

### Tree-sitter, Tree-sitter Graph, and LSP architecture

The accepted extraction architecture is a converging evidence pipeline, not the linear
`Tree-sitter -> AST -> LSP -> Analyzer -> Graph` chain:

```text
source bytes ──> Tree-sitter CST ──> queries / Tree-sitter Graph rules ──> syntax drafts ─┐
                                                                                         │
build + deployment descriptors ──> deterministic collectors ───────────> config drafts ─┼─>
                                                                                         │
optional LSP server ──> symbols / references / types / diagnostics ────> semantic drafts ┘

drafts -> identity resolution -> canonical evidence graph -> analyzers -> claims/seams
```

Tree-sitter produces a concrete syntax tree shaped by a grammar. ModernLink may project a
normalized Java syntax model from named nodes, fields, query captures, source bytes, and parse
health, but it must not call that model a type-resolved AST. Tree-sitter node runtime IDs are
never durable identities.

The initial pinned syntax stack is:

```text
tree-sitter       = 0.26.13
tree-sitter-java  = 0.23.5
tree-sitter-graph = compatibility upgrade from upstream 0.12.0 to tree-sitter 0.26
```

The upstream commits assessed on 2026-08-23 are:

- `tree-sitter/tree-sitter` `dad7d0bd88817233637d7eae645c02e82d7884db`, MIT;
- `tree-sitter/tree-sitter-graph` `b930fb59c2177a90b3a6a68e1feeca6918ceb58b`,
  MIT OR Apache-2.0; and
- the official `tree-sitter-java` grammar release `0.23.5`, whose source commit and grammar
  ABI are recorded when the dependency is locked.

Unmodified Tree-sitter Graph `0.12.0` cannot consume Tree-sitter `0.27` trees because its
public Rust API is compiled against Tree-sitter `0.24` concrete `Language`, `Tree`, and `Node`
types. ModernLink must not resolve both versions and bridge them with unsafe conversion.

The preferred decision is a minimal, licensed compatibility upgrade of Tree-sitter Graph
under the CLI domain:

- preserve the upstream MIT and Apache-2.0 license files, copyright notices, history, and
  exact source commit;
- change only the Tree-sitter compatibility surface required for `0.27`;
- maintain an explicit patch ledger and upstream parity fixtures;
- run upstream tests plus ModernLink Java extraction tests before accepting the fork; and
- stop the integration if the upgrade cannot preserve strict-mode behavior without broader
  redesign.

The compatibility copy, if accepted after the spike, lives under
`cli/vendor/tree-sitter-graph/`. It is a CLI dependency only and never enters the root runtime
workspace or library artifact.

Tree-sitter Graph is a declarative per-language CST-to-intermediate-graph rule engine. It is
not ModernLink's durable evidence graph. The adapter executes one fresh strict-mode graph per
artifact, resolves syntax references while the parse tree is alive, emits source-spanned
drafts, canonicalizes them into ModernLink identities, and discards the upstream graph.

The adapter must:

- use normalized repository-relative path, artifact digest, half-open UTF-8 byte span,
  collector/rule digest, grammar version, and parser ABI as evidence identity inputs;
- canonical-sort node kinds, edges, property keys, sets, and observations before hashing or
  serialization;
- never persist upstream insertion indexes, `Node::id()`, runtime syntax references, or
  upstream JSON output;
- use strict execution initially; lazy execution remains disabled until cross-process and
  cross-order parity tests prove it deterministic;
- avoid `execute_into` for durable accumulation because partial mutation is not transactional;
- restrict custom functions to a reviewed, pure allowlist with bounded runtime and no
  filesystem, process, network, clock, randomness, environment, or mutable global access; and
- mark parse errors, missing nodes, cancellation, timeouts, query-limit overflow, stale
  digests, and unsupported syntax as degraded evidence rather than dropping the file.

LSP is optional and parallel because Tree-sitter and Tree-sitter Graph do not provide an LSP
pipeline, and a language server may require a resolvable build, dependencies, generated code,
an application-server classpath, and a compatible JDK. Syntax-only analysis must remain valid
when no server exists or semantic initialization fails.

Every LSP observation records server identity/version, initialization options, workspace
digest, JDK, build/classpath fingerprint, request, response digest, file URI, source range, and
diagnostic completeness. Semantic drafts join syntax evidence through the artifact digest,
normalized byte span, and canonical Java symbol identity. A conflicting LSP result adds
contradicting evidence; it never rewrites the syntax observation or erases provenance.

Fresh full-file parsing is the deterministic batch default. Incremental parsing is restricted
to watch mode until golden tests prove that fresh and incrementally updated inputs produce the
same canonical evidence graph.

The Java grammar is a modern union grammar, not a Java source-level validator. ModernLink must
carry version-partitioned fixtures for Java 1.4, 5, 6, 7, 8, and later syntax, including
malformed files, Unicode, CRLF/LF, generated sources, Maven/Gradle/Ant layouts, EAR/WAR
descriptors, JMS, JNDI, EJB, JTA, JAX-WS, JAXB, JDBC, Hibernate, and vendor APIs. Parse quality
and declared/observed source level are separate facts.

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

### Component migration-readiness prerequisites

ModernLink assesses readiness per migration component or seam. A repository-wide coverage
percentage cannot unlock a component whose own behavior is unobserved. Before a component may
transition from `assess` to `design`, the plugin creates a versioned
`MigrationReadinessRecord` containing:

- component and seam identifiers plus included/excluded nodes and entry points;
- pinned baseline commit, source/artifact digests, build selector, feature flags, runtime,
  application-server version, external-service versions, and test environment fingerprint;
- business criticality, data sensitivity, owner, approver, acceptable downtime, rollback
  objective, and parity/deviation policy;
- exact commands and scopes for unit, component, integration, system, coverage, and mutation
  tests;
- observed line, branch, function/method, and mutation coverage where the ecosystem can
  measure them;
- white-box, black-box, integration, data, operational, and security evidence matrices;
- known failing, flaky, quarantined, unavailable, or environment-dependent tests;
- uncovered business rules, error paths, integrations, and accepted risks with owner and
  expiry; and
- the final state: `blocked`, `evidence-complete`, `accepted-risk`, or `ready`.

Measurement is mandatory; a universal percentage is not treated as proof. During the
coherence interview, the plugin proposes defaults for the selected criticality and requires
the user to accept or replace them. The initial recommended profile for a high-criticality
component is at least 80% line coverage, 70% branch coverage, 60% mutation score when a stable
tool exists, 100% exercised critical business rules, and 100% represented public contract
scenarios. Different thresholds are valid only when their rationale, owner, and consequences
are recorded.

Coverage evidence must name the ref and selector. A measurement with a different component
scope, feature set, build profile, generated-source policy, or excluded-file set is not
comparable and cannot be used as a trend.

Required test layers:

1. **White-box characterization.** Exercise internal rules, branches, error paths, state
   transitions, concurrency boundaries, configuration variants, transactions, and recovery
   behavior. These tests capture what the legacy component does before restructuring.
2. **Black-box contract.** Drive every public API, message, job, file, screen, or protocol
   boundary through stable inputs and observe outputs, errors, status codes, side effects,
   ordering, timing class, and externally visible state. Golden masters are allowed only with
   normalization rules and reviewable diffs.
3. **Integration.** Exercise each database, transaction manager, broker, directory, HTTP/SOAP
   service, filesystem, application-server service, and security provider through a pinned
   fixture, container, emulator, recorded contract, or approved real environment.
4. **Data parity.** Prove schema, encoding, precision, null, default, identifier, ordering,
   transactional, migration, and rollback behavior using representative non-sensitive data.
5. **Operational behavior.** Record startup, shutdown, retry, timeout, back-pressure,
   idempotency, observability, resource, and failure-recovery expectations.
6. **Security behavior.** Preserve authorization, authentication, audit, redaction, transport,
   input-validation, and secret-handling contracts without storing credential values.

The baseline suite must pass against the pinned legacy component before modernization begins.
Existing failures may remain only when reproduced, classified, explicitly excluded from
parity, and assigned an owner. The candidate implementation runs the same black-box and
integration contracts; every difference is either a failure or an approved, traceable
behavioral deviation.

If the legacy component cannot be built or executed, the plugin does not invent readiness. It
records the missing environment and blocks automated `modernize`, `cutover`, and `detach`.
`accepted-risk` may permit design or preparatory work, but never unattended cutover.

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

### Coherence interview engine

The plugin asks as many questions as are required to produce a coherent, internally
consistent modernization envelope. It does not impose a fixed question count, but it also
does not ask for facts the deterministic analyzer can establish from the repository.

Questions are generated from unresolved decisions, contradictions, low-confidence claims,
missing migration prerequisites, and consequences that materially change the plan. They are
asked one at a time in interactive mode, checkpointed after every answer, and grouped into
reviewable rounds:

1. **Purpose and outcome:** why modernization is needed, success measure, deadline, desired
   end state, and whether retaining the legacy component is acceptable.
2. **Scope and ownership:** repositories, modules, deployment units, components, exclusions,
   owners, approvers, teams, change cadence, and business criticality.
3. **Runtime and deployment:** JDK/source level, build system, app server and exact version,
   OS/architecture, containers, topology, class loading, startup, shutdown, and release path.
4. **Contracts and domain:** entry points, consumers, invariants, transactions, workflows,
   permissions, terminology, tolerated behavior changes, and context boundaries.
5. **Data and integrations:** stores, schemas, queues, topics, external APIs, identity systems,
   file exchanges, ownership, consistency, volume, retention, sensitivity, and test access.
6. **Verification:** existing commands, test levels, coverage/mutation tools and thresholds,
   fixtures, golden oracles, known failures/flakes, environment availability, and who decides
   parity.
7. **Migration and operations:** candidate strategies, sequencing, coexistence, data movement,
   observability, capacity, downtime, cutover window, rollback objective, and detach proof.
8. **Security, compliance, and autonomy:** regulated data classes, network permission, tool
   installation, model/harness permissions, write boundaries, approvals, and actions that must
   remain human-only.
9. **Documentation and lifecycle:** tracked versus local artifacts, canonical instructions,
   required living docs, review cadence, revision policy, and definition of done.

Every question record contains the triggering claim/gap, alternatives considered, default and
why it is safe, answer, answer source, responder, timestamp, affected decisions, and whether
the answer confirms or contradicts repository evidence. Free-form answers are normalized into
a proposed decision and shown back for confirmation.

The interview finishes only when every material decision is `answered`, `derived-and-
confirmed`, `explicitly-deferred` with owner/date, or `blocked`. Before the lifecycle advances,
the plugin presents a coherence summary and checks:

- goals, scope, tests, architecture, plan, and rollback do not contradict each other;
- every assumption is labeled and has a validation action;
- every accepted risk has an owner and expiry;
- every migration component has a readiness record; and
- unresolved blockers are not hidden by an autonomous/default mode.

An unattended run is allowed only after the interactive envelope explicitly authorizes its
scope and defaults. New contradictory or high-impact questions stop the run regardless of
that authorization.

## Plugin bundle

The plugin is a control and interpretation layer, not a second analyzer.

Initial command intentions (an intention marked **planned** is not an executable CLI command):

```text
modernlink setup
modernlink interview          # planned
modernlink analyze
modernlink status
modernlink architecture
modernlink domains            # planned
modernlink seams
modernlink readiness          # planned
modernlink plan
modernlink prepare            # planned
modernlink verify             # planned
modernlink migrate            # planned as root command; lifecycle guidance exists in plugin
modernlink cutover            # planned
modernlink detach             # planned
modernlink cleanup            # planned
modernlink docs update        # planned
```

Initial skills:

- repository-reconnaissance
- architecture-discovery
- vendor-lock-analysis
- domain-hypothesis
- bounded-context-review
- modernization-seams
- migration-readiness
- migration-strategy
- characterization-testing
- parity-verification
- application-server-exit
- cutover-and-rollback
- provenance-and-evidence-review
- docs-lifecycle

Initial agent roles:

- orchestrator
- repository-archaeologist
- Java/application-server analyst
- dependency and architecture analyst
- domain analyst
- modernization architect
- migration planner
- test and parity strategist
- documentation steward
- verification auditor
- implementation reviewer

Each skill declares which binary query it consumes and which claim types it may emit. Skills
cannot write `Observed` or `Derived` claims. Agents that propose changes must reference seam and
claim identifiers instead of restating unsupported repository facts.

## Documentation lifecycle

The plugin includes a clean implementation of the operator's local `/project:docs:*` logic as
the `docs-lifecycle` skill and `modernlink docs update` command. The local files are design
input only; the installed plugin never reads `C:\Users\dyamm\.claude`, depends on a Claude
installation, or copies machine-specific paths.

The documentation pipeline is idempotent and ordered:

```text
-1 coverage preflight -> 0 project detection -> 1 audit -> 2 lean
                      -> 3 create missing -> 4 reconcile existing
                      -> 5 format/revise -> 6 verify/capture/stamp
```

### Stage -1: coverage preflight

Before a documentation run writes coverage claims, it validates a machine-readable baseline
against the documented ref, tool, selector, features, exclusions, and component scope. A
missing or stale comparable baseline triggers measurement. A different scope is re-measured
and labeled non-comparable. If no coverage tool is wired, documentation says `N/A`, creates a
backlog item, and never estimates a percentage.

Coverage for the working tree and coverage for `main` are distinct records. The documentation
names which one it reports.

### Stage 0: project detection

Detection is based on tracked marker files and actual build consumption, not the first marker
encountered in an unclassified tree. It supports multi-language and monorepo subtrees, prefers
specific markers and verified task-runner commands, and classifies authored, generated,
vendored, and build-output trees before counting or documenting them.

The initial ecosystem registry covers Go, Rust, Node/TypeScript, Bun, Deno, Python, Zig,
Maven, Gradle, .NET, Elixir, Ruby, PHP, Dart/Flutter, Swift, Haskell, and C/C++ through CMake or
Make. Unknown layouts become interview questions rather than guessed project types.

### Stage 1: audit

The audit is read-only. It inventories manifests, task runners, source units, public entry
points, recent Git state, coverage, per-unit documentation, instruction-file relationships,
and the managed documentation set:

```text
README.md                    AGENTS.md                 CLAUDE.md
LICENSE                      docs/ROADMAP.md           docs/MILESTONES.md
docs/BACKLOG.md              docs/ISSUES.md            docs/BUGS.md
docs/FEATURES.md             docs/CONTRIBUTORS.md      docs/ARCHITECTURE.md
docs/IMPLEMENTATION_TASKS.md docs/adr/
```

The report classifies every item as current, missing, stale, contradictory, oversized, or not
applicable, with evidence. No write stage runs without a corresponding audit finding.

### Stage 2: lean

`AGENTS.md` is the project-level canonical cross-harness instruction file. `CLAUDE.md` is a
thin importer plus Claude-only behavior; other harness entry files point to the canonical
rules rather than duplicate them.

Resident files retain hard rules, routing, common commands, security, and contribution rules.
Long reference tables, templates, examples, and deep procedures move into lazy-loaded topic
documents. Content is moved and verified before removal, never summarized away. Nested
subproject instructions remain scoped to their subtree.

The default lean target is at most 150 lines and 8 KiB; exceeding 200 lines or 10 KiB requires
a split unless the coherence review records why the content must remain resident.

### Stages 3 and 4: create and reconcile

Missing documents are created from actual repository state without placeholders. Existing
documents receive targeted factual corrections rather than stylistic rewrites. Completed work
is checked only when code and verification evidence exist. Bugs, limitations, backlog,
features, milestones, and tasks remain separate ledgers with cross-referenced stable IDs.

Architecture diagrams describe real components and happy/error flows. ADRs record decisions a
future maintainer would otherwise relitigate and are superseded rather than silently rewritten.

### Stage 5: format and revision discipline

Every created or edited living instruction, governance, or contract document carries this
exact line immediately after its single H1:

```text
<!-- rev:NNN (RFC 3339) YYYY-MM-DDTHH:MM:SSZ -->
```

Rules:

- new living documents start at `rev:001` with the current UTC timestamp;
- each later edit increments the zero-padded counter by exactly one and refreshes the
  timestamp;
- a timestamp-less legacy tag gains the RFC 3339 timestamp on its next edit;
- untouched documents are not reformatted or revision-bumped; and
- dated specifications/plans/notes, ADRs with `Status`, migrations, changelogs, evidence
  records, and generated Markdown do not receive revision tags.

The formatter also verifies one H1, consistent heading levels, fenced-code languages where
appropriate, resolved relative links, no trailing whitespace, and one trailing newline.

### Stage 6: verify, capture, and stamp

The pipeline runs the detected non-destructive build/lint/reference verification, captures the
coverage baseline when available, and writes ignored local state at
`docs/.project/docs/LAST-UPDATE.json` containing:

- update timestamp, branch, and commit;
- files created or edited;
- coverage status, ref, selector, scope, and comparability;
- known stale or intentionally deferred documentation; and
- the project-detection and pipeline protocol versions.

The stamp is written even when no tracked document changes. It proves freshness without
creating churn and never implies that a named stale document was reconciled.

Documentation changes remain inside the analyzed target project. The CLI's docs pipeline does
not modify the ModernLink runtime/library documentation merely because the preparation
ecosystem ran against another repository.

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
├── interviews/
├── readiness/
├── testing/
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

Tree-sitter is consumed as an MIT-licensed dependency rather than copied into ModernLink.
Tree-sitter Java is consumed as an MIT-licensed grammar dependency. If the Tree-sitter Graph
compatibility spike succeeds, its accepted source is copied only into the CLI domain under
the upstream `MIT OR Apache-2.0` terms. `cli/THIRD_PARTY_NOTICES.md` then records the project,
version, commit, chosen license, affected directory, local patch ledger, and license-file
locations. Binary release archives include the applicable notices.

The documentation lifecycle was derived from the operator's local
`/project:docs:update` revision 009 and related project-detection/template guidance as of
2026-08-23. ModernLink reimplements the behavior in its own Rust state, schemas, and plugin
instructions; the local Claude configuration is neither packaged nor read at runtime.

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
- Tree-sitter `0.27.0` and Java grammar `0.23.5` ABI/version lock tests.
- Fresh versus incremental parse parity tests before incremental batch reuse is enabled.
- CST/graph fixtures proving canonical IDs survive process, file-order, newline, and path-root
  variation while source spans remain exact.
- Upstream Tree-sitter Graph strict-mode parity tests against the licensed compatibility
  upgrade, plus rejection of lazy mode and impure custom functions.
- Parse-degradation fixtures for `ERROR`, `MISSING`, timeout, cancellation, query overflow,
  unsupported source level, Unicode, and malformed legacy Java.

### Migration readiness and parity

- Schema and state-transition tests for every readiness status.
- Component-scope coverage tests that reject repository-wide or selector-incompatible
  measurements.
- White-box characterization fixtures with deliberate uncovered branches and mutations.
- Black-box old/new differential suites covering normal, error, side-effect, ordering, and
  approved-deviation cases.
- Integration suites for database, transaction, messaging, directory, HTTP/SOAP,
  application-server, filesystem, and security boundaries.
- Tests proving a missing baseline environment, unowned accepted risk, failing legacy suite,
  or unreviewed parity difference blocks `modernize`, `cutover`, and `detach`.
- Tests proving `accepted-risk` can allow design/preparation when policy permits but never
  unattended cutover.

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
- Interview tests for derived answers, contradictory answers, resume, explicit deferral,
  coherence review, and newly discovered high-impact blockers during unattended runs.
- Documentation-pipeline fixtures for every detected ecosystem, multi-language roots,
  idempotent no-op runs, revision bumps, point-in-time exceptions, stale coverage, broken
  links, canonical AGENTS/CLAUDE relationships, and freshness stamps with `known_stale`.

## Initial implementation sequence

This umbrella design contains four independently reviewable implementation plans. Each plan
must leave a working, tested product slice and cannot depend on an unfinished later plan.

### Plan A: deterministic foundation and domain firewall

1. create the isolated `cli/` Cargo workspace without changing the root workspace membership;
2. add CI assertions that the root and CLI dependency graphs do not reference each other;
3. implement the `model` crate with evidence, graph, claim, capability, lifecycle, interview,
   and migration-readiness types;
4. implement graph validation, canonical serialization, stable identities, and schema round
   trips with tests;
5. implement the `state` crate's transition table, append-only event model, approval latches,
   and recovery tests;
6. implement CLI commands `version`, `status`, and schema validation; and
7. record Reversa and Tree-sitter-stack provenance decisions in tracked documentation.

### Plan B: Java syntax-to-evidence vertical slice

1. pin Tree-sitter `0.27.0` and tree-sitter-java `0.23.5` with source and ABI metadata;
2. run the Tree-sitter Graph `0.12.0` compatibility-upgrade spike against `0.27`;
3. if the spike passes upstream and parity tests, place the licensed compatibility source and
   patch ledger under `cli/vendor/tree-sitter-graph/`; otherwise stop for a dependency decision;
4. implement fresh Java parsing, parse-health evidence, source spans, query/rule digests, and
   the strict per-file intermediate-graph adapter;
5. canonicalize Java package, type, method, import, inheritance, annotation, and call-site
   drafts into the ModernLink evidence graph; and
6. verify deterministic goldens across Java version, malformed-source, newline, Unicode,
   process, and file-order fixtures.

### Plan C: plugin coherence and migration readiness

1. define `plugin.toml`, command/skill/agent, pointer, locator, harness, interview, readiness,
   and docs-state schemas;
2. add Codex and Claude descriptors without installing either yet;
3. implement resumable coherence interviews and summary validation;
4. implement component-scoped test inventory, coverage comparability, readiness gates, and
   blocked/accepted-risk transitions;
5. implement white-box, black-box, integration, data, operational, and security evidence
   matrices; and
6. implement the idempotent documentation pipeline through audit-only and dry-run outputs
   before enabling tracked-document writes.

### Plan D: release bootstrap and safe installation

1. implement the npm bootstrap's platform selection and release-manifest validation;
2. implement HTTPS download, archive safety, SHA-256 verification, atomic versioned install,
   pointer writing, and binary delegation;
3. implement harness installation planning, per-file ownership manifests, updates, repair, and
   uninstall without recursive directory ownership;
4. test bootstrap behavior against a local fake release server and hostile fixture archives;
   and
5. produce independently versioned CLI/plugin release artifacts without touching the runtime
   library release.

## Acceptance criteria

The initial implementation sequence is acceptable when:

- the root runtime workspace builds without resolving any CLI dependency;
- the CLI workspace builds without resolving any root runtime crate or artifact;
- the isolated CLI workspace formats, lints, and tests cleanly;
- model validation rejects unsupported epistemic transitions and orphaned evidence links;
- state recovery reproduces an identical snapshot from the event log;
- the Java syntax collector produces byte-stable canonical evidence across repeated runs;
- no unmodified Tree-sitter Graph `0.12` type crosses a Tree-sitter `0.27` API boundary;
- the licensed graph compatibility upgrade, if accepted, passes upstream and ModernLink parity
  tests and ships with notices and a patch ledger;
- every migration component has a readiness record with component-scoped coverage plus
  white-box, black-box, and integration evidence or remains visibly blocked;
- unresolved high-impact interview questions prevent autonomous lifecycle advancement;
- two consecutive healthy documentation runs produce no second tracked diff;
- edited living docs carry an exactly-one revision increment and point-in-time records remain
  unstamped;
- the npm bootstrap selects only an exact supported release asset and rejects a bad digest;
- the pointer contains no target-repository path and resolves only inside the ModernLink user
  installation root;
- Codex and Claude descriptors pass schema validation;
- no installer command recursively deletes a directory;
- no Reversa source or template appears in the diff; and
- tracked provenance names every upstream repository, pinned commit/version, license, reused
  or copied component, patch, notice, and implementation exclusion.
