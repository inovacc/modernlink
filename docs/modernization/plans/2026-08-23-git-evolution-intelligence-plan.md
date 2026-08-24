# ModernLink Git Evolution Intelligence Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a private Rust `git` crate and `modernlink history` command that produce deterministic, local-only Git-history evidence without parsing `git` terminal output.

**Architecture:** The `git` crate uses `gix 0.87.0` as the structured repository backend and owns Git-specific facts, limits, completeness reporting, canonical serialization, and cache keys. The CLI owns argument parsing and atomic output publication. The existing analyzer stays independent until a later shared-model extraction can join the two schemas through an explicit adapter.

**Tech Stack:** Rust 1.96, edition 2024, `gix 0.87.0` (MIT OR Apache-2.0), serde, serde_json, sha2, thiserror, clap, tempfile.

**Spec:** `docs/modernization/2026-08-23-git-evolution-intelligence-design.md`

## Global Constraints

- Create `cli/crates/git/` with package and crate name `git`; all CLI manifests inherit `publish = false`.
- Use `gix` structured APIs for repository/history facts. Do not parse human-readable output from `git` commands.
- Never mutate, clone, fetch, push, checkout, or otherwise change the inspected repository.
- Default ref scope is all locally reachable refs; record refs and every incompleteness condition in output.
- Preserve raw author/committer identity observations; never infer a person or merge identities by display name alone.
- Do not store commit-message text, remote credentials, source content, or telemetry in default artifacts.
- Canonical JSON is stable across repeated traversals of unchanged local inputs.
- Cache only under `.modernlink/cache/git/`; no cache becomes a committed repository artifact.
- Do not copy code from `E:\projects_old\gitc`; record `gix` provenance in `cli/THIRD_PARTY_NOTICES.md` when it becomes a dependency.

---

### Task 1: Create the private Git crate and deterministic snapshot model

**Files:**
- Modify: `cli/Cargo.toml`
- Modify: `cli/Cargo.lock`
- Create: `cli/crates/git/Cargo.toml`
- Create: `cli/crates/git/src/lib.rs`
- Create: `cli/crates/git/src/model.rs`
- Test: `cli/crates/git/tests/model.rs`

**Interfaces:**
- Produces: `GitHistorySnapshot`, `SelectedRef`, `Completeness`, `CommitFact`, `PathChange`, `CoChangeFact`, `ContributorIdentity`, `KnowledgeSignal`, and `GitHistoryError`.
- Produces: `GitHistorySnapshot::canonical_json(&self) -> Result<String, GitHistoryError>`.

- [ ] **Step 1: Write failing model tests**

```rust
#[test]
fn canonical_json_is_stable_and_round_trips() {
    let snapshot = GitHistorySnapshot::empty_for_test();
    let first = snapshot.canonical_json().expect("canonical JSON");
    let second = snapshot.canonical_json().expect("canonical JSON");
    assert_eq!(first, second);
    assert_eq!(serde_json::from_str::<GitHistorySnapshot>(&first).unwrap(), snapshot);
    assert!(first.contains("modernlink.git-history/v1alpha1"));
}
```

- [ ] **Step 2: Run the model test and observe the missing-type failure**

Run: `cargo test --manifest-path cli/Cargo.toml -p git --test model`

Expected: compilation fails because package `git` and `GitHistorySnapshot` do not exist.

- [ ] **Step 3: Add the workspace member and private manifest**

```toml
[package]
name = "git"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
publish.workspace = true

[dependencies]
serde.workspace = true
serde_json.workspace = true
sha2.workspace = true
thiserror.workspace = true
```

Add `"crates/git"` to the `cli/Cargo.toml` member list. Do not change the root runtime workspace.

- [ ] **Step 4: Implement the canonical model**

Implement the public serializable types in `model.rs`, using `BTreeMap`/sorted vectors for all
collections that affect output. `GitHistorySnapshot::canonical_json` serializes the snapshot after
calling `normalize()` which sorts refs by name, commits by object ID, path changes by
`(commit_id, path)`, co-changes by `(left_path, right_path)`, identities by comparison key, and
limitations by stable code. `schema_version` is the literal
`modernlink.git-history/v1alpha1`.

