# ModernLink CLI Deterministic Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Create an isolated Rust CLI workspace with stable evidence, graph, claim, interview, readiness, and lifecycle-state contracts that cannot depend on the ModernLink runtime library.

**Architecture:** `cli/` is a separate Cargo workspace and release domain. Pure `model` types feed the `state` crate and the `modernlink-cli` binary; canonical serialization and validation make every later parser/plugin component consume the same deterministic contracts.

**Tech Stack:** Rust 1.96.0, edition 2024, Cargo resolver 2, serde, serde_json, sha2, thiserror, clap, tempfile.

**Spec:** `docs/modernization/2026-08-23-modernization-cli-plugin-design.md`

**Execution note (2026-08-23):** A working-path slice now precedes these foundation tasks:
`modernlink-analyzer` and `modernlink-cli` execute `modernlink analyze` on real Java repositories,
and `modernlink plugin bind` writes the plugin binary pointer. The model/state extraction described
below remains future work; preserve the command and report contracts while moving those types into
dedicated crates.

## Global Constraints

- The root runtime workspace and `cli/` workspace must have no Cargo dependency or workspace-member edge in either direction.
- All new Rust production code lives under `cli/`; no CLI crate enters the Java 6 JAR or `libmodernlink` artifact.
- Observed evidence requires a repository-relative path, artifact digest, half-open byte span, collector identity, and collector version.
- Durable IDs are content-derived SHA-256 identifiers; parser/runtime insertion IDs are forbidden.
- Properties use ordered maps and canonical serialization explicitly sorts all unordered collections.
- Only deterministic collectors create `Observed`; only deterministic rules create `Derived`; plugin agents create `Hypothesis`.
- Confirmed/rejected claims require a review record; accepted risks require an owner and expiry.
- No Reversa source, skill text, template, or schema is copied.
- New living docs start with `<!-- rev:001 (RFC 3339) <current-UTC> -->`; this dated implementation plan receives no revision tag.
- Every task runs `cargo fmt`, targeted tests, and clippy before commit.

## File Map

- `cli/Cargo.toml` — isolated workspace membership and shared dependency versions.
- `cli/crates/model/src/{lib,id,artifact,evidence,graph,claim,capability,lifecycle,interview,readiness}.rs` — stable serialized contracts.
- `cli/crates/state/src/{lib,event,policy,store}.rs` — lifecycle transitions, append-only journal, snapshot recovery.
- `cli/crates/modernlink-cli/src/{main,commands}.rs` — command parser and foundation commands.
- `cli/crates/modernlink-cli/tests/domain_firewall.rs` — proves workspace separation from Cargo metadata.
- `.github/workflows/cli.yml` — independent CLI formatting, lint, test, and domain-firewall gate.
- `cli/THIRD_PARTY_NOTICES.md` — dependency notice ledger; initially records no copied source.

---

### Task 1: Create the isolated CLI workspace and domain firewall

**Files:**
- Create: `cli/Cargo.toml`
- Create: `cli/crates/model/Cargo.toml`
- Create: `cli/crates/model/src/lib.rs`
- Create: `cli/crates/state/Cargo.toml`
- Create: `cli/crates/state/src/lib.rs`
- Create: `cli/crates/modernlink-cli/Cargo.toml`
- Create: `cli/crates/modernlink-cli/src/main.rs`
- Test: `cli/crates/modernlink-cli/tests/domain_firewall.rs`

**Interfaces:**
- Consumes: repository root `Cargo.toml` and `cli/Cargo.toml` through `cargo metadata --no-deps`.
- Produces: independent packages `modernlink-analysis-model`, `modernlink-analysis-state`, and binary `modernlink`.

- [ ] **Step 1: Create the virtual workspace and minimal crates**

`cli/Cargo.toml` must contain:

