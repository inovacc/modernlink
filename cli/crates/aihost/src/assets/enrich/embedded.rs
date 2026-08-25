//! frontmatter/body embedded from the source implementation's rendered plugin. Maintained as Rust-owned embedded assets.

use crate::assets::mk;
use crate::registry::register_asset;

pub(super) fn register() {
    register_asset(&[
        mk(
            "commands/help.md",
            r#"description: Show all modernlink-plugin commands with usage hints
argument-hint: [verbose=1]
allowed-tools: []
"#,
            r#"
# /modernlink:help

Index of the `modernlink` plugin's command surface. Print the table below
verbatim, then exit. No MCP calls, no LLM, no side effects.

## Arguments

| key | default | meaning |
|-----|---------|---------|
| `verbose` | 0 | If 1: also print the "When to chain" section. |

## Output (always)

```
modernlink-plugin — JS module enrichment via Claude Code subagent fanout
======================================================================

ENRICH       /modernlink:enrich   [app=<a>] [limit=<N>] [batch_size=<N>]
                               [dry_run=1] [body_cap=<B>] [retry_failed=1]
                               [help=1]
             Reverse-engineer pending modules. Spawns Tasks in parallel.

PENDING      /modernlink:pending  [app=<a>] [limit=<N>]
             Read-only backlog. Pick next target before /modernlink:enrich.

RETRY        /modernlink:retry    [app=<a>] [limit=<N>] [status=1]
             Re-run failed enrichments (parse/schema). status=1 → counts only.

VERIFY       /modernlink:verify   [app=<a>] [limit=<N>] [role=<r>] [tag=<t>]
             Spot-check recent enrichments. Emits quality flags.

VENDORED     /modernlink:vendored [app=<a>] [min_count=<N>] [top=<N>]
             Find repeat-hash modules. Emit MODERNLINK_VENDORED_SHAS env line.

HELP         /modernlink:help     [verbose=1]
             This index.

DOCTOR       /modernlink:doctor   [verbose=1] [fix=1]
             Full health check (CC-side + server-side). Run on any breakage.
```

## Output (verbose=1, additional block)

```
WHEN TO CHAIN
─────────────────────────────────────────────────────────────
First time on a new app:
  1. /modernlink:pending app=<a>            ← see backlog size
  2. /modernlink:enrich  app=<a> limit=10   ← smoke (~2 min)
  3. /modernlink:verify  app=<a> limit=5    ← quality check
  4. /modernlink:vendored app=<a>           ← cut quota burn ~30%
  5. /modernlink:enrich  app=<a> limit=25   ← steady-state cadence

After a noisy run (failed>0):
  1. /modernlink:retry status=1 app=<a>     ← see failure breakdown
  2. /modernlink:retry app=<a> limit=25     ← drain failed bucket

Multi-thousand backlog:
  /loop 5m /modernlink:enrich app=<a> limit=25 batch_size=1

Before declaring an app done:
  /modernlink:pending app=<a>               ← expect total_pending=0
  /modernlink:retry status=1 app=<a>        ← expect failed_*=0
  /modernlink:verify app=<a> limit=20       ← expect flags:OK

QUOTA GUARDRAIL
─────────────────────────────────────────────────────────────
Subscription window is 5 hours. Each /modernlink:enrich limit=25 burns
~30 seconds of warm cache. Safe cadence:
  - foreground: every 2-5 min for limit=25
  - /loop: 5m intervals for limit=25 (max ~300 mod/hour)

Watch the `rate=` field in the summary. Rate < 10 mod/min ⇒ cache cold
or sequential dispatch bug — pause and investigate before retrying.
```

## Out of scope

- No MCP calls.
- No state mutation.
- Not a substitute for reading individual command files in
  `modernlink-plugin/commands/`.
"#,
            "2026-05-24",
        ),
        mk(
            "commands/vendored.md",
            r#"description: Detect repeat-hash modules (likely vendored libraries) and emit MODERNLINK_VENDORED_SHAS env line
argument-hint: [app=<name>] [min_count=<N>] [top=<N>]
allowed-tools: [Bash]
"#,
            r#"
# /modernlink:vendored

Find duplicate-hash modules — almost always vendored libraries (React,
MobX, Apollo, jwt-decode, Dexie, …) that are not worth burning
enrichment quota on. Emits a paste-ready `MODERNLINK_VENDORED_SHAS=` line
the user can set on the MCP server env to filter them out of future
`modernlink kb enrich pending` results.

Saves ~30% quota on Teams-class bundles where library duplicates
dominate the pending-module list.

## Arguments

Arguments provided: $ARGUMENTS

| key | default | range | meaning |
|-----|---------|-------|---------|
| `app` | none (= all apps) | string | scope detection to one app |
| `min_count` | 3 | 2-100 | a hash must appear in ≥ this many modules to count as vendored |
| `top` | 50 | 1-200 | emit the top-N repeat hashes in the env line |

Unknown-arg validation: abort with
`unknown arg: <key> (allowed: app, min_count, top)`.

## How to execute

There is no dedicated CLI verb for vendored-hash detection yet — run
the aggregate query directly via `modernlink kb catalog query` (Bash):

1. Run `modernlink kb catalog query` with a SQL statement equivalent to:

```sql
SELECT body_sha256, COUNT(*) AS occurrences, MIN(name) AS sample_name
FROM modules
WHERE (app = '<app>' OR '<app>' = '')
GROUP BY body_sha256
HAVING COUNT(*) >= <min_count>
ORDER BY occurrences DESC
LIMIT <top>;
```

   From the result rows, compute `total_hashes` (row count) and
   `total_vendored_modules` (sum of `occurrences`), and build
   `env_line = "export MODERNLINK_VENDORED_SHAS=<comma-joined sha256 list>"`.

2. (optional) Run `modernlink kb catalog stats` to fetch
   `total_pending` for the impact estimate.

### Output

Render two blocks:

```
REPEAT HASHES (app=<app|all>, min_count=<N>, top=<T>)
 occurrences  sha256                            sample_name
       142    ab12cd34ef56...                   teams_module_124664
        87    98765432abcd...                   teams_module_124712
       ...
total_vendored_hashes=<H> total_vendored_modules=<M>
```

Then print the paste-ready env line from `env_line` verbatim:

```
# Add to your modernlink mcp server env (~/.config/modernlink/env or shell rc):
<env_line value>

# Or merge with existing list (Windows PowerShell):
$env:MODERNLINK_VENDORED_SHAS = (($env:MODERNLINK_VENDORED_SHAS -split ',') + @('<sha1>','<sha2>')) -join ','
```

Estimate impact:

```
estimated_quota_saved=<total_vendored_modules - total_hashes> modules (~<P>% of pending backlog)
```

Where `P = (total_vendored_modules - total_hashes) / total_pending * 100`.
Pull `total_pending` from the optional `kb_stats` call (or `/modernlink:pending`).

## When to use

- Once per project after the first enrichment pass identifies
  repeat-hash modules.
  libraries).
- When `/modernlink:pending` shows backlog growing faster than enrichment
  can drain it — vendored skip is the cheapest mitigation.

## Out of scope

- Does NOT write the env var for the user — they must restart the MCP
  server with the new env so the change picks up.
- Does NOT enrich, retry, or verify anything.
- Does NOT identify which library each hash IS — that's a separate
  exercise (`modernlink npm_probe`, `nodeaddon_strings`, …).
"#,
            "2026-05-24",
        ),
        mk(
            "commands/verify.md",
            r#"description: Spot-check recently enriched modules for quality (read-only)
argument-hint: [app=<name>] [limit=<N>] [query=<text>]
allowed-tools: [Bash]
"#,
            r#"
# /modernlink:verify

Read-only quality probe. Pulls N recently-enriched modules from the KB
and renders their summaries for human review. No LLM calls, no writes.

Use after a `/modernlink:enrich` run to confirm:
- Summaries are coherent (not "opaque module" for everything).
- `role` distribution looks sane (not 100% `other` or 100% `util`).
- Inputs/outputs/deps populated where expected.

## Arguments

Arguments provided: $ARGUMENTS

| key | default | range | meaning |
|-----|---------|-------|---------|
| `app` | none (= all apps) | string | scope to one app |
| `limit` | 5 | 1-50 | sample size |
| `query` | none (= recency only) | string | trigram match against summary/long_summary/tags — pass a role name (`receive`), tag (`trouter`), or substring |

Unknown-arg validation: abort with
`unknown arg: <key> (allowed: app, limit, query)`.

**Filter note.** The backing `modernlink kb catalog search` command only exposes
trigram match via `query` plus `app/component/fact_type/lang/since/limit/cursor`
— there is no first-class `role=` or `tag=` filter. We map all
substring needs onto `query`. If you need exact role enum filtering,
fall back to `modernlink kb catalog dump <id>` after picking ids from the sample.

## How to execute

1. Run `modernlink kb catalog search` with `{query, app, limit}` via Bash.
   If `query` is empty, send a minimal query like `"."` so the trigram
   index returns recent rows.
2. For each module returned, render:

```
─────────────────────────────────────────────────────────────
[<id>] <app> / <name>                              role=<role>
sha256:  <sha256[:8]>
tags:    <tag1, tag2, ...>
summary: <one-line summary>

<long_summary>

inputs:       <N>  ──  <name>:<type>, ...
outputs:      <N>  ──  <name>:<type>, ...
side_effects: <N>  ──  <bullet 1>, <bullet 2>, ...
deps:         <N>  ──  <dep1>, <dep2>, ...
```

3. At the end, print a 1-line heuristic verdict:

```
sample=<N> opaque_rate=<X%> empty_io_rate=<Y%> unique_roles=<R>
```

Where:
- `opaque_rate` = pct of summaries starting with `"opaque module"`.
- `empty_io_rate` = pct of modules with BOTH inputs and outputs empty.
- `unique_roles` = count of distinct `role` values in the sample.

### Quality flags

Append a `flags:` line with any of:

| flag | trigger | hint |
|------|---------|------|
| `OPAQUE_HIGH` | opaque_rate > 30% | bodies probably under-truncated; re-enrich with `body_cap=4096` |
| `ROLE_FLAT` | unique_roles == 1 and sample > 5 | subagent collapsing all modules into one role; review schema check |
| `EMPTY_IO_HIGH` | empty_io_rate > 50% | symbols extraction may be failing upstream |
| `OK` | none of the above | sample looks healthy |

## When to use

- Right after `/modernlink:enrich` finishes (default sample=5 is fast).
- Before promoting a snapshot to `kb_export`.
- Periodically inside a long catch-up loop to detect quality drift.

## Out of scope

- No LLM. This is a pure read.
- Does NOT re-enrich. Use `/modernlink:retry` or `/modernlink:enrich` for that.
- Does NOT score individual modules — flags are sample-aggregate only.
"#,
            "2026-05-24",
        ),
        mk(
            "skills/enrich/SKILL.md",
            r#"name: enrich
description: Reverse-engineer pending JS modules from the modernlink Postgres knowledge base. Uses Task-spawned subagents (one per module batch) so every LLM call runs inside this Claude Code session — taps the active subscription via the warm-session prompt cache instead of fanning out external `claude --print` children. Invoke when the user says "enrich N modules of <app>", "/modernlink:enrich", "run kb enrich", or any variant of "summarise pending modules / fill in missing summaries / catch up enrichment".
"#,
            r#"
# modernlink-enrich — Task-fanout enrichment orchestrator

You are running the `/modernlink:enrich` workflow. The modernlink MCP server provides
the I/O (read pending modules, write enriched results) but NEVER calls the LLM
itself. The LLM calls happen here, inside this conversation, by spawning
`modernlink-enricher` subagents via the Task tool — the same pattern
Understand-Anything uses for its `file-analyzer` subagents.

## Why this design

modernlink originally used `kbllm.Call → exec.Command("claude", "--print")` to do
per-module enrichment. Each child was a cold Claude session with no prompt
cache, and a Phase A `conc=8` recipe drained a 5-hour subscription window in
20 minutes. The MCP sampling pivot (commit 65acd4de) tried to route through
`sampling/createMessage` so the parent host would make the LLM call instead —
but Claude Code's MCP client doesn't currently implement client-side sampling,
so that path returns "Method not found".

This skill closes the gap by inverting the integration: Claude Code (the
runtime) does the LLM work via Task; modernlink (the MCP server) only handles
deterministic I/O. The result is the Understand-Anything quota model — one
warm session, one prompt cache, one subscription.

## Arguments (the user may supply any combination)

| Arg | Default | Range | Meaning |
|-----|---------|-------|---------|
| `app` | none (= all apps) | string | e.g. `teams`, `whatsapp`, `slack` |
| `limit` | 10 | 1-100 | max modules this run |
| `batch_size` | 5 | 1-10 | modules per subagent — 1 keeps validation tight; 5-10 amortises subagent boot cost on cheap modules. Measured 2026-05-23: `batch_size=1` + 1-at-a-time gave 5.1 mod/min; CLAUDE.md recipe target is 44.4 mod/min. Default 5 with concurrent dispatch (see step 2) closes the gap. |
| `dry_run` | 0 | 0 or 1 | If 1: fetch pending modules, print table (id, app, name, sha256[:8]) and exit. No Task spawn, no DB writes. |
| `body_cap` | 2048 | 256-8192 | Truncate `body_excerpt` to this many bytes before sending to subagent. |
| `retry_failed` | 0 | 0 or 1 | After main loop, if any modules failed, re-invoke `/modernlink:enrich` for the same `app` (idempotent — already-summarised modules are skipped) and roll the results into the final reintegration. |

Sanity-cap `limit` at 100; if user passes more, clamp and print
`notice: limit clamped from <N> to 100`. Anything larger should rust through
the catch-up loop (see bottom of this file).

**Argument validation.** Before any work, validate every parsed key
against the table above. On unknown key, abort with:

```
unknown arg: <key> (allowed: app, limit, batch_size, dry_run, body_cap, retry_failed)
```

This prevents silent no-ops from typos like `aplication=teams`.

## Quota guardrail (soft, session-scoped)

Before fetching pending modules, check this session's prior
`/modernlink:enrich` invocations (visible in the conversation transcript).
If the session has already dispatched more than **400 modules** of
combined `limit` in the last hour, print a warning and ask the user to
confirm:

```
quota_guard: this session has enriched ~<N> modules in the last hour.
Subscription window is 5 hours; typical safe ceiling is 1500 modules.
Continue? Reply "yes" to proceed, anything else aborts.
```

If the user did not just answer the guard (i.e. this is the first
invocation of the new run), block on their reply. Skip the guard inside
`/loop`-driven re-invocations only if the parent `/loop` was started
within the same session — the loop is the user's explicit consent.

This is a heuristic, not a hard limit. Real rate limits are enforced by
the Anthropic subscription itself; this just prevents the obvious
foot-gun of `/modernlink:enrich limit=100 batch_size=1` typed 10x in a
row.

## Workflow

## Model-escalation protocol (KBC-ENRICH-MODEL-ESCALATION)

For each module the subagent attempts to summarise:

1. **First 3 attempts** use the default model from the `model` arg
   (sonnet 4.6 by default; haiku 4.5 opt-in for speed). Track each
   attempt's status — success / parse-failure / timeout / safety-filter
   — yourself in scratch state; there is no dedicated CLI per-attempt
   log verb yet.

2. **After 3 sonnet failures on the same module**, the orchestrator
   retries ONCE with `model: opus`. The opus call uses the same prompt
   shape (cacheable prefix preserved); only the model alias changes.

3. **On opus success**, run `modernlink kb enrich write-enrichment` with the
   normal parsed_json payload and `--model opus`. `write-enrichment`
   has no `escalated_to` flag — it only supports `--id --app --sha256
   --model --file|--json --database`. Record the escalation afterward via
   `modernlink kb catalog query` (there is no dedicated CLI verb for this
   yet):

   ```sql
   UPDATE modules SET escalated_to = 'opus' WHERE id = <module_id>;
   ```

4. **On opus failure** (parse error, timeout, or safety filter), run
   `modernlink kb enrich write-enrichment` with a placeholder parsed_json
   (`{"summary": "", "role": "unparseable", "long_summary": "<one-line cause>", "tags": "needs-human-review"}`)
   and `--model opus`. Same flag gap as step 3 — `write-enrichment` has
   no `needs_human_verification` flag. Flag the module for human review
   afterward via `modernlink kb catalog query`:

   ```sql
   UPDATE modules SET needs_human_verification = true WHERE id = <module_id>;
   ```

   Subsequent `modernlink kb enrich pending` calls automatically exclude this
   module (the WHERE clause already filters `needs_human_verification = false`).

5. The operator clears the flag via
   `modernlink kb catalog query` (UPDATE `modules SET needs_human_verification = false`
   for the module id — there is no dedicated CLI mark-resolved verb yet) after
   fixing the underlying issue (typically prompt-template adversarial
   input, or a module body that genuinely needs human curation).

## Quota cost

Opus is ~5x more expensive per token than sonnet. The 3x-then-1x escalation
pattern caps the worst-case spend at ~2x normal (3 sonnet + 1 opus = ~8
sonnet-equivalents instead of 3). Per-app rate-limit the opus path if
the failure-rate climbs above 10% — that signals a systemic issue, not
sparse per-module failures.

## Why the orchestrator (skill), not Rust

The Rust enrich pipeline rejects `model: opus` explicitly (cost guard).
Adding opus to the Rust allowlist is reachable, but the v2.13 plugin
pivot moved every LLM call out of Rust and into the Claude Code session
— putting the retry logic in the skill keeps the data plane (Rust)
deterministic and the control plane (LLM orchestration) in the
Markdown layer where it belongs.

Use the **TaskCreate / TaskList** tools to track these steps as you rust:

### 0. Open the run (cost accounting)

Per-batch token-cost attribution has no dedicated CLI verb yet (no
`enrich_run` open/record call is exposed outside the MCP layer). Skip
this step in CLI-only mode — cost capture is best-effort and never
blocks enriching; track cumulative token spend informally from the
Task subagent results if useful, but do not attempt to persist it.

### 1. Fetch pending modules

Run `modernlink kb enrich pending` via Bash with the user's `app`,
`limit`, and `body_cap` (default 2048 — the command truncates
before returning). The output is a JSON object:

```json
{
  "modules": [
    {
      "id": 12345,
      "app": "teams",
      "name": "teams_module_124664",
      "sha256": "ab12...",
      "body_excerpt": "function send(...) { ... }",
      "symbols_json": "{\"methods\":[\"send\",\"close\"]}"
    },
    ...
  ],
  "count": 10,
  "app": "teams"
}
```

Iterate `response.modules` — NOT the top-level value. Treating the
response as a bare array breaks under MCP-spec-strict hosts that require
structured content to be a record-typed JSON object.

If `count == 0` (or `modules` is empty), tell the user "no pending
modules for filter X" and stop.

**Dry-run branch.** If `dry_run=1`, print one row per module as a table:

```
PENDING (dry-run, no Tasks spawned)
 id      app    name                       sha256
 12345   teams  teams_module_124664        ab12cd34
 12346   teams  teams_module_124665        ef56gh78
 ...
total=<N> app=<app|all>
```

Then exit. No Task spawn, no DB writes. Use this to sanity-check the
filter before burning quota.

### Cache-friendly subagent prompt format

The `modernlink-enricher` subagent's frontmatter description already contains
the canonical JSON schema + hard rules. Do NOT repeat those in the Task
prompt — duplicating them defeats the prompt cache. Instead, send the
minimal per-module envelope:

```
id: <int>
app: <string>
name: <string>
symbols_json: <raw symbols JSON>
body_excerpt: <up to body_cap bytes>
```

That's it. The subagent reads its own contract from frontmatter; your
prompt only needs the variable inputs. This keeps the cacheable prefix
~95% identical across the whole batch.

### 2. Dispatch one Task per batch

Group the rows into chunks of `batch_size`. For each chunk, invoke the **Task**
tool with `subagent_type="modernlink:modernlink-enricher"`. The prompt MUST contain:

- The batch's module rows verbatim (id, app, name, body_excerpt, symbols_json).
- A reminder that the subagent must return one JSON object per module, keyed
  by the input id, with `summary`, `long_summary`, `role`, `inputs`, `outputs`,
  `side_effects`, `deps`, `tags`. See `agents/modernlink-enricher.md` for the
  schema contract.

**Dispatch ALL batches in parallel.** Multiple Task tool invocations in a
single assistant message run concurrently — this is the only way to hit the
44.4 mod/min recipe target. Emit `min(8, number_of_batches)` Task calls in
the same message; do not wait for one to return before issuing the next.
Sequential dispatch (one Task per message) caps throughput at ~5 mod/min and
defeats the prompt-cache win — verified empirically 2026-05-23
(see `docs/quality/mcp-interaction-quality-2026-05-23.md`).

Concurrency cap = 8 (matches the original `conc=8` Phase A recipe). Rusting
higher risks subscription rate-limit; lower wastes the warm session.

**Progress signal.** After dispatching, record a monotonic start time.
As each Task settles, print one line:

```
batch <i>/<total> done (modules=<k>, parse_ok=<true|false>)
```

Do NOT print the JSON body — counts only. This gives the user visible
progress during long runs without flooding context.

**body_cap enforcement.** Before building each per-module envelope,
truncate `body_excerpt` to `body_cap` bytes (default 2048). Append
`... <truncated>` if cut. Larger caps waste cache prefix; smaller caps
starve the subagent of context.

### 3. Validate + write back

For each subagent result:

1. Parse the JSON. If it's not valid JSON, mark the modules in that batch as
   `failed` and skip — don't try to repair in this loop. There is no
   dedicated CLI retry verb; re-running `/modernlink:enrich` for the same
   app later picks these back up automatically (already-summarised
   modules are skipped by `modernlink kb enrich pending`).
2. **Schema check** each element before writing. Required keys:
   `id` (int), `summary` (non-empty string), `role` (must be one of
   `send|receive|auth|pair|storage|sync|protocol|crypto|media|presence|call|ui|telemetry|util|other|vendored-library`),
   `inputs` (array), `outputs` (array), `side_effects` (array),
   `deps` (array), `tags` (array). Missing/wrong-typed key → mark that
   element `failed` with `error_class="schema"`. Do not write garbage.
3. For each module the subagent returned, run
   `modernlink kb enrich write-enrichment` via Bash with `{module_id, app, sha256,
   raw_response, parsed_result, model_used: "claude-code-subagent"}`. The CLI
   command persists into `modules.summary` / `modules.tags` / `module_enrichment`.

### After each batch — record token cost

Per-batch token-cost accounting has no dedicated CLI verb yet. Skip
this in CLI-only mode — cost capture is best-effort and must never
block enriching. If a running total is useful, sum each Task
subagent result's `subagent_tokens` figure locally and surface it in the
final report only.

**Optional retry pass.** If `retry_failed=1` and the failed count > 0,
re-run this same workflow (fetch pending -> dispatch -> validate) once
more after the main loop — there is no dedicated CLI retry verb, but
`modernlink kb enrich pending` naturally re-surfaces any module that
still has `summary IS NULL`. Add its results into the final counts
under `retry_completed=<N>`.

### 4. Report

When all Tasks have settled, emit a single summary block:

```
enriched=<N> failed=<M> retry_completed=<R> elapsed=<seconds>s rate=<X.X> mod/min app=<app> limit=<limit> batch_size=<bs>
```

`rate` = `enriched / (elapsed / 60)`. Round to one decimal. The CLAUDE.md
recipe target is 44.4 mod/min; anything under 10 mod/min ⇒ cache cold or
sequential dispatch bug (verify all Task calls were in ONE message).

Omit `retry_completed` when `retry_failed=0`. Do NOT print per-module
diffs.

### 5. Report cost

Per-run cost totals have no dedicated CLI verb yet. Skip this block —
report only the token counts you tallied informally in step "after
each batch" above, if any.

## Failure handling

- `modernlink kb enrich pending` errors or the `modernlink` binary isn't
  found → the CLI isn't on PATH or the KB isn't initialized. Tell the
  user to run `modernlink kb ops doctor` and retry.
- Subagent returns garbage JSON → record failure with `error_class="parse"`,
  surface in the summary, suggest re-running `/modernlink:enrich` for the
  same app (idempotent).
- The user cancels mid-run → respect it. Already-persisted modules stay
  persisted; in-flight Tasks abort when their context cancels.

## Out of scope (do NOT do these here)

- Do NOT rely on MCP `sampling/createMessage` for enrichment — Claude
  Code's MCP client doesn't implement client-side sampling, so that path
  errors with "Method not found" in this host. Use the Task-fanout flow
  above (`modernlink kb enrich pending` / `modernlink kb enrich write-enrichment`,
  re-invoked as needed) instead.
- Do NOT shell out to `claude --print` via Bash. That's the legacy
  subprocess quota-burning path, hard-gated by `task d09:check`.
- Do NOT touch `pkg/knowledge/kbenrich` or `kbllm.Call` from here. The skill
  is pure orchestration; all DB writes rust through the modernlink CLI.

## Quick smoke test

```
/modernlink:enrich app=teams limit=3 batch_size=1
```

Expected outcome: 3 Task calls spawned in parallel (one assistant
message), each returns a JSON array element for one module, 3
`modernlink kb enrich write-enrichment` writes, three `batch i/3 done` progress
lines, then final summary:

```
enriched=3 failed=0 elapsed=<S>s rate=<X.X> mod/min app=teams limit=3 batch_size=1
```

## Catch-up loop

For multi-thousand-module backlogs, wrap in `/loop` so the skill keeps
re-invoking until pending hits zero (or the user interrupts):

```
/loop 5m /modernlink:enrich app=teams limit=25 batch_size=1
```

The skill is idempotent — already-summarised modules are filtered out by
`modernlink kb enrich pending`, so re-invocations only ever touch new rows.
Cap to ~25-50 per loop iteration to keep each turn's output manageable.

## Vendored-library skip

`MODERNLINK_VENDORED_SHAS=hash1,hash2,...` env var on the modernlink mcp server
filters known third-party library hashes (React, MobX, Apollo, jwt-decode,
Dexie, etc.) out of `modernlink kb enrich pending` results. Saves ~30% of
quota on Teams-class bundles where library duplicates dominate the
pending-module list. Populate the list once per project after a first
pass identifies repeat-hash modules:

```sql
SELECT body_sha256, COUNT(*) FROM modules
WHERE summary IS NOT NULL
GROUP BY body_sha256 HAVING COUNT(*) > 3
ORDER BY 2 DESC LIMIT 50;
```
"#,
            "2026-05-24",
        ),
    ]);
}
