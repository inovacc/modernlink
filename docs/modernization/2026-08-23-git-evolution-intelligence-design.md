# ModernLink Git Evolution Intelligence Design
<!-- rev:005 (RFC 3339) 2026-08-24T01:18:46Z -->

## Status

Approved direction. This design defines the first implementation unit of the ModernLink CLI
modernization product: deterministic, local Git-history facts that enrich the evidence graph
without parsing human-oriented `git` command output. It is intentionally separate from domain
inference and AI interpretation.

## Goal

Enable ModernLink to answer evidence-backed evolution questions about a repository:

- which files, packages, modules, and symbols change together;
- which paths have concentrated or volatile change history;
- where history indicates a likely knowledge holder who can help a modernization effort; and
- which architectural hypotheses are supported or challenged by historical coupling.

The feature supports people; it does not rank developer productivity, infer employment status, or
claim code ownership as a fact.

## Product boundary

```text
local Git repository
        |
        v
private Rust `git` crate
        |
        +-- immutable history facts
        +-- path/change coupling facts
        +-- identity observations
        +-- repository/ref completeness facts
        |
        v
Git history snapshot (`modernlink.git-history/v1alpha1`)
        |
        +-------------------+
        |                   |
        v                   v
Evidence Graph       Agent/plugin interpretation
        |                   |
        v                   v
facts only       support candidates and migration hypotheses
```

The Git crate never decides that someone owns a domain, is the best engineer, or must be
contacted. Those are hypotheses or human decisions made above this layer.

## Crate and backend decision

Create `cli/crates/git/` with Cargo package and Rust crate name `git`. All ModernLink CLI crates
are private; every manifest must inherit or set `publish = false`. New private crates do not carry
the `modernlink-` prefix.

The primary backend is the Rust `gix` ecosystem. It reads repositories, object databases,
references, commit graphs, and diffs as structured Rust data. ModernLink must not invoke `git log`,
`git shortlog`, `git blame`, or `git diff` and then parse their terminal output for analyzer facts.

An external Git executable may be used only as an explicit, separately labeled fallback for a
capability that the selected `gix` API cannot supply. Such a fallback must produce a structured
adapter result, preserve raw exit status separately from facts, record reduced precision, and
never silently replace the primary backend. The initial Git-history slice has no fallback.

`E:\projects_old\gitc` is a concepts-and-fixtures reference only. No source, C shim, history
rewriter, secret scanner, or launcher code is copied into ModernLink. Its dirty worktree is never
modified by this work.

## Inputs and repository completeness

The collector accepts a local repository path. It must support a normal working tree, linked
worktree, and bare repository when the underlying backend supports the form. It never clones,
fetches, pushes, checks out, changes refs, updates submodules, or modifies the repository.

The default ref scope is every reachable local reference (`HEAD`, local branches, tags, and remote
tracking branches present locally). The snapshot records the resolved reference name and target
object ID for every selected ref. Command-line ref selection is explicit and becomes part of the
snapshot identity.

The collector reports, rather than hides:

- shallow-repository state and shallow boundary;
- unreadable or unsupported objects;
- dangling or symbolic refs that cannot resolve;
- selected refs that are absent locally; and
- any cancelled or budget-limited traversal.

An incomplete history snapshot is still useful evidence, but every derived metric retains the
completeness limitations that produced it.

## Deterministic snapshot contract

The `git` crate produces canonical JSON with schema version
`modernlink.git-history/v1alpha1`. Object IDs are retained in their native Git hexadecimal form;
ModernLink IDs are stable hashes of normalized inputs and never depend on traversal order, local
paths, timestamps generated during collection, or machine-specific Git configuration.

```text
GitHistorySnapshot
  schema_version
  repository_identity
  selected_refs[]
  completeness[]
  commits[]
  path_changes[]
  contributors[]
  co_changes[]
  metrics[]
  evidence[]
```

Each emitted fact includes an evidence ID, collector/version, source object/ref, and precision.
Facts are sorted by stable identifier before serialization.

### Commit facts

A `CommitFact` records:

- commit object ID, parent object IDs, tree object ID, author and committer timestamps/offsets;
- author and committer identity observations exactly as stored, plus a conservative normalized
  comparison key;
- reachable refs that led to the commit;
- parent count, merge classification, and selected diff-parent policy; and
- a message fingerprint by default, not the message text.

Commit messages are withheld from the default artifact because they can contain proprietary or
sensitive material. A future, explicit local-only option may include redacted message terms; it
cannot upload them or make them telemetry.

### Identity observations and support candidates

`ContributorIdentity` represents observed author/committer metadata, not a verified person. The
collector preserves the decoded header name and email separately and lowercases the email only in
the comparison key. It never joins two different email addresses because their names match.
`--mailmap repo` is accepted as an intent and cache input, but this implementation records
`mailmap-not-applied` rather than silently claiming resolution until a structured mailmap backend
and fixture coverage exist.

The deterministic layer emits path-level `KnowledgeSignal` facts such as recent touch count,
distinct-change count, and last observed touch. It does not emit a single contributor score.
The agent layer may present a `CandidateSupportContact` only as a hypothesis with the signals,
missing-history caveats, and an explicit instruction to ask the team rather than assume authority.

### Changes and co-change coupling

For every selected commit, the collector records path-level additions, deletions, modifications,
type changes, and executable-bit changes available from the structured diff. Merge commits retain
all parents but use the first parent for default path metrics. This makes the metric reproducible
and avoids multiplying a merge's change count. The policy is recorded on every affected fact.