```toml
[workspace]
members = [
    "crates/model",
    "crates/state",
    "crates/modernlink-cli",
]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2024"
rust-version = "1.96"
license = "Apache-2.0"
publish = false

[workspace.dependencies]
clap = { version = "4.5", features = ["derive"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
sha2 = "0.10"
tempfile = "3.20"
thiserror = "2.0"
```

Each package inherits the workspace fields. `modernlink-analysis-state` depends only on
`modernlink-analysis-model`; `modernlink-cli` depends only on those two CLI crates plus clap.

- [ ] **Step 2: Verify both workspace roots resolve independently**

Run:

```text
cargo metadata --manifest-path Cargo.toml --no-deps --format-version 1
cargo metadata --manifest-path cli/Cargo.toml --no-deps --format-version 1
```

Expected: root metadata contains no manifest below `cli/`; CLI metadata contains no manifest
below root `crates/`, `java/`, or `hacks/`.

- [ ] **Step 3: Add the failing domain-firewall test**

Create a test that executes both metadata commands, parses JSON, and rejects crossing manifest
paths:

```rust
#[test]
fn runtime_and_cli_workspaces_are_disjoint() {
    let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors().nth(3).expect("repository root");
    let runtime = metadata(repo.join("Cargo.toml"));
    let cli = metadata(repo.join("cli/Cargo.toml"));
    assert!(runtime.iter().all(|path| !path.starts_with(repo.join("cli"))));
    assert!(cli.iter().all(|path| !path.starts_with(repo.join("crates"))
        && !path.starts_with(repo.join("java"))
        && !path.starts_with(repo.join("hacks"))));
}
```

Run `cargo test --manifest-path cli/Cargo.toml domain_firewall` before adding the `metadata`
helper. Expected: compile failure because `metadata` is undefined.

- [ ] **Step 4: Implement the metadata helper**

Use `std::process::Command::new("cargo")`, pass `metadata --manifest-path <path> --no-deps
--format-version 1`, assert success, parse `packages[*].manifest_path` using `serde_json::Value`,
canonicalize each path, and return `Vec<PathBuf>`.

- [ ] **Step 5: Run the workspace gate**

Run:

```text
cargo fmt --manifest-path cli/Cargo.toml --all -- --check
cargo test --manifest-path cli/Cargo.toml domain_firewall
cargo clippy --manifest-path cli/Cargo.toml --workspace --all-targets -- -D warnings
```

Expected: all pass; `git diff -- Cargo.toml Cargo.lock` shows no root-workspace change.

- [ ] **Step 6: Commit**

```text
git add cli/Cargo.toml cli/Cargo.lock cli/crates
git commit -m "build: create isolated modernization CLI workspace"
```

---

### Task 2: Implement stable identities, artifacts, spans, and evidence

**Files:**
- Create: `cli/crates/model/src/id.rs`
- Create: `cli/crates/model/src/artifact.rs`
- Create: `cli/crates/model/src/evidence.rs`
- Modify: `cli/crates/model/src/lib.rs`
- Test: colocated `#[cfg(test)]` modules in the three new files.

**Interfaces:**
- Consumes: UTF-8 canonical identity components.
- Produces: `ArtifactId`, `EvidenceId`, `NodeId`, `EdgeId`, `ClaimId`, `Artifact`, `SourceSpan`, `Evidence`, and `ParseHealth`.

- [ ] **Step 1: Write failing stable-ID and span tests**

```rust
#[test]
fn identity_is_prefixed_sha256_and_repeatable() {
    let a = EvidenceId::derive(["src/A.java", "sha256:abc", "10..20", "java-cst@1"]);
    let b = EvidenceId::derive(["src/A.java", "sha256:abc", "10..20", "java-cst@1"]);
    assert_eq!(a, b);
    assert!(a.as_str().starts_with("evidence:sha256:"));
}

#[test]
fn source_span_rejects_reverse_and_out_of_bounds_ranges() {
    assert!(SourceSpan::new("src/A.java", 20, 10, 100).is_err());
    assert!(SourceSpan::new("src/A.java", 0, 101, 100).is_err());
}
```