- [ ] **Step 5: Run the model test and workspace compilation**

Run: `cargo test --manifest-path cli/Cargo.toml -p git --test model`

Expected: the command exits `0`; this is evidence only that the controlled model test executed.

- [ ] **Step 6: Commit the isolated model**

```text
git add cli/Cargo.toml cli/Cargo.lock cli/crates/git
git commit -m "feat(cli): add Git history evidence model"
```

### Task 2: Open repositories and resolve explicit ref scope through `gix`

**Files:**
- Modify: `cli/Cargo.toml`
- Modify: `cli/Cargo.lock`
- Modify: `cli/crates/git/Cargo.toml`
- Create: `cli/crates/git/src/repository.rs`
- Modify: `cli/crates/git/src/lib.rs`
- Test: `cli/crates/git/tests/repository.rs`

**Interfaces:**
- Produces: `HistoryOptions { ref_scope, max_commits, max_cochange_paths, mailmap }`.
- Produces: `open_repository(path: &Path) -> Result<gix::Repository, GitHistoryError>`.
- Produces: `resolve_refs(repo: &gix::Repository, scope: RefScope) -> Result<Vec<SelectedRef>, GitHistoryError>`.

- [ ] **Step 1: Write failing ref-scope tests using a local fixture repository**

```rust
#[test]
fn all_scope_records_head_branch_and_tag_targets() {
    let fixture = FixtureRepository::linear_with_branch_and_tag();
    let refs = resolve_refs(&fixture.repository(), RefScope::All).unwrap();
    assert!(refs.iter().any(|item| item.name == "HEAD"));
    assert!(refs.iter().any(|item| item.name == "refs/heads/main"));
    assert!(refs.iter().any(|item| item.name == "refs/tags/v1"));
    assert!(refs.iter().all(|item| !item.target_id.is_empty()));
}
```

`FixtureRepository` creates repository objects through `gix` APIs or fixture objects. It must not
capture or parse `git` executable output.

- [ ] **Step 2: Run the repository test and observe the missing-module failure**

Run: `cargo test --manifest-path cli/Cargo.toml -p git --test repository`

Expected: compilation fails because `HistoryOptions`, `RefScope`, and `resolve_refs` do not exist.

- [ ] **Step 3: Add the structured backend dependency**

Add this exact workspace dependency and inherit it from `git`:

```toml
gix = "0.87.0"
```

Record the resolved version, license, and upstream URL in `cli/THIRD_PARTY_NOTICES.md`; do not
copy upstream source into this repository.

- [ ] **Step 4: Implement read-only open and ref selection**

Use `gix::open(path)` or the equivalent read-only open API. Enumerate references through `gix`,
resolve symbolic targets, and return stable `SelectedRef` records. Ref failures become
`Completeness` records only when other selected refs can still form a coherent result; a path that
is not a repository returns `GitHistoryError::NotRepository` without producing a snapshot.

- [ ] **Step 5: Run repository tests**

Run: `cargo test --manifest-path cli/Cargo.toml -p git --test repository`

Expected: the command exits `0`; it does not prove behavior for remote, corrupt, or enterprise
repositories.

- [ ] **Step 6: Commit structured repository access**

```text
git add cli/Cargo.toml cli/Cargo.lock cli/THIRD_PARTY_NOTICES.md cli/crates/git
git commit -m "feat(cli): resolve Git history refs with gix"
```

### Task 3: Traverse commit facts, preserve identity ambiguity, and enforce history limits

**Files:**
- Create: `cli/crates/git/src/traverse.rs`
- Modify: `cli/crates/git/src/lib.rs`
- Modify: `cli/crates/git/src/model.rs`
- Test: `cli/crates/git/tests/traverse.rs`

**Interfaces:**
- Produces: `collect_history(path: &Path, options: &HistoryOptions) -> Result<GitHistorySnapshot, GitHistoryError>`.
- Produces: one `CommitFact` per reachable object ID, with all parent IDs and explicit selected diff-parent policy.
- Produces: `Completeness { code: "commit-limit-reached", .. }` once `max_commits` stops traversal.