Directory tree entries are excluded from file-level churn and co-change facts because they are not
diffable blobs and would otherwise create false coupling. Nested file paths remain recorded. A
controlled two-parent merge fixture verifies the first-parent policy; it does not establish that
merge semantics from a representative enterprise repository have been assessed.

Rename/copy similarity is deliberately disabled in the first slice even though the selected
backend can be configured to infer it. A delete and add remain distinct paths and the snapshot
records `rename_detection = disabled`; agents must not claim a file lineage from that pair.

Line additions and deletions come from the structured blob-diff API. Each path fact carries a
`line_count_status`: `measured` means the numbers are available; `unavailable` means the backend
could not produce meaningful line counts (for example, a binary diff) and the numeric fields are
zero only as placeholders, never evidence of zero churn. A diff-processing error aborts the
collection rather than silently fabricating a count.

`CoChangeFact` is emitted for unique unordered path pairs changed in the same commit. It records
the contributing commit IDs, count, ref scope, and traversal completeness. Aggregation is bounded
by a configurable maximum changed-path count per commit; skipped expansions become explicit
limitations rather than partial silent metrics.

### Metrics

Metrics summarize observed history without fake precision:

- path churn: changed-commit count, additions, deletions, and last observed change;
- change concentration: share of observed changes over the selected history;
- co-change count and normalized coupling ratio;
- contributor diversity: number of distinct identity observations, not people; and
- history coverage: selected refs, reachable commits, and stated traversal limitations.

Metrics never label a path “risky,” “owned,” “critical,” or “abandoned.” Higher layers combine
them with static evidence, runtime evidence, and human review.

## Integration with ModernLink evidence

The current analyzer report remains `modernlink.analysis/v1alpha1`. The Git collector is produced
independently first and is attached through a versioned adapter after the planned shared `model`
crate exists. Until that extraction, `modernlink analyze --history` writes two sibling canonical
artifacts:

```text
.modernlink/
  evidence/
    analysis.json
    git-history.json
```

The CLI prints a small machine-readable result that identifies both artifact paths and their
digests. It does not fold Git facts into the current inline analyzer structs and create a second
incompatible evidence model.

The later Evidence Graph adapter creates only lower-layer edges:

- `COMMIT_PARENT`;
- `COMMIT_TOUCHES_PATH`;
- `REF_REACHES_COMMIT`;
- `IDENTITY_AUTHORED_COMMIT`; and
- `PATH_CO_CHANGED_WITH_PATH`.

Candidate contexts, support contacts, migration priority, and seams remain derived claims with
their own evidence references and confidence.

## Command surface

The first user-facing command is:

```text
modernlink history <repository> --output <path>
```

It emits only canonical JSON to a new output path and a concise JSON receipt to stdout. Publication
uses a same-directory temporary file and refuses to replace an existing report. It supports:

```text
--refs all|head|local|<explicit ref>
--max-commits <positive integer>
--max-cochange-paths <positive integer>
--mailmap off|repo
```

`modernlink analyze <repository> --history --output <analysis-path>` invokes the same library API
and writes the sibling `git-history.json` artifact without changing the
`modernlink.analysis/v1alpha1` schema. Both target paths are checked before publication; the
operation never silently replaces either artifact.

Future commands map the same data to the lifecycle taxonomy:

```text
modernlink inspect <repository> --history
modernlink architecture --history
modernlink boundaries --history
modernlink seams --history
```

None of these commands may infer architecture solely from Git facts.

## Performance and cache

The collector is local-first and incremental. Cache entries live under
`.modernlink/cache/git/` and are ignored by Git. A cache key includes repository object-format,
selected ref targets, ref-scope options, collector version, traversal limits, and mailmap mode.
Changing any of those inputs invalidates the relevant entry.

Cache files contain only normalized local analysis artifacts. ModernLink sends no source,
commit metadata, identity metadata, or telemetry by default. A future hosted service requires a
separate explicit opt-in design and cannot reuse this collector's output implicitly.

## Safety and failure behavior

- Repository access is read-only.
- No automatic network operation is permitted.
- Invalid repository paths fail with a structured diagnostic and no output artifact.
- Unsupported object formats or incomplete traversal retain the facts that were safely observed
  and emit explicit completeness records when a coherent snapshot can be produced.
- Credential-bearing remote URLs are not emitted. Remote names and host classes may be emitted
  only after redaction rules exist in the shared evidence model.
- History size limits are explicit user-visible facts, never hidden truncation.

## Test and acceptance evidence

The implementation uses controlled local repositories created through structured Rust APIs or
versioned fixture objects, not parsed `git` command output. The minimum fixture suite covers:

1. linear commits touching one and multiple paths;
2. a merge commit with two parents and first-parent metric policy;
3. divergent author display names and exact/ambiguous email observations;
4. a shallow or explicitly limited history result;
5. a commit above the co-change expansion cap;
6. deterministic byte-for-byte output across repeated traversal; and
7. a repository path that is not Git, which produces a stable structured error.

Acceptance requires evidence that `modernlink history` executes on controlled repositories and
that its output records incomplete history rather than presenting it as complete. Those machine
results do not prove its conclusions are correct for a real enterprise repository; that requires
human-authorized representative repositories and review.

## Non-goals for this slice

- Git hosting APIs, pull-request analysis, issue tracking, or network fetches.
- Git history rewriting, secret scanning, branch mutation, or deployment actions.
- Individual performance scoring, employment inference, or automatic contact recommendations.
- Semantic symbol-history tracking before the Java symbol model has stable identities.
- Rename similarity claims unless deterministic structured evidence is implemented and labeled.
