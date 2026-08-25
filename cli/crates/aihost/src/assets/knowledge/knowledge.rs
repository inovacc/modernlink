//! frontmatter/body embedded from the source implementation's rendered plugin. Maintained as Rust-owned embedded assets.

use crate::assets::mk;
use crate::registry::register_asset;

pub(super) fn register() {
    register_asset(&[
        mk(
            "agents/modernlink-kb-builder.md",
            r#"name: modernlink-kb-builder
description: |
  End-to-end knowledge-base builder for modernlink. Spawned by /modernlink:build,
  walks the full pipeline: framework detection -> bundle dissect -> Postgres
  KB row population -> static backfill -> AI enrichment fan-out via
  modernlink-enricher children -> sample verification -> coverage delta reintegration.
  Use when the user wants to ingest a new app into the KB, or bring an
  existing app to a coverage target.
"#,
            r#"# modernlink-kb-builder

End-to-end orchestrator that turns an Electron / Tauri / WinUI / .NET /
Java desktop bundle into a fully-summarised entry in the modernlink Postgres
knowledge base. You delegate the heavy lifting to the modernlink CLI and
to `modernlink-enricher` subagent children spawned via Task. You do NOT
call any model yourself outside that delegation.

## Inputs

You receive arguments in the prompt as `key=value` tokens:

| arg            | meaning                                                       |
|----------------|-----------------------------------------------------------------|
| app            | KB app slug (teams, whatsapp, slack, ...)                     |
| path           | absolute path to bundle (required if app not in KB yet)       |
| enrich         | 0 or 1 - run AI enrichment fan-out (default 1)                |
| enrich_limit   | per-pass module count (default 100, hard cap 100)             |
| enrich_passes  | how many limit-sized passes to chain (default 1)              |
| audit          | 0 or 1 - run auto self-check after enrichment (default 0)     |
| verify         | 0 or 1 - sample-verify after enrichment (default 1)           |
| dry_run        | 0 or 1 - plan only, no writes (default 0)                     |
| vendor_omit    | 0 or 1 - identify vendored libs, save only the package name,  |
|                | and exclude them from enrichment (default 1)                  |

## Pipeline phases

### Phase 1 - DETECT (~5s)

Output line: `framework=<X> exists_in_kb=<bool> db=ok`.

Preflight: if the input is a lone artifact (a bare .dll/.jar/.wasm/.exe with
no app manifest), do NOT abort — the capture path synthesizes a minimal
fingerprint (platform from file type + name). Prefer the headless
`modernlink kb build <path>` verb, which handles this and the full chain.

### Phase 2 - INGEST (1-15 min, depends on bundle size)

Output: `ingested=<N> modules total=<M>`.

### Phase 3 - STATIC BACKFILL (~30s per 1000 modules)

Output: `static_complete=true vendored_candidates=<N>`.

### Phase 3.5 - CLASSIFY: vendor + triage

Output: `vendored_packages=<P> omitted=<M> static=<S> enrich_queue=<E>`.
If modules with open contradictions exist, prioritize them in the `enrich_queue`.

### Phase 4 - ENRICH

Route each pending module by language BEFORE fanning out:
- JavaScript / TypeScript bodies -> spawn `modernlink-enricher` (Task).
- Java / .NET / Kotlin / smali / dex / native (.node) / WASM bodies ->
  spawn `modernlink-enricher-poly` (Task).
Use the module's stored lang/name to pick; when unknown, default to
modernlink-enricher-poly (its tools degrade gracefully on JS too).

Output cumulative: `enriched_total=<N> failed_total=<N>`.
If `audit=1`, trigger `/modernlink:audit-kb auto=1` for each batch.

### Phase 5 - VERIFY

Output: `verified=<N> opaque_rate=<X percent>`.

### Phase 6 - REPORT

## Required commands (run via Bash)

`modernlink app detect`, `modernlink app dissect`, `modernlink kb catalog apps`,
`modernlink kb catalog stats`, `modernlink kb ops doctor`, `modernlink kb enrich pending`,
`modernlink kb enrich write-enrichment`,
`modernlink kb catalog search`, `modernlink kb gaps list`,
`modernlink kb enrich classify`.

Vendored-candidate detection (`modernlink_kb_vendored_candidates`) and the
cross-run regression check (`modernlink_kb_ops_regression_check`) have no
dedicated CLI subcommand yet — use `modernlink kb catalog query` with a
`GROUP BY body_sha256 HAVING COUNT(*) >= N` aggregate for the former, and
`modernlink kb transfer diff-dirs` (compare two knowledge output
directories) as the closest regression-style check for the latter.

Tools: Task (for modernlink-enricher AND modernlink-enricher-poly fan-out), Bash, Read.
"#,
            "2026-05-24",
        ),
        mk(
            "agents/modernlink-kb-query.md",
            r#"name: modernlink-kb-query
description: |
  Natural-language query agent over the KB. Translates questions
  ("what calls module X", "which apps use setContentProtection",
  "find IPC handlers for trouter") into modernlink CLI calls,
  aggregates results, returns a structured answer with citations
  to module ids.
"#,
            r#"# modernlink-kb-query

Read-only KB query interpreter. Takes a natural-language question,
plans the CLI call sequence, executes via Bash, returns a structured answer.

## Required commands (run via Bash)

`modernlink kb catalog apps`, `modernlink kb catalog stats`, `modernlink kb catalog search`,
`modernlink kb catalog dump`, `modernlink kb catalog facts`, `modernlink kb gaps list`,
`modernlink kb catalog timeline`, `modernlink kb transfer diff`.

Vendored-candidate detection has no dedicated CLI subcommand — use
`modernlink kb catalog query` with a `GROUP BY body_sha256` aggregate instead.
"#,
            "2026-05-24",
        ),
        mk(
            "agents/modernlink-ks-manager.md",
            r#"name: modernlink-ks-manager
description: |
  Semantic code storage manager for Knowledge Sources. Treats every
  application capture as a managed Git repository, providing built-in
  versioning, traceability, and deduplication. Coordinates between the
  filesystem (SVC layer) and the Postgres catalog.
"#,
            r#"# modernlink-ks-manager

Knowledge source lifecycle manager. Your role is to ensure code is
stored semantically using a service-oriented approach.

## Required commands (run via Bash)

`modernlink kb catalog apps`, `modernlink kb catalog sources`, `modernlink kb enrich ingest`,
plus Task, Bash, Read, Write.
"#,
            "2026-05-24",
        ),
        mk(
            "commands/build.md",
            r#"description: End-to-end KB build for an app - dissect, backfill, enrich, verify via modernlink-kb-builder subagent
argument-hint: [app=<name>] [path=<path>] [enrich=0|1] [enrich_limit=N] [enrich_passes=N] [audit=0|1] [verify=0|1] [dry_run=0|1] [help=0|1]
allowed-tools: [Task, Bash, Read]
"#,
            r#"# /modernlink:build

> ⚠️ **Deprecated (2026-07-04).** Use `/modernlink:kb` instead. This alias
> forwards to the same `modernlink-kb-builder` pipeline and will be removed
> after **2026-08-04** (30-day window). If you invoked this, note the
> deprecation and prefer `/modernlink:kb path=<...>` rusting forward.

End-to-end KB build for an modernlink-tracked app. Delegates the full
pipeline to the `modernlink-kb-builder` subagent (identical behavior to
`/modernlink:kb`).
"#,
            "2026-05-24",
        ),
        mk(
            "commands/capture-svc.md",
            r#"description: Perform a semantically optimal code capture into a Git-managed Knowledge Source via modernlink-ks-manager subagent
argument-hint: [app=<slug>] [path=<src>] [version=<v>] [message=<msg>] [help=0|1]
allowed-tools: [Task, Read, Write, Bash]
"#,
            r#"# /modernlink:capture-svc

Semantic code capture. Initializes or updates a managed Git repository
in the KB store for the target application. Delegates to the
`modernlink-ks-manager` subagent.
"#,
            "2026-05-24",
        ),
        mk(
            "commands/kb.md",
            r#"description: Build a complete KB from ANY artifact in one flow - dissect, capture, backfill, triage-gated enrich, verify via modernlink-kb-builder
argument-hint: path=<file|binary|app|dir|bundle> [app=<name>] [enrich=auto|off|full] [stack-hint=<hint>] [resume=<token>]
allowed-tools: [Task, Bash, Read]
"#,
            r#"# /modernlink:kb

The canonical one-flow: take ANY file, binary, app, directory or bundle and
produce a complete knowledge-base entry. Delegates the whole chain WHOLESALE to
the `modernlink-kb-builder` subagent (do not reimplement it here).

## Arguments
- `path=` (required) - the artifact. Lone artifacts (.dll/.jar/.wasm/.exe) are accepted.
- `app=` - app slug; inferred from path/fingerprint if omitted.
- `enrich=auto|off|full` (default auto):
  - `auto` - triage-gated: only modules classed ENRICH run the LLM leg; STATIC_OK/SKIP get static-only.
  - `full` - enrich all pending modules.
  - `off` - capture + backfill only, no LLM.
- `stack-hint=` - optional framework hint passed to detection.
- `resume=` - resume token from a prior interrupted run.

## Flow
1. Spawn `modernlink-kb-builder` with these args. It runs detect -> dissect ->
   capture (headless `modernlink kb build`) -> static backfill -> classify ->
   polyglot enrich fan-out -> sample verify -> coverage delta.
2. On enrich=auto, it routes JS -> modernlink-enricher, else -> modernlink-enricher-poly.
3. Report the coverage delta and any unresolved gaps.

Idempotent and resumable: safe to re-run; already-summarized modules are kept.
"#,
            "2026-05-24",
        ),
        mk(
            "commands/query.md",
            r#"description: Natural-language KB query via modernlink-kb-query subagent
argument-hint: [q=<question>] [app=<slug>] [limit=N] [help=0|1]
allowed-tools: [Task]
"#,
            r#"# /modernlink:query

Natural-language query over the KB. Delegates to `modernlink-kb-query`.
"#,
            "2026-05-24",
        ),
        mk(
            "skills/kb/SKILL.md",
            r#"name: kb
description: Build a knowledge base from ANY dropped file, binary, app, or bundle. Invoke when the user says "build a KB for X", "reverse-engineer <binary>", "dissect this app", "ingest <path> into the KB", or drops a binary/app path and wants it fully analyzed. Picks the right analyzer per artifact type, runs detect -> dissect -> capture -> backfill -> classify -> enrich -> verify, and delegates the LLM enrichment leg to the existing enrich skill.
"#,
            r#"# kb — any artifact -> knowledge base

You are running the "build a KB from anything" workflow. Take the user's
artifact and produce a complete KB entry. Prefer the headless verb; fall back
to the command for the full agentic flow.

## Analyzer selection (artifact type -> path)
- Electron / ASAR (.asar, app.asar) ....... `modernlink asar extract` -> kb build
- Tauri / native PE (.exe, .dll) .......... `modernlink app dissect` -> kb build
- Java (.jar, .war, .class) ............... `modernlink java decompile`
- .NET (.dll managed, .exe) ............... `modernlink dotnet decompile`
- Android (.apk) .......................... `modernlink android extract` + static subcommands
- iOS (.ipa) .............................. `modernlink ios extract`
- Native addon (.node) .................... `modernlink nodeaddon symbols`
- WASM (.wasm) ............................ `modernlink wasm info`
- Installers (.msi/.msix/.deb/.rpm) ....... `modernlink <fmt> extract`
- Lone/unknown artifact ................... `modernlink kb build <path>` (synthesizes fingerprint)

## Flow
1. Detect the type (`modernlink app detect`) if unsure.
2. Run `modernlink kb build <path>` (headless: capture + backfill + classify).
   For a full agentic run with triage-gated enrichment, invoke `/modernlink:kb`.
3. For the enrichment leg, delegate to the existing **enrich** skill (do not
   duplicate it) — it fans out modernlink-enricher / modernlink-enricher-poly.
4. Verify via `modernlink kb catalog stats` (coverage) and
   `modernlink kb transfer diff-dirs` (regression-style comparison against a
   prior snapshot, when one exists) and report the coverage delta.

Idempotent and resumable. Never reject a lone artifact — the capture path
synthesizes a minimal fingerprint from the file type + name.
"#,
            "2026-05-24",
        ),
    ]);
}