Run `cargo test -p modernlink-analysis-model id::tests artifact::tests evidence::tests`.
Expected: compile failure because the types do not exist.

- [ ] **Step 2: Implement typed IDs**

Implement one private `derive_id(prefix, components)` function that length-prefixes every
component before SHA-256 hashing. Generate lowercase hex and expose only `derive`, `as_str`,
`Display`, serde, equality, ordering, and hashing. Do not expose a constructor accepting an
unchecked string.

- [ ] **Step 3: Implement artifact and source contracts**

`Artifact` fields are:

```rust
pub struct Artifact {
    pub id: ArtifactId,
    pub path: String,
    pub digest: String,
    pub language: Option<String>,
    pub kind: ArtifactKind,
    pub classification: ArtifactClassification,
}
```

`SourceSpan` stores repository-relative `/`-separated path, `start_byte`, `end_byte`, and
optional one-based start/end lines. Reject absolute paths, `..`, NUL, reverse ranges, and end
beyond artifact length.

- [ ] **Step 4: Implement evidence and degradation states**

`Evidence` contains `id`, `artifact_id`, `span`, `collector`, `collector_version`,
`observation_kind`, ordered `properties`, and `health`. `ParseHealth` variants are `Complete`,
`RecoveredError`, `MissingNode`, `Cancelled`, `TimedOut`, `QueryLimitExceeded`,
`UnsupportedSyntax`, and `StaleArtifact`.

- [ ] **Step 5: Verify model serialization**

Add JSON round-trip tests and run:

```text
cargo test -p modernlink-analysis-model
cargo clippy -p modernlink-analysis-model --all-targets -- -D warnings
```

Expected: all tests pass and serialized paths contain `/`, never host-specific separators.

- [ ] **Step 6: Commit**

```text
git add cli/crates/model
git commit -m "feat: define stable analysis evidence identities"
```

---

### Task 3: Implement the canonical evidence graph and epistemic claims

**Files:**
- Create: `cli/crates/model/src/graph.rs`
- Create: `cli/crates/model/src/claim.rs`
- Create: `cli/crates/model/src/capability.rs`
- Create: `cli/crates/model/src/lifecycle.rs`
- Modify: `cli/crates/model/src/lib.rs`

**Interfaces:**
- Consumes: artifacts and evidence from Task 2.
- Produces: `EvidenceGraph::validate`, `EvidenceGraph::canonical_json`, `Claim::transition`, `Capability`, and `LifecycleStage`.

- [ ] **Step 1: Write failing graph-integrity tests**

Test orphan edges, observed evidence without spans, derived claims without rules/evidence,
hypothesis creation by an analyzer actor, and confirmation without review:

```rust
assert_eq!(graph.validate(), Err(GraphError::MissingEdgeSource(edge_id)));
assert_eq!(claim.transition(EpistemicState::Confirmed, None),
           Err(ClaimError::ReviewRequired));
```

Run `cargo test -p modernlink-analysis-model graph::tests claim::tests` and confirm failure.

- [ ] **Step 2: Implement nodes, edges, and graph validation**

Use `BTreeMap` storage keyed by typed IDs. Define node kinds from the spec and edge kinds
`Contains`, `Imports`, `Calls`, `Reads`, `Writes`, `Publishes`, `Consumes`, `Deploys`,
`Configures`, `Implements`, `Extends`, and `Contradicts`. Validation returns every error in a
sorted `Vec<GraphError>` so one defect never hides another.

- [ ] **Step 3: Implement canonical JSON**

Create a serialization view with sorted artifacts, evidence, nodes, edges, claims, and ordered
properties. Assert two graphs populated in reverse insertion order produce byte-identical
JSON and the same SHA-256 graph digest.

- [ ] **Step 4: Implement claim authority and transitions**

