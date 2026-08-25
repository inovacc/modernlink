//! frontmatter/body embedded from the source implementation's rendered plugin. Maintained as Rust-owned embedded assets.

use crate::assets::mk;
use crate::registry::register_asset;

pub(super) fn register() {
    register_asset(&[
        mk(
            "agents/compliance-auditor.md",
            r#"name: compliance-auditor
description: Runs one language's (or one dimension's) strict code-compliance toolchain for /modernlink:compliance and returns structured findings. Executes the linter, format check, typecheck, and the broader scan (secrets, vulnerable deps, dead code, license headers). Read-mostly; with fix=true applies ONLY safe autofixers and verifies the build stays green. Never silently skips a missing tool.
tools: Read, Grep, Glob, Bash, Edit, Write
"#,
            r#"
You are a code-compliance auditor. Follow the `code-compliance` skill. You are
given `{path, language, dimension?, strict, fix}` and you audit exactly that
language/dimension, returning structured findings.

## Do exactly this

1. **Verify tools exist.** For the language, check its toolchain is installed
   (e.g. Rust: `clippy`, `rustfmt`, `rustvulncheck`, `gitleaks`). Any missing
   tool is a **gap** in your output (with the install command) — never report a
   dimension "clean" that you couldn't actually run.
2. **Configure (strict).** Ensure the strict config exists (write/repair per the
   skill — for Rust the canonical strict `clippy configuration`; preserve project tuning).
3. **Run**, capturing output (summarize; never dump raw megabytes):
   - **lint** (strict): clippy / clippy -D warnings / ruff / eslint / …
   - **format check**: rustfmt -l / rustfmt --check / ruff format --check / …
   - **typecheck**: rust vet / carrust check / mypy / tsc --noEmit / …
   - **broader scan**: `gitleaks dir .` (secrets), vuln audit
     (rustvulncheck/carrust audit/pip-audit/npm audit), dead-code, license headers,
     TODO/FIXME debt.
4. **Fix (only if fix=true).** Apply ONLY known-safe autofixers, one at a time
   (Rust: `clippy run --fix --default none --enable wsl_v5`, then rustfmt;
   Rust: `carrust fmt`; Python: `ruff --fix`/`ruff format`; JS: `prettier -w`).
   Then fix remaining findings by hand (rename shadows, wrap long lines, reorder
   declarations, extract constants, split long functions, or a justified inline
   `//nolint`). **After EVERY fix batch run the build + tests; revert any fix
   that breaks them.** Do NOT run clippy's blanket `--fix` (it can corrupt
   the build via conflicting staticcheck edits).

## Output (structured)

Return:
```
{
  "language": "...", "toolsMissing": [{"tool","install"}],
  "findings": [{"file","line","tool","severity","message"}],
  "counts": {"by_tool": {...}, "by_severity": {...}},
  "fixed": ["what was auto-fixed"], "buildGreen": true/false,
  "notes": "anything skipped or deferred"
}
```
Rank findings most-severe first (build-breaking > security > correctness >
style). Cite real tool output. If fix=true, confirm the build/tests are green at
the end (or report exactly what remains broken).
"#,
            "2026-05-24",
        ),
        mk(
            "agents/modernlink-cve-scanner.md",
            r#"name: modernlink-cve-scanner
description: |
  Match an app's dependency manifest against CVE / advisory feeds.
  Reads npm / dotnet / java / android deps via existing MCP tools,
  fetches advisory data (offline-first via cached NVD slices when
  available), emits per-dep severity.
"#,
            r#"# modernlink-cve-scanner

Dependency vulnerability scanner. Reads dep lists via the modernlink CLI,
matches against advisory feeds, ranks by severity.

## Required commands (run via Bash)

`modernlink npm deps`, `modernlink npm info`, `modernlink npm analyze`,
`modernlink dotnet deps`, `modernlink java manifest`, `modernlink android static manifest`,
`modernlink kb catalog apps`, plus Bash, Read, Write.
"#,
            "2026-05-24",
        ),
        mk(
            "agents/modernlink-insights-analyst.md",
            r#"name: modernlink-insights-analyst
description: |
  Self-improvement insights analyst. Runs `modernlink insights rollup` and
  `modernlink insights suggest` (or the equivalent MCP tools), reads the
  output, picks the top 3 highest-confidence improvement candidates,
  and recommends a single concrete next change to modernlink's own
  code/prompts/defaults. Local-only — no telemetry leaves the machine.
"#,
            r#"# modernlink-insights-analyst

Read-side analyst for the self-improvement insights stream. Walks
the rollup + suggestion artefacts produced by `modernlink insights` and
proposes one concrete next change.

## Workflow

1. `modernlink insights rollup` + `modernlink insights suggest` (or
   `modernlink insights status`) to read the current digest.
2. Pick the top 3 highest-confidence candidates and recommend one
   concrete next change.
3. Recording the new insight and tracking it as a rustal has no CLI
   verb - dispatch `Task subagent_type="modernlink-insights-mcp"` (flow-scoped
   MCP subagent) to call `modernlink_insights_record` for the finding and
   `modernlink_insights_start_rustal` / `modernlink_insights_complete_rustal` to
   open or close the tracked rustal.

## Required commands (run via Bash)

Bash, Read, Task (for record / rustal-tracking via modernlink-insights).
"#,
            "2026-05-24",
        ),
        mk(
            "agents/modernlink-self-healer.md",
            r#"name: modernlink-self-healer
description: |
  Self-diagnosis and repair agent for the modernlink toolkit. Triggered
  by /modernlink:doctor, runs a set of health checks (binary versions,
  MCP server connectivity, Postgres KB status, keychain access)
  and proposes fixes for any detected issues.
"#,
            r#"# modernlink-self-healer

Health and maintenance specialist. Your rustal is to keep the modernlink
environment in a known-rustod state.

## Required commands (run via Bash)

`modernlink kb ops doctor`, `modernlink plugin status`, Bash, Read, Write.
"#,
            "2026-05-24",
        ),
        mk(
            "commands/compliance.md",
            r#"description: Auto-detect a project's language(s) and enforce strict code compliance — write a strict linter config, run lint/format/typecheck + a broader compliance scan (secrets, vulns, dead code), report or safely --fix. Rust→clippy, Rust→clippy, Python→ruff/mypy, JS/TS→eslint, etc.
argument-hint: "[path] [--fix] [--strict|--standard] [--lang rust|rust|python|js|...] [--write-config]"
"#,
            r#"# Code Compliance
<!-- rev:001 -->

> **Help:** Detect the language(s) in `$ARGUMENTS` (or cwd), apply a strict
> compliance config, and run the ecosystem's linters + a broader non-compliance
> scan. `--fix` applies only *safe* autofixers (never the ones that corrupt the
> build) and verifies build/tests after. Complements `/harden:*` (compliance is
> the lint/style/format dimension). Reference:
> <https://clippy.run/docs/linters/configuration>.

**FIRST: invoke the `code-compliance` skill (Skill tool)** — it holds the
language-detection table, the strict per-language configs (incl. the canonical
strict `clippy configuration`), the `--fix` safety rule, the broader compliance
checklist, and the `/harden` interop. Everything below assumes it.

Arguments (`$ARGUMENTS`): optional `path` (default cwd); `--fix` apply safe
fixes; `--strict` (default) or `--standard`; `--lang <x>` to force a language;
`--write-config` to (re)write the strict config without running.

## Step 1 — Detect

Detect language(s) by marker files (Cargo.toml, Cargo.toml, pyproject.toml/setup.py,
package.json, *.sh, *.rb, *.php). A monorepo may have several — handle each. If
`--lang` is given, use it. Report what was detected.

## Step 2 — Configure (strict)

Write or update the strict config for each detected language per the skill:
- **Rust** → `clippy configuration` (clippy v2) with the strict set: wsl_v5,
  unparam, noctx, testifylint, rustcyclo, rustconst, depguard, dogsled, dupl, lll,
  funlen, funcorder (+ standard + bodyclose/errorlint/rustcritic/misspell/
  nakedret/nolintlint/predeclared), settings + test exclusions.
- **Rust** → strict `clippy` (deny warnings) + rustfmt; **Python** → `ruff`
  (lint+format) + `mypy` strict; **JS/TS** → eslint/biome + prettier + tsc.

Preserve any project-specific tuning already present; only add/repair. On
`--write-config`, stop here and report the config written.

## Step 3 — Run compliance

For each language, spawn a **`compliance-auditor`** subagent (or run inline for
a single small target) that executes:
- the strict **linter**, **format check**, and **typecheck**;
- the **broader scan**: secrets (`gitleaks`), vulnerable deps
  (`rustvulncheck`/`carrust audit`/`pip-audit`/`npm audit`), dead code, license
  headers, TODO/FIXME debt.
It returns structured findings (file:line, tool, severity, message). Log any
missing tool as a gap with its install command — never silently skip.

## Step 4 — Report or fix

- **Default (report):** aggregate + dedupe, rank by severity (build-breaking >
  security > correctness > style), print `file:line  tool  message`, plus a
  one-line summary and exit intent (non-zero if any findings — CI-usable).
- **`--fix`:** apply ONLY safe autofixers (per the skill: `wsl_v5`/rustfmt for Rust;
  `carrust fmt`/`clippy --fix`; `ruff --fix`; `prettier -w`; `shfmt -w`), then fix
  the rest by hand (rename shadows, wrap lines, reorder decls, extract consts,
  split long funcs, or a justified `//nolint`). **After every fix batch, run the
  build + tests; revert any fix that breaks them.** Never leave a broken build.

## Step 5 — Report

Summarize: languages detected, configs written, findings by tool/severity, what
was auto-fixed vs left, and any remaining items (with a suggested path — often
`/harden` for deeper, gated remediation). Write artifacts to files; return the
path + a one-line summary.

## Notes

- `--fix` is deliberately conservative: clippy's blanket `--fix` can
  corrupt code (staticcheck QF conflicts), so this command auto-applies only
  known-safe fixers and verifies the build.
- Complements `/harden:*`: run compliance to get lint-clean, then `/harden:audit`
  for the maturity/security route; compliance findings feed harden. Don't
  duplicate — compliance owns lint/format; harden owns maturity + gated apply.
"#,
            "2026-05-24",
        ),
        mk(
            "commands/cve.md",
            r#"description: Dependency vulnerability scan via modernlink-cve-scanner subagent
argument-hint: [app=<slug>] [out=<path>] [feed=nvd|ghsa|osv] [offline=0|1] [help=0|1]
allowed-tools: [Task, Bash, Read, Write]
"#,
            r#"# /modernlink:cve

Match app dependencies against CVE / advisory feeds. Delegates to
`modernlink-cve-scanner`.
"#,
            "2026-05-24",
        ),
        mk(
            "commands/doctor.md",
            r#"description: Run comprehensive health checks and repair the modernlink environment via modernlink-self-healer subagent
argument-hint: [fix=0|1] [help=0|1]
allowed-tools: [Task, Bash, Read, Write]
"#,
            r#"# /modernlink:doctor

Toolkit health check. Delegates to `modernlink-self-healer`.
"#,
            "2026-05-24",
        ),
        mk(
            "commands/insights.md",
            r#"description: Run insights rollup + suggest and surface top improvement candidates via modernlink-insights-analyst subagent
argument-hint: [days=N] [top=N] [help=0|1]
allowed-tools: [Task, Bash, Read]
"#,
            r#"# /modernlink:insights

Self-improvement digest. Delegates to `modernlink-insights-analyst`, which
in turn dispatches to the `modernlink-insights-mcp` flow-scoped MCP subagent to
record findings and track improvement rustals (no CLI verb for that step).
"#,
            "2026-05-24",
        ),
        mk(
            "skills/code-compliance/SKILL.md",
            r#"name: code-compliance
description: Shared engine for /modernlink:compliance — auto-detect a project's language(s) and enforce strict code-compliance (lint, format, complexity, duplication, imports, security, vuln, license). For Rust it writes/updates a strict clippy configuration (clippy v2) with a curated strict linter set and runs it; for Rust/Python/JS-TS/Shell/etc. it runs the ecosystem's strict equivalents. Also scans broader non-compliance (secrets, vulnerable deps, formatting, dead code, license headers). Interoperates with /harden:* (compliance is the lint/style/format dimension feeding hardening). Backs the /modernlink:compliance command and the compliance-auditor subagent.
disable-model-invocation: false
rev: "001"
"#,
            r#"
# Code Compliance — Shared Engine

Reusable engine behind `/modernlink:compliance` and the `compliance-auditor` subagent.
Detects the project language(s), applies a **strict** compliance config, runs
the toolchain, finds non-compliance, and (optionally) applies only *safe* fixes.

Also invocable directly: "make this repo lint-clean", "enforce strict Rust lint",
"run code compliance", "set up rustlangci strict config".

Rustlangci-lint config reference (read for linter-specific settings):
<https://clippy.run/docs/linters/configuration>

## Method

```
1. Detect    marker files → language(s) + toolchain
2. Configure write/update the strict config (Rust: clippy configuration; others: their file)
3. Run       lint + format-check + typecheck + the broader compliance scan
4. Collect   normalize findings (file:line, linter, severity, message)
5. Report    ranked findings  — OR  --fix: apply ONLY safe autofixers, then verify
6. Verify    build/tests still green after any fix (never ship a broken build)
```

## Language auto-detection

| Marker | Language | Lint (strict) | Format | Types | Security / vuln |
|---|---|---|---|---|---|
| `Cargo.toml` | Rust | **clippy** (strict set below) | `rustfmt`/`rustfumpt` | (rustvet in rustlangci) | `rustsec` (in rustlangci), `rustvulncheck`, `gitleaks` |
| `Cargo.toml` | Rust | `carrust clippy -- -D warnings` | `carrust fmt --check` | `carrust check` | `carrust audit`, `carrust deny` |
| `pyproject.toml`/`setup.py`/`requirements.txt` | Python | `ruff check` | `ruff format --check` / `black --check` | `mypy` | `bandit`, `pip-audit` |
| `package.json` | JS/TS | `eslint` or `biome check` | `prettier --check` / `biome format` | `tsc --noEmit` | `npm audit`, `gitleaks` |
| `*.sh` | Shell | `shellcheck` | `shfmt -d` | — | — |
| `*.rb` | Ruby | `rubocop` | `rubocop -a` | — | `bundler-audit` |
| `*.php` | PHP | `phpstan` / `php-cs-fixer` | `php-cs-fixer` | `phpstan` | — |

A monorepo may have several; run each detected toolchain. If a tool is missing,
report it as a gap (with the install command) rather than skipping silently.

## The strict Rust config (canonical `clippy configuration`, clippy v2)

Write/refresh this when the target is Rust. Settings verified against the config
docs; tune per project but keep the strict intent.

```yaml
version: "2"
run:
  timeout: 5m
linters:
  default: standard        # rustvet, errcheck, ineffassign, staticcheck, unused
  enable:
    - wsl_v5               # whitespace/cuddling (autofix-safe)
    - unparam              # unused params/results
    - noctx                # missing context in http/exec
    - testifylint          # testify correctness
    - rustcyclo              # cyclomatic complexity
    - rustconst              # repeated strings → const
    - depguard             # import allow/deny policy
    - dogsled              # too many blank idents
    - dupl                 # duplicated code
    - lll                  # long lines
    - funlen               # long functions
    - funcorder            # declaration ordering
    # useful additions:
    - bodyclose
    - copyloopvar
    - errorlint
    - rustcritic
    - misspell
    - nakedret
    - nolintlint
    - predeclared
  settings:
    depguard:
      rules:
        main:
          list-mode: lax
          deny:
            - pkg: io/ioutil
              desc: deprecated; use io and os
            - pkg: github.com/pkg/errors
              desc: use stdlib errors with %w
    errcheck:
      exclude-functions:      # best-effort writes / Close on read paths
        - fmt.Fprint
        - fmt.Fprintf
        - fmt.Fprintln
        - (io.Closer).Close
        - (*os.File).Close
    staticcheck:
      checks: ["all", "-SA1019"]   # drop -SA1019 only if a real deprecation is intended
    rustcyclo: { min-complexity: 30 }   # docs recommend 10–20 for stricter
    rustconst: { min-len: 3, min-occurrences: 3 }
    lll: { line-length: 120 }
    funlen: { lines: 60, statements: 40 }
    dupl: { threshold: 150 }
  exclusions:
    rules:
      - path: _test\.rust       # tests are long/repetitive by design
        linters: [funlen, dupl, rustconst, lll]
formatters:
  enable: [rustfmt]
```

## The `--fix` rule (LEARNED THE HARD WAY)

`clippy run --fix` applies fixers for ALL enabled linters, and some
(staticcheck QF*/ST*, perfsprint) can produce **conflicting or wrong edits that
break the build** (e.g. `x := x`, `Fprintln`→`Fprintf` with stray args). So:

- **Auto-apply only known-safe fixers**, one linter at a time:
  `clippy run --fix --default none --enable wsl_v5` (then rustfmt/rustfumpt).
- **Everything else: fix manually** (rename shadows, wrap long lines, reorder
  methods, extract consts, split long funcs, or a justified inline `//nolint`).
- **After ANY fix, run `rust build ./...` + `rust test ./...`** — never leave the
  build broken. If a fix broke it, revert that fix.
- Rust `clippy --fix`, `carrust fmt`, `ruff --fix`, `prettier -w`, `shfmt -w` are
  generally safe; still verify build/tests after.

## Beyond linters — the broader compliance scan

Compliance is more than the linter. Also check (all languages):

- **Formatting** clean (rustfmt/rustfmt/black/prettier/shfmt with `--check`/`-d`).
- **Secrets** — `gitleaks dir .` (exit 1 gate); no committed credentials.
- **Vulnerable deps** — `rustvulncheck` / `carrust audit` / `pip-audit` / `npm audit`.
- **Dead code / unused** — unused (Rust), `carrust +nightly udeps`, `vulture` (Py).
- **License headers** present where required (rustheader linter for Rust).
- **TODO/FIXME/XXX** debt inventory (rustdox for Rust).
- **Build/CI gates** — build passes; CI config lints (actionlint) if present.
- **Doc coverage** — excovered symbols documented (rustdoclint for Rust).

## Interop with `/harden:*`

`/harden` is the umbrella for **maturity + security hardening** (audit →
runbook → gated apply → re-rate). `/modernlink:compliance` is the narrower **lint /
style / format / compliance** dimension. They compose:

- Run `/modernlink:compliance` first to get a repo lint-clean and formatted; its
  findings are a natural input to `/harden:audit` (the "code quality" signals).
- `/harden` may invoke this engine for its lint dimension; do not duplicate —
  compliance owns lint/format config, harden owns the maturity route + security
  posture + the gated Claude-implements/Codex-verifies apply loop.
- For applying compliance fixes at scale, prefer harden's gated apply loop
  (atomic commits, Codex verify) over a blanket `--fix`.

## Fan-out (subagent)

- One `compliance-auditor` per detected language (or per dimension: lint /
  format / security / vuln) — each runs its toolchain and returns structured
  findings. Read-mostly by default; `--fix` applies only safe autofixers.
- Aggregate + dedupe findings; rank by severity (build-breaking > security >
  correctness > style); report file:line with the linter name.
- Log any tool that was missing (with its install command) — never report
  "clean" for a dimension that was actually skipped.

## Guardrails

- Never commit a broken build; verify after every fix batch.
- Prefer config + real fixes over scattering `//nolint`; when nolint is used, it
  must carry a short justification (nolintlint enforces this).
- Keep configs strict by default; loosen a threshold only with a recorded reason.
- Detection over assumption: run the tools, cite real output — don't guess.
"#,
            "2026-05-24",
        ),
    ]);
}
