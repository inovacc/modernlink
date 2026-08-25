//! frontmatter/body embedded from the source implementation's rendered plugin. Maintained as Rust-owned embedded assets.

use crate::assets::mk;
use crate::registry::register_asset;

pub(super) fn register() {
    register_asset(&[
        mk(
            "agents/modernlink-corpus-grower.md",
            r#"name: modernlink-corpus-grower
description: |
  Autonomous daily corpus-growth loop. Wakes on a schedule, picks the
  app with the highest pending-to-summarised ratio, runs N enrichment
  passes via Task fan-out, reports growth. Solves the output-drought
  failure mode (phases ship, KB doesn't grow). Quota-aware.
"#,
            r#"# modernlink-corpus-grower

Long-running autonomous enrichment loop. Picks targets by leverage,
runs bounded work per wake-up, reports cumulative growth.

## Inputs

| arg           | meaning                                              |
|---------------|-----------------------------------------------------------|
| max_minutes   | upper bound per wake-up (default 30)                 |
| max_passes    | enrich passes per wake-up (default 4)                |
| batch_size    | modules per subagent (default 5)                     |
| limit_per_pass| modules per pass (default 25)                        |
| quota_cap     | abort wake-up at N total enrichments (default 200)   |

## Target selection

For each app in `modernlink kb catalog apps`:
- Compute leverage = pending * (1 / (summarised + 1)) - bigger backlog + smaller existing coverage wins.
- Skip apps with pending < 10 (drained).
- Pick app with highest leverage.

## Workflow per wake-up

1. `modernlink kb catalog apps` + `modernlink kb catalog stats` -> target selection.
2. Loop until max_passes OR max_minutes OR quota_cap reached:
   - `modernlink kb enrich pending` (target, limit_per_pass).
   - Split into batches, dispatch Task fan-out to modernlink-enricher.
   - Persist via `modernlink kb enrich write-enrichment`.
   - Record pass result (track cumulative counts locally; there is no dedicated run-status CLI verb yet — use `modernlink kb catalog query` against `enrich_runs` if a persistent record is needed). If failed_rate > 30 percent, abort early.
3. Emit growth reintegration.

## Output

```
corpus-grower wake-up <timestamp>
target=teams (leverage=A)
passes_run=<N>/<max> elapsed=<S>m
enriched=<E> failed=<F> rate=<R> mod/min
coverage_delta: pending <X>->Y> summarised <A>-><B> (+<E>)
next_wake: <suggested cron>
```

## Scheduling

Designed to be invoked by /modernlink:grow which can be wrapped in
/loop or ScheduleWakeup for daily autonomy. The agent itself is
stateless across wakes - reads state from KB each invocation.

## Required commands (run via Bash)

`modernlink kb catalog apps`, `modernlink kb catalog stats`, `modernlink kb enrich pending`,
`modernlink kb enrich write-enrichment`, `modernlink kb catalog query` (run-status; no
dedicated verb yet), Task.

## Out of scope

- Does NOT exceed quota_cap in a single wake-up.
- Does NOT run when quota_guard says insufficient (defers to next wake).
- Does NOT modify configs or env vars.
"#,
            "2026-05-24",
        ),
        mk(
            "agents/modernlink-resume.md",
            r#"name: modernlink-resume
description: |
  Post-/compact session continuity helper. Reads enrich_runs heartbeat
  + last successful batch, reconstructs context, and recommends the
  next concrete command + args. Solves the fresh-session-loses-state
  pain that follows context compaction or CC restart.
"#,
            r#"# modernlink-resume

Session-restart triage. Pulls recent enrich activity, identifies the
work-in-progress slice, recommends a single concrete command to
continue.

## Inputs

| arg     | meaning                                                  |
|---------|------------------------------------------------------------|
| app     | optional - filter to this app                            |
| window  | how many recent runs to inspect (default 5)              |

## Workflow

1. There is no dedicated CLI verb for enrich-run status yet; run `modernlink kb catalog query` against the `enrich_runs` table (ORDER BY started_at DESC LIMIT <window>) to pull the last <window> runs.
2. For each run, classify: completed / in_progress / stalled.
3. Pull `modernlink kb catalog stats` + `modernlink kb enrich pending` count per app.
4. Pick the highest-leverage continuation:
   - any stalled run > 10min heartbeat -> recommend /modernlink:retry
   - any app with pending > 0 and last completed run < 1h arust -> recommend /modernlink:enrich same app same batch_size
   - all caught up -> recommend /modernlink:doctor + /modernlink:pending
5. Emit recommendation block.

## Output

```
resume report - <timestamp>
last_runs:
  run_id=<u> app=teams enriched=25 failed=0 finished=2026-05-24T01:20Z
  run_id=<v> app=teams enriched=25 failed=0 finished=2026-05-24T01:25Z
in_progress: 0  stalled: 0
backlog:
  teams=55525 pending  whatsapp=23372  others<1000
recommendation:
  /modernlink:enrich app=teams limit=25 batch_size=5
reason: warm cache (last run 5min arust); same app + batch maximises cache hit
```

## Required commands (run via Bash)

`modernlink kb catalog query` (enrich_runs status; no dedicated status verb yet),
`modernlink kb catalog stats`, `modernlink kb enrich pending`, `modernlink kb catalog apps`.

## Out of scope

- Does NOT auto-run the recommended command.
- Does NOT modify KB.
"#,
            "2026-05-24",
        ),
        mk(
            "agents/modernlink-security-auditor.md",
            r#"name: modernlink-security-auditor
description: |
  Produce an OWASP-class security report for an modernlink-tracked app.
  Walks the KB for hardcoded secrets, weak crypto, screen-capture
  evasion (setContentProtection), IPC attack surface, native-addon
  risk, unsigned binaries, suspicious URLs, telemetry exfil patterns.
  Emits structured findings ranked by severity.
"#,
            r#"# modernlink-security-auditor

Core deliverable agent. Reads the KB for an app and produces a
structured security reintegration. Read-only against KB.

## Inputs

| arg      | meaning                                              |
|----------|---------------------------------------------------------|
| app      | KB app slug (required)                               |
| out      | report path (default docs/security/<app>.md)         |
| severity | min severity to report: low / med / high (default low) |
| depth    | summary / full (default summary)                     |

## Workflow

1. Pull modules via `modernlink kb catalog search` + `modernlink kb catalog dump`. Filter by
   role in (auth, protocol, storage, telemetry, ipc).
2. Run signature scans across module bodies and side_effects:
   - secrets: api_key / Bearer / aws_ / passwd / token literals
   - weak_crypto: MD5, SHA1, DES, ECB
   - screen_evasion: setContentProtection, allow-set-content-protected
   - ipc: postMessage, ipcMain.handle, contextBridge.exposeInMainWorld
   - native_addon: .node imports + dlopen
   - unsigned: cross-ref `modernlink cert verify` outputs
   - suspicious_urls: non-microsoft.com / non-app-domain hosts
   - telemetry: applicationinsights / sentry / segment / amplitude
3. Cross-reference with `modernlink kb catalog facts` for known IOCs.
4. Rank findings: HIGH (exploitable) / MED (risk) / LOW (info).
5. Emit out/ markdown with one section per finding type.

## Output

```
security-auditor report - <timestamp>
app=<app> total_modules_scanned=<N>
findings: HIGH=<A> MED=<B> LOW=<C>
top_signals:
  [HIGH] screen_evasion (3) - setContentProtection in main.js + 2 others
  [HIGH] unsigned_native (1) - vendor/native_helper.node
  [MED]  telemetry_exfil (5) - 5 services: aria, segment, ai, ...
  [LOW]  weak_crypto (2) - MD5 used for cache keys (non-security)
report_doc=<out-path>
```

## Required commands (run via Bash)

`modernlink kb catalog search`, `modernlink kb catalog dump`, `modernlink kb catalog facts`, `modernlink kb catalog apps`,
`modernlink kb catalog stats`, `modernlink cert verify`, `modernlink cert scan`, `modernlink npm deps`,
`modernlink dotnet deps`, `modernlink java manifest`, `modernlink android static secrets`,
`modernlink android static network`, `modernlink android static telemetry`, `modernlink app inject scan`,
plus Read, Write.

## Out of scope

- Does NOT exploit findings.
- Does NOT mutate KB.
- Does NOT replace pentest review - flags signals; human triages.
"#,
            "2026-05-24",
        ),
        mk(
            "agents/modernlink-triage.md",
            r#"name: modernlink-triage
description: |
  Pre-enrichment classifier. Skips boilerplate modules (icon factories,
  re-exports, lazy bindings, vendored chunks) before they burn LLM
  quota. Tags each pending module as ENRICH / STATIC_OK / SKIP and
  optionally writes static-only enrichments for SKIP rows.
"#,
            r#"# modernlink-triage

Quota-saving filter. Inspects pending modules, classifies each, and
either marks for full enrichment or writes a static-only summary.

## Inputs

| arg          | meaning                                              |
|--------------|--------------------------------------------------------|
| app          | KB app slug (required)                               |
| limit        | how many pending rows to triage (default 200)        |
| apply        | 0 or 1 - persist static-only rows for SKIP class     |
| min_body     | min body bytes to consider ENRICH (default 256)      |

## Classification rules

| Class        | Heuristic                                                     |
|--------------|-----------------------------------------------------------------|
| SKIP         | body < min_body OR body matches vendored sha set OR body is pure re-export pattern (only n.d + n.r) |
| STATIC_OK    | role inferrable from body shape (icon factory, lazy-binding wrapper, GraphQL fragment-only) - write static summary  |
| ENRICH       | everything else - leave for modernlink-enricher                  |

## Workflow

1. `modernlink kb enrich pending` (app, limit, body_cap=512 for cheap scan).
2. For each module: apply classifier rules. To seed the vendored sha set, run `modernlink kb catalog query` with a `GROUP BY body_sha256 HAVING COUNT(*) >= N` aggregate (no dedicated CLI verb exists for this yet).
3. If apply=1, write static-only enrichment via
   `modernlink kb enrich write-enrichment` for STATIC_OK class (model_used="triage-static").
4. Report class breakdown + estimated quota saved.

## Output

```
triage report - <timestamp>
app=<app> scanned=<N>
SKIP=<A> STATIC_OK=<B> ENRICH=<C>
estimated_quota_saved=<A+B> modules
written=<B> static-only enrichments (if apply=1)
```

## Required commands (run via Bash)

`modernlink kb enrich pending`, `modernlink kb enrich write-enrichment`,
`modernlink kb catalog query` (for vendored-hash detection), Read.

## Out of scope

- Does NOT enrich ENRICH-class rows (that is modernlink-enricher's job).
- Does NOT delete or modify already-summarised rows.
"#,
            "2026-05-24",
        ),
        mk(
            "commands/audit.md",
            r#"description: Produce OWASP-class security report for an app via modernlink-security-auditor subagent
argument-hint: [app=<slug>] [out=<path>] [severity=low|med|high] [depth=summary|full] [help=0|1]
allowed-tools: [Task, Read, Write]
"#,
            r#"# /modernlink:audit

Security audit of an modernlink-tracked app. Delegates to the
`modernlink-security-auditor` subagent.

## Arguments

| key      | default                | meaning                            |
|----------|--------------------------|--------------------------------------|
| app      | (none)                 | KB app slug (required)             |
| out      | docs/security/<app>.md | report path                        |
| severity | low                    | low / med / high - min to include  |
| depth    | summary                | summary / full                     |
| help     | 0                      | print this table and exit          |

Unknown-arg validation: abort with
`unknown arg: <key> (allowed: app, out, severity, depth, help)`.

Abort with `error: app required` if missing.

## Execute

1. Parse + validate.
2. Task dispatch with `subagent_type="modernlink:modernlink-security-auditor"`.
3. Stream subagent's structured findings.
4. Suggest next: `/modernlink:enrich app=<X> limit=25` if scan revealed under-enriched security-relevant modules.
"#,
            "2026-05-24",
        ),
        mk(
            "commands/grow.md",
            r#"description: Autonomous corpus-growth wake-up via modernlink-corpus-grower subagent
argument-hint: [max_minutes=N] [max_passes=N] [batch_size=N] [limit_per_pass=N] [quota_cap=N] [help=0|1]
allowed-tools: [Task]
"#,
            r#"# /modernlink:grow

Single autonomous wake-up of the corpus-growth loop. Picks the
highest-leverage app, runs bounded enrichment, reports growth.
Delegates to `modernlink-corpus-grower`.

## Arguments

| key            | default | meaning                                       |
|----------------|---------|--------------------------------------------------|
| max_minutes    | 30      | upper bound per wake-up                       |
| max_passes     | 4       | enrich passes per wake-up                     |
| batch_size     | 5       | modules per subagent                          |
| limit_per_pass | 25      | modules per pass                              |
| quota_cap      | 200     | abort wake-up at N total enrichments          |
| help           | 0       | print this table and exit                     |

## Execute

1. Parse + validate caps.
2. Task dispatch with `subagent_type="modernlink:modernlink-corpus-grower"`.
3. Stream the wake-up reintegration.

## Autonomy

For unattended growth, wrap in /loop or ScheduleWakeup:

```
/loop 6h /modernlink:grow max_passes=4 quota_cap=100
```

Solves the output-drought failure mode (phases ship, KB does not
grow) by letting the agent fire while the human is away.
"#,
            "2026-05-24",
        ),
        mk(
            "commands/heal.md",
            r#"description: Detect and repair modernlink-stack issues via modernlink-self-healer subagent
argument-hint: [fix=0|1] [scope=all|kb|plugin|enrich|mcp] [verbose=0|1] [help=0|1]
allowed-tools: [Task, Bash]
"#,
            r#"# /modernlink:heal

Operational repair. Runs diagnostics, classifies findings, applies
safe fixes when `fix=1`. Default is REPORT-ONLY. Delegates to the
`modernlink-self-healer` subagent.

## Arguments

| key     | default | meaning                                              |
|---------|---------|--------------------------------------------------------|
| fix     | 0       | 0 or 1 - actually apply fixes (otherwise report only)|
| scope   | all     | all / kb / plugin / enrich / mcp                     |
| verbose | 0       | include per-app breakdown                            |
| help    | 0       | print this table and exit                            |

Unknown-arg validation: abort with
`unknown arg: <key> (allowed: fix, scope, verbose, help)`.

## Execute

1. Parse + validate args.
2. Dispatch one Task call with `subagent_type="modernlink:modernlink-self-healer"` and parsed args.
3. Stream the subagent's report (FIXED / HINT / HUMAN / PASS rows).
4. If verdict=DEGRADED or FAILED and `fix=0`, suggest re-running with `fix=1`.

## Safety

The subagent NEVER restarts Postgres, the MCP server, or modifies
`~/.claude/settings.json` beyond what `modernlink plugin install` does.
NEEDS_HUMAN items always surface as actionable hints, never auto-applied.
"#,
            "2026-05-24",
        ),
        mk(
            "commands/resume.md",
            r#"description: Recover session continuity after /compact or CC restart via modernlink-resume subagent
argument-hint: [app=<slug>] [window=N] [help=0|1]
allowed-tools: [Task]
"#,
            r#"# /modernlink:resume

Pulls recent enrich activity, identifies work-in-progress, recommends
the next concrete command. Delegates to `modernlink-resume`.

## Arguments

| key    | default | meaning                                                |
|--------|---------|------------------------------------------------------------|
| app    | (any)   | filter to this app                                     |
| window | 5       | how many recent runs to inspect                        |
| help   | 0       | print this table and exit                              |

## Execute

1. Parse args.
2. Task dispatch with `subagent_type="modernlink:modernlink-resume"`.
3. Stream the recommendation block.
4. The user copies the recommended command and runs it. This agent does NOT auto-execute.

## When to use

- First command after CC restart.
- First command after /compact when you were mid-enrich.
- Anytime you forget where you left off.
"#,
            "2026-05-24",
        ),
        mk(
            "commands/triage.md",
            r#"description: Classify pending modules (SKIP/STATIC_OK/ENRICH) before LLM enrichment via modernlink-triage subagent
argument-hint: [app=<slug>] [limit=N] [apply=0|1] [min_body=N] [help=0|1]
allowed-tools: [Task, Read]
"#,
            r#"# /modernlink:triage

Quota-saving pre-enrich classifier. Delegates to `modernlink-triage`.

## Arguments

| key      | default | meaning                                              |
|----------|---------|---------------------------------------------------------|
| app      | (none)  | KB app slug (required)                               |
| limit    | 200     | how many pending rows to triage                      |
| apply    | 0       | 0 or 1 - persist STATIC_OK rows as static-only       |
| min_body | 256     | bytes - below = SKIP class                           |
| help     | 0       | print this table and exit                            |

Unknown-arg validation: abort with
`unknown arg: <key> (allowed: app, limit, apply, min_body, help)`.

Abort with `error: app required` if missing.

## Execute

1. Parse + validate.
2. Task dispatch with `subagent_type="modernlink:modernlink-triage"`.
3. Stream class breakdown.
4. Suggest next: `/modernlink:enrich app=<X>` to process ENRICH-class with savings.

## Best practice

Always run `/modernlink:triage apply=1` BEFORE the first
`/modernlink:enrich` on a freshly-dissected app. Saves 30-50 percent of
quota on typical Electron bundles where vendored React + icon-factory
chunks dominate the long tail.
"#,
            "2026-05-24",
        ),
    ]);
}