Define `ClaimAuthority::{Collector,Rule,PluginAgent,Reviewer}` and reject invalid creations:
collector→Observed, rule→Derived, plugin agent→Hypothesis, reviewer→Confirmed/Rejected.
`ReviewRecord` contains reviewer, timestamp, decision, rationale, and evidence digest.

- [ ] **Step 5: Add capabilities and lifecycle stages**

Define the exact capability enum (`Commands`, `Skills`, `Subagents`, `Hooks`, `Mcp`,
`FileReferences`) and all lifecycle stages from `Setup` through `Cleanup`. Derive serde and
stable kebab-case names.

- [ ] **Step 6: Run model gates and commit**

```text
cargo fmt --manifest-path cli/Cargo.toml --all -- --check
cargo test -p modernlink-analysis-model
cargo clippy -p modernlink-analysis-model --all-targets -- -D warnings
git add cli/crates/model
git commit -m "feat: add canonical evidence graph and claim authority"
```

---

### Task 4: Define coherence-interview and migration-readiness contracts

**Files:**
- Create: `cli/crates/model/src/interview.rs`
- Create: `cli/crates/model/src/readiness.rs`
- Modify: `cli/crates/model/src/lib.rs`

**Interfaces:**
- Consumes: `ClaimId`, `EvidenceId`, `LifecycleStage`.
- Produces: `QuestionRecord`, `DecisionState`, `CoherenceEnvelope`, `MigrationReadinessRecord`, `CoverageMeasurement`, `TestEvidenceMatrix`, and `ReadinessStatus`.

- [ ] **Step 1: Write failing contract tests**

```rust
#[test]
fn accepted_risk_requires_owner_and_expiry() {
    let mut record = readiness_fixture();
    record.status = ReadinessStatus::AcceptedRisk;
    record.risks[0].owner = None;
    assert!(record.validate().contains(&ReadinessError::RiskOwnerRequired));
}

#[test]
fn interview_cannot_close_with_unowned_blocker() {
    let envelope = envelope_with(DecisionState::Blocked { owner: None });
    assert_eq!(envelope.can_close(), false);
}
```

Run the targeted tests and confirm compile failure.

- [ ] **Step 2: Implement interview records**

Model all nine interview rounds from the spec. Store trigger claim/gap, alternatives, proposed
default and rationale, raw answer, normalized decision, responder, timestamp, affected IDs,
and confirmation state. `can_close` returns false for any unresolved material decision,
ownerless deferral/blocker, or unconfirmed normalization.

- [ ] **Step 3: Implement component-scoped coverage**

`CoverageMeasurement` contains ref, commit, tool, selector, features, excluded paths,
component scope, line/branch/function percentages, optional mutation score, and captured time.
`comparable_to` requires every scope-defining field to match; numeric equality is irrelevant.

- [ ] **Step 4: Implement readiness matrices**

Create explicit white-box, black-box, integration, data, operational, and security matrix
records. Validation enforces baseline command/result, oracle, environment fingerprint,
coverage policy decision, test gaps, risk ownership, and parity approver.

- [ ] **Step 5: Run tests and commit**

```text
cargo test -p modernlink-analysis-model interview::tests readiness::tests
cargo clippy -p modernlink-analysis-model --all-targets -- -D warnings
git add cli/crates/model
git commit -m "feat: model coherence interviews and migration readiness"
```

---

### Task 5: Implement lifecycle policy, append-only events, and recovery

**Files:**
- Create: `cli/crates/state/src/event.rs`
- Create: `cli/crates/state/src/policy.rs`
- Create: `cli/crates/state/src/store.rs`
- Modify: `cli/crates/state/src/lib.rs`

**Interfaces:**
- Consumes: lifecycle/readiness/interview types from `modernlink-analysis-model`.
- Produces: `TransitionPolicy::evaluate`, `StateStore::append`, `StateStore::recover`, and `CurrentState`.