- [ ] **Step 1: Write failing traversal tests**

```rust
#[test]
fn traversal_keeps_raw_identity_and_records_commit_limit() {
    let fixture = FixtureRepository::three_commit_history_with_two_display_names();
    let report = collect_history(
        fixture.path(),
        &HistoryOptions { max_commits: 2, ..HistoryOptions::all() },
    ).unwrap();
    assert_eq!(report.commits.len(), 2);
    assert!(report.completeness.iter().any(|item| item.code == "commit-limit-reached"));
    assert!(report.contributors.iter().any(|identity| identity.raw_name == "Alice Example"));
    assert!(report.contributors.iter().any(|identity| identity.raw_name == "A. Example"));
}
```

- [ ] **Step 2: Run the traversal test and observe the missing-collector failure**

Run: `cargo test --manifest-path cli/Cargo.toml -p git --test traverse`

Expected: compilation fails because `collect_history` and the fixture builder do not exist.

- [ ] **Step 3: Implement deterministic graph walk**

Traverse each selected ref using `gix` commit/object APIs. Deduplicate commit object IDs across
refs, preserve reachability on the commit record, and stop at `max_commits` with a completeness
record. Store raw author/committer fields, compute comparison keys from normalized emails only,
and default `message_fingerprint` to SHA-256 of the raw message bytes without serializing the
message itself.

- [ ] **Step 4: Add merge-parent policy tests and implementation**

Add a fixture merge commit and assert that all parent IDs remain present while
`diff_parent_policy == "first-parent"`. Implement this literal policy in `CommitFact`; do not
derive path metrics from every parent in the first release.

- [ ] **Step 5: Run traversal tests**

Run: `cargo test --manifest-path cli/Cargo.toml -p git --test traverse`

Expected: the command exits `0`; output remains limited to controlled fixture evidence.

- [ ] **Step 6: Commit commit-history collection**

```text
git add cli/crates/git
git commit -m "feat(cli): collect deterministic Git commit history"
```

### Task 4: Emit path changes, bounded co-change evidence, and transparent metrics

**Files:**
- Create: `cli/crates/git/src/diff.rs`
- Create: `cli/crates/git/src/metrics.rs`
- Modify: `cli/crates/git/src/lib.rs`
- Modify: `cli/crates/git/src/model.rs`
- Test: `cli/crates/git/tests/diff.rs`
- Test: `cli/crates/git/tests/metrics.rs`

**Interfaces:**
- Produces: `collect_path_changes(...) -> Result<Vec<PathChange>, GitHistoryError>`.
- Produces: `derive_co_changes(changes: &[PathChange], cap: usize) -> (Vec<CoChangeFact>, Vec<Completeness>)`.
- Produces: `derive_metrics(snapshot: &GitHistorySnapshot) -> Vec<HistoryMetric>`.

- [ ] **Step 1: Write failing path and cap tests**

```rust
#[test]
fn cochange_cap_records_skipped_expansion_instead_of_silent_truncation() {
    let changes = vec![
        change("c1", "a.java"), change("c1", "b.java"), change("c1", "c.java"),
    ];
    let (pairs, limitations) = derive_co_changes(&changes, 2);
    assert!(pairs.is_empty());
    assert!(limitations.iter().any(|item| item.code == "cochange-cap-exceeded"));
}
```

- [ ] **Step 2: Run tests and observe the missing-function failure**

Run: `cargo test --manifest-path cli/Cargo.toml -p git --test diff --test metrics`

Expected: compilation fails because the diff/metric functions do not exist.

- [ ] **Step 3: Implement structured first-parent path deltas**

Use `gix` tree/diff APIs to collect additions, deletions, modifications, type changes, and
executable-bit changes from each commit to its first parent. For root commits, compare against an
empty tree. Deliberately configure `rename_detection = "disabled"` in this first slice. Line
counts must report whether they are measured or unavailable; a diff-processing error must not be
converted into zero churn.

- [ ] **Step 4: Implement co-change and metric derivation**

Sort and deduplicate paths per commit before pairing. If the per-commit path count exceeds the
configured cap, emit no pairs for that commit and emit `cochange-cap-exceeded` with the count and
cap. Derive only documented quantities: changed-commit count, additions, deletions, last observed
change, identity-observation count, and coupling ratio with its numerator/denominator recorded.

- [ ] **Step 5: Run controlled diff and metric tests**

Run: `cargo test --manifest-path cli/Cargo.toml -p git --test diff --test metrics`

Expected: the command exits `0`; rename lineage remains explicitly unavailable.

- [ ] **Step 6: Commit path/coupling evidence**

```text
git add cli/crates/git
git commit -m "feat(cli): derive Git change coupling evidence"
```

### Task 5: Add local cache identity and safe artifact publication

**Files:**
- Create: `cli/crates/git/src/cache.rs`
- Modify: `cli/crates/git/src/lib.rs`
- Modify: `.gitignore`
- Test: `cli/crates/git/tests/cache.rs`

**Interfaces:**
- Produces: `CacheKey::from_snapshot_inputs(repository, options, refs) -> CacheKey`.
- Produces: `load_cached(path, key) -> Result<Option<GitHistorySnapshot>, GitHistoryError>`.
- Produces: `store_cached(path, key, snapshot) -> Result<(), GitHistoryError>`.

- [ ] **Step 1: Write failing cache-isolation tests**

```rust
#[test]
fn cache_key_changes_when_selected_ref_target_changes() {
    let first = CacheKey::from_parts("sha1", [("refs/heads/main", "a".repeat(40))], 10, 100);
    let second = CacheKey::from_parts("sha1", [("refs/heads/main", "b".repeat(40))], 10, 100);
    assert_ne!(first, second);
}
```

- [ ] **Step 2: Run the cache test and observe the missing-type failure**

Run: `cargo test --manifest-path cli/Cargo.toml -p git --test cache`

Expected: compilation fails because `CacheKey` does not exist.

- [ ] **Step 3: Implement cache key and atomic writes**

Hash object format, selected `(ref, target)` pairs, collector version, ref scope, commit cap,
co-change cap, and mailmap mode with SHA-256. Write canonical JSON to a same-directory temporary
file then rename it into `.modernlink/cache/git/<key>.json`. Reject a cache path outside the
supplied repository root after canonical containment checks. Add `.modernlink/cache/` to the root
ignore rules without ignoring reviewed `modernlink/` artifacts.

- [ ] **Step 4: Run cache tests**

Run: `cargo test --manifest-path cli/Cargo.toml -p git --test cache`

Expected: the command exits `0`; tests demonstrate cache-key behavior only.

- [ ] **Step 5: Commit local cache support**

```text
git add .gitignore cli/crates/git
git commit -m "feat(cli): cache Git history evidence locally"
```

### Task 6: Expose `modernlink history` and integrate optional analyzer history output

**Files:**
- Modify: `cli/crates/modernlink-cli/Cargo.toml`
- Modify: `cli/crates/modernlink-cli/src/main.rs`
- Create: `cli/crates/modernlink-cli/src/history_command.rs`
- Modify: `cli/crates/modernlink-cli/tests/analyze_command.rs`
- Create: `cli/crates/modernlink-cli/tests/history_command.rs`

**Interfaces:**
- Produces: `modernlink history <repository> --output <path>`.
- Produces: a stdout JSON receipt with `report`, `repository_digest`, `schema_version`, and completeness count.
- Produces: `modernlink analyze <repository> --history --output <path>` with sibling `git-history.json`.

- [ ] **Step 1: Write failing CLI integration tests**