- [ ] **Step 1: Write the failing transition table tests**

Use table-driven cases for every forward stage plus rollback. Assert `Assess -> Design` requires
an evidence-complete or policy-approved readiness record; `Modernize`, `Cutover`, and `Detach`
reject `AcceptedRisk`; unattended runs stop on high-impact questions.

- [ ] **Step 2: Implement transition policy**

Represent policies as exhaustive matches, not string configuration. Every rejection includes
a stable code, current/attempted stage, and sorted missing gate IDs.

- [ ] **Step 3: Write failing crash-recovery tests**

Append three events, omit the snapshot, recover from JSON Lines, and assert the reconstructed
state and digest equal the pre-crash state. Add corrupt-line and hash-chain-break cases.

- [ ] **Step 4: Implement the event store**

Each event includes sequence, previous hash, event hash, run ID, timestamp, transition,
artifact hashes, approvals, and actor. Append opens with create+append, writes one complete
line, flushes, and `sync_all`s. Snapshot writes to a sibling temporary file, flushes, syncs,
and renames atomically.

- [ ] **Step 5: Verify recovery and commit**

```text
cargo test -p modernlink-analysis-state
cargo clippy -p modernlink-analysis-state --all-targets -- -D warnings
git add cli/crates/state
git commit -m "feat: add resumable modernization lifecycle state"
```

---

### Task 6: Expose foundation CLI commands and independent CI

**Files:**
- Create: `cli/crates/modernlink-cli/src/commands.rs`
- Modify: `cli/crates/modernlink-cli/src/main.rs`
- Create: `cli/crates/modernlink-cli/tests/cli.rs`
- Create: `.github/workflows/cli.yml`
- Create: `cli/THIRD_PARTY_NOTICES.md`

**Interfaces:**
- Consumes: model/state public APIs.
- Produces: `modernlink version`, `modernlink status --state <path>`, and `modernlink schema validate --kind <kind> <path>`.

- [ ] **Step 1: Write failing command tests**

Using `std::process::Command` and `env!("CARGO_BIN_EXE_modernlink")`, assert version prints one
JSON object with CLI/protocol/schema versions, status recovers a fixture journal, and schema
validation returns exit 1 with a stable error code for an invalid readiness record.

- [ ] **Step 2: Implement clap commands and JSON output**

Use typed clap enums. stdout contains only requested data; diagnostics use stderr. Exit codes:
0 success, 1 invalid input/state, 2 missing file/dependency, 3 unsupported protocol/platform,
4 internal invariant failure.

- [ ] **Step 3: Add the independent CLI workflow**

The workflow checks out the repository and runs:

```text
cargo fmt --manifest-path cli/Cargo.toml --all -- --check
cargo test --manifest-path cli/Cargo.toml --workspace
cargo clippy --manifest-path cli/Cargo.toml --workspace --all-targets -- -D warnings
cargo test --manifest-path cli/Cargo.toml domain_firewall
```

It does not build the runtime workspace. Existing runtime CI remains unchanged.

- [ ] **Step 4: Add the initial notice ledger**

Record Apache-2.0 for ModernLink CLI, Reversa as concepts-only/no copied material at the pinned
commit, and Tree-sitter components as planned dependencies not yet distributed. Do not copy
third-party license text until the dependency is actually included.

- [ ] **Step 5: Run all gates**

```text
cargo fmt --manifest-path cli/Cargo.toml --all -- --check
cargo test --manifest-path cli/Cargo.toml --workspace
cargo clippy --manifest-path cli/Cargo.toml --workspace --all-targets -- -D warnings
cargo test --workspace
git status --short
```

Expected: both workspaces pass independently; only the pre-existing `.gitignore` change remains
outside this plan's staged files.

- [ ] **Step 6: Commit**

```text
git add cli .github/workflows/cli.yml
git commit -m "feat: expose deterministic CLI foundation"
```