```rust
#[test]
fn history_command_writes_canonical_history_artifact() {
    let fixture = FixtureRepository::linear_with_branch_and_tag();
    let output = fixture.path().join("history.json");
    let run = Command::new(env!("CARGO_BIN_EXE_modernlink"))
        .args(["history", fixture.path().to_str().unwrap(), "--output"])
        .arg(&output)
        .output()
        .expect("run modernlink history");
    assert!(run.status.success(), "stderr: {}", String::from_utf8_lossy(&run.stderr));
    let json: serde_json::Value = serde_json::from_slice(&fs::read(output).unwrap()).unwrap();
    assert_eq!(json["schema_version"], "modernlink.git-history/v1alpha1");
}
```

- [ ] **Step 2: Run the CLI test and observe clap’s unknown-subcommand failure**

Run: `cargo test --manifest-path cli/Cargo.toml -p modernlink-cli --test history_command`

Expected: test fails because `history` is not a ModernLink subcommand.

- [ ] **Step 3: Implement typed history arguments and atomic output**

Add a `History` command variant with typed `--refs`, `--max-commits`, `--max-cochange-paths`, and
`--mailmap` arguments. It calls only the `git` crate API, writes the canonical artifact atomically,
and prints one JSON receipt. Input/state errors preserve the existing structured diagnostic format.

- [ ] **Step 4: Add optional analyzer integration after standalone command behavior exists**

Add `--history` to `Analyze`. Given `--output path/to/analysis.json`, write the history artifact
to `path/to/git-history.json`, return both paths/digests in stdout, and do not change the existing
`modernlink.analysis/v1alpha1` schema.

- [ ] **Step 5: Run CLI integration tests**

Run: `cargo test --manifest-path cli/Cargo.toml -p modernlink-cli --test history_command --test analyze_command`

Expected: the command exits `0`; this demonstrates controlled local command behavior, not
enterprise-history accuracy.

- [ ] **Step 6: Commit command integration**

```text
git add cli/crates/modernlink-cli cli/crates/git cli/Cargo.lock
git commit -m "feat(cli): expose Git history analysis"
```

### Task 7: Run the first end-to-end local evidence exercise and reconcile documentation

**Files:**
- Modify: `docs/modernization/2026-08-23-git-evolution-intelligence-design.md`
- Modify: `docs/modernization/2026-08-23-modernization-cli-plugin-design.md`
- Modify: `docs/ROADMAP.md`
- Modify: `docs/FEATURES.md`
- Create: `docs/evidence/<date>-cli-git-history.md`

**Interfaces:**
- Consumes: the public `modernlink history` command and its JSON artifact.
- Produces: a revision-scoped evidence record that labels what was executed and what remains unproven.

- [ ] **Step 1: Run the controlled command against a fixture and a local non-destructive repository**

Run:

```text
cargo run --manifest-path cli/Cargo.toml -p modernlink-cli -- history <fixture-repository> --output <temporary-output>
cargo run --manifest-path cli/Cargo.toml -p modernlink-cli -- history . --output <temporary-output>
```

Record exact command, revision, output schema, and limits. Do not record commit messages, email
addresses, source content, or private remote URLs in the evidence document.

- [ ] **Step 2: Update status language without overstating scope**

Mark the Git-history foundation as implemented only to the precise extent demonstrated by code and
machine facts. Keep contributor support, semantic ownership, rename lineage, and real enterprise
history interpretation explicitly unproven.

- [ ] **Step 3: Run static checks for the isolated CLI workspace**

Run:

```text
cargo fmt --manifest-path cli/Cargo.toml --all -- --check
cargo test --manifest-path cli/Cargo.toml --workspace
cargo clippy --manifest-path cli/Cargo.toml --workspace --all-targets -- -D warnings
```

Report each result as a machine fact; none establishes product correctness.

- [ ] **Step 4: Commit documentation/evidence reconciliation**

```text
git add docs cli
git commit -m "docs: record Git history CLI evidence"
```

## Plan self-review

This plan covers every design section: structured backend, private crate naming, local-only
operation, deterministic snapshot, ref completeness, identity ambiguity, first-parent changes,
bounded co-change, transparent metrics, cache, standalone and analyzer commands, tests, and
documentation evidence. It deliberately excludes future semantic symbol history, hosting APIs,
network operations, and contributor ranking because the design marks them outside this slice.
