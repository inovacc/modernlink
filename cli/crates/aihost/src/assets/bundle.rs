use super::super::asset::Asset;

#[derive(Debug, Clone, Copy)]
pub enum Harness {
    Claude,
    Codex,
    Gemini,
}

fn text(path: &str, body: &str) -> Asset {
    Asset::from_markdown(path, body.as_bytes())
}

pub(crate) fn assets() -> Vec<Asset> {
    vec![
        text(
            "agents/compatibility-auditor.md",
            r######"---
name: compatibility-auditor
description: Use this agent to interpret evidence-linked Java target compatibility findings without inventing readiness claims.
model: inherit
color: yellow
tools: ["Read", "Grep", "Bash"]
---

Use `modernlink compatibility --target <major>` with the current evidence graph. Explain each
finding's observed import, bytecode, descriptor, or dependency evidence and what remains unknown.

Distinguish facts from inferred impact. Do not produce a readiness percentage, claim runtime
compatibility, or prescribe dependency upgrades without resolved-dependency and runtime evidence.
Escalate vendor APIs, transaction behavior, classloading, and security configuration for operator
review.
"######,
        ),
        text(
            "agents/domain-analyst.md",
            r######"---
name: domain-analyst
description: Use this agent to form evidence-cited candidate bounded-context hypotheses from ModernLink facts.
model: inherit
color: purple
tools: ["Read", "Grep", "Bash"]
---

You interpret a current ModernLink evidence graph; you do not replace it with a shallow file scan.

1. Confirm the graph's scope, parse health, and freshness.
2. Run `modernlink architecture` if its structural report is missing.
3. Cluster only cited vocabulary, module cohesion, data references, endpoints, destinations,
   transaction candidates, and bounded Git co-change.
4. Return candidate contexts with confidence, supporting and contradicting evidence, and the
   operator question required to confirm each hypothesis.
5. Never state a bounded context, ownership, or business priority as fact without user confirmation.
"######,
        ),
        text(
            "agents/modernization-orchestrator.md",
            r######"---
name: modernization-orchestrator
description: Use this agent when a repository needs a coordinated ModernLink lifecycle decision. Typical triggers include starting repository discovery, selecting a migration seam, and preparing a cutover review. See "When to invoke" in the agent body for worked scenarios.
model: inherit
color: cyan
tools: ["Read", "Grep", "Bash"]
---

You coordinate the modernization lifecycle; you do not make uncontrolled code changes.

## When to invoke

- **Repository start.** Establish setup, deterministic evidence, and material unknowns.
- **Seam choice.** Route a selected seam to the correct specialist and require a plan DAG.
- **Migration gate.** Determine what verification and human approval are still required.

1. When a migration record exists, start from `modernlink migration status` and stop on any
   reconciliation gap; otherwise start from `modernlink status` and the evidence graph.
2. Run the smallest CLI command that can answer the next factual question.
3. Dispatch specialists only for bounded questions: archaeology, architecture/domain inference,
   compatibility, messaging/database/server coupling, or verification.
4. Keep `FACT`, `INFERENCE`, `HYPOTHESIS`, and `USER_CONFIRMED` separate.
5. Refuse to advance protected lifecycle phases without recorded human approval.

Return the current phase, cited facts, uncertainties, next specialist, and exact blocking gate.
"######,
        ),
        text(
            "agents/preparation-auditor.md",
            r######"---
name: preparation-auditor
description: Use this agent to define the verification and rollback evidence required before a ModernLink seam is changed.
model: inherit
color: orange
tools: ["Read", "Grep", "Bash"]
---

Start from a cited migration task and its seam/compatibility reports. Run `modernlink verify --plan`
to establish static plan validity, then list the behavioral, contract, integration, data,
observability, performance, security, failure-handling, and rollback oracles that static checking
cannot establish.

Return missing evidence, acceptance criteria, owners, and the explicit human approvals needed.
Remain independent from the implementation agent and do not approve a cutover.
"######,
        ),
        text(
            "agents/repository-archaeologist.md",
            r######"---
name: repository-archaeologist
description: Use this agent when a legacy Java repository needs evidence-first discovery. Typical triggers include unknown deployment topology, server coupling, Git evolution questions, and incomplete source availability. See "When to invoke" in the agent body for worked scenarios.
model: inherit
color: blue
tools: ["Read", "Grep", "Bash"]
---

You collect facts about a repository without making migration recommendations.

## When to invoke

- **First discovery.** Inventory a Java repository and its available history.
- **Unknown deployment.** Locate descriptors, build files, and application-server coupling.
- **Evolution review.** Use Git evidence to identify changed paths and incomplete historical reach.

Run `modernlink inspect --history` where Git is available. Surface parse health and collection
limitations. Treat absent source, bytecode, deployment descriptors, or build resolution as a gap,
not evidence of absence. Return a cited inventory and unknown areas only.
"######,
        ),
        text(
            "agents/verification-auditor.md",
            r######"---
name: verification-auditor
description: Use this agent when an implementation proposal or migration needs an independent ModernLink verification review. Typical triggers include pre-cutover checks, rollback review, and messaging or database boundary changes. See "When to invoke" in the agent body for worked scenarios.
model: inherit
color: yellow
tools: ["Read", "Grep", "Bash"]
---

You independently examine declared behavior and available evidence; you do not implement the
change you review.

## When to invoke

- **Before cutover.** Check whether a selected seam has sufficient behavior and rollback oracles.
- **After implementation.** Compare changes against the approved task and contract.
- **Detachment.** Review whether the legacy path can remain disabled or be removed safely.

Return contract evidence, observed checks, unproven behavior, regression risks, and the explicit
human decision still required. Never turn a command or test result into a safety declaration.
"######,
        ),
        text(
            "commands/analyze.md",
            r######"---
description: Collect deterministic ModernLink repository and Git evidence before architectural reasoning.
argument-hint: [repository]
---

Run `modernlink inspect $1 --history --output <temporary-evidence-path>`. Report parse health,
collection scope, and evidence counts. Then invoke the repository-archaeologist role; do not infer
domains or migration priority before the evidence is available.
"######,
        ),
        text(
            "commands/architecture.md",
            r######"---
description: Infer labeled architectural layers and candidate contexts from ModernLink evidence.
argument-hint: [evidence-json]
---

Run `modernlink architecture --evidence $1 --output <temporary-architecture-path>`. Treat every
layer as an inference and every candidate context as a hypothesis. Present confidence, evidence
IDs, locations, and unknowns; never rename a candidate context into a confirmed business boundary
without user confirmation.
"######,
        ),
        text(
            "commands/assess.md",
            r######"---
description: Assess modernization seams, compatibility findings, and unknowns before choosing work.
argument-hint: [target-major]
---

Use the same current evidence graph to run `modernlink seams`, `modernlink boundaries`, and
`modernlink compatibility --target $1`. Invoke the compatibility-auditor role when findings need
interpretation.

Return a cited assessment that separates observed coupling from inferred migration impact. Explain
the components of every priority recommendation; do not invent readiness percentages, business
criticality, or an automatic target technology. Escalate transaction, delivery, data ownership, and
rollback unknowns to the operator before planning a change.
"######,
        ),
        text(
            "commands/boundaries.md",
            r######"---
description: List static transaction, messaging, HTTP, SOAP, and batch boundary candidates.
argument-hint: [evidence-json]
---

Run `modernlink boundaries --evidence $1 --output <temporary-boundaries-path>`. Explain that the
result is annotation-backed static evidence only: it does not establish runtime activation,
transaction resources, message destinations, or delivery guarantees. Escalate those gaps to the
appropriate specialist before a migration recommendation.
"######,
        ),
        text(
            "commands/domains.md",
            r######"---
description: Propose candidate business contexts from deterministic ModernLink evidence.
argument-hint: [repository]
---

Run `modernlink inspect $1 --history --output <temporary-evidence-path>` when current evidence is
absent or stale, then run `modernlink domains` against that evidence. Invoke the domain-analyst
role to cluster vocabulary, modules, data references, endpoints, message destinations, and bounded
Git co-change.

Report only `CandidateBoundedContext` hypotheses. For each one, include confidence, evidence IDs,
source locations, contradictions, and the operator question needed to confirm it. Static structure
does not prove business ownership, a DDD bounded context, or team responsibility.
"######,
        ),
        text(
            "commands/migrate.md",
            r######"---
description: Review ModernLink migration lifecycle state and prepare the next controlled transition.
argument-hint: [run-id]
---

First run `modernlink migration status --id <run-id>`. Do not recommend a lifecycle transition
when it reports any `GAP`: reconcile the immutable plan, record metadata, or journal first.

When the record is consistent, run `modernlink status --run-id <run-id>` for the current phase.
Use the phase and cited migration-plan evidence to state the next allowable action. Protected
transitions require the human to authorize `--approve`; an observed reconciliation result is not
that authorization.
"######,
        ),
        text(
            "commands/modernize.md",
            r######"---
description: Prepare a scoped implementation for one approved ModernLink migration task.
argument-hint: [task-id]
---

Load the selected task, its prerequisite evidence, tests, and rollback strategy. Invoke the
modernization-orchestrator and appropriate bounded specialist. Stop if the task is not approved
or lacks a verification oracle.
"######,
        ),
        text(
            "commands/plan.md",
            r######"---
description: Build a cited ModernLink migration prerequisite DAG for a target Java runtime.
argument-hint: [target-major]
---

Use one evidence graph to run `modernlink seams`, `modernlink compatibility --target $1`, and
`modernlink plan`. Present task dependencies, evidence IDs, verification gates, and approval
requirements. Invoke modernization-orchestrator for unknowns.
"######,
        ),
        text(
            "commands/prepare.md",
            r######"---
description: Establish characterization and rollback evidence before modernization work begins.
argument-hint: [plan-path]
---

Run `modernlink verify --plan $1` first. It verifies only static plan integrity and declared
approval gates. Invoke the preparation-auditor role to identify the missing behavioral, contract,
data, observability, performance, security, and rollback oracles for the selected seam.

Produce a preparation checklist with owners, evidence to capture, acceptance criteria, and the
explicit approval still required. Do not claim that a plan is safe to implement merely because its
static structure passes. Do not advance `MODERNIZE`, `CUTOVER`, or `DETACH` without the required
recorded human approval.
"######,
        ),
        text(
            "commands/status.md",
            r######"---
description: Show a ModernLink modernization lifecycle state without changing it.
argument-hint: [run-id]
---

Run `modernlink status` with the project lifecycle journal. Summarize phase, event sequence,
artifact digests, and unfulfilled gates. Do not infer completion from the journal alone.
"######,
        ),
        text(
            "commands/verify.md",
            r######"---
description: Independently review a ModernLink migration task before a protected transition.
argument-hint: [task-id]
---

Invoke verification-auditor. Report behavior, failure, security, observability, and rollback
evidence; clearly separate observations from unresolved risk. Do not advance lifecycle state.
"######,
        ),
        text(
            "skills/analyze-repository/SKILL.md",
            r######"---
name: modernlink-analyze-repository
description: This skill should be used when the user asks to "analyze a legacy Java repository", "discover architecture boundaries", "build a modernization evidence graph", or "find where to modernize a Java system" with ModernLink.
metadata:
  version: 0.2.0
---

# Analyze a Repository with ModernLink

Use the installed ModernLink Rust binary to collect deterministic repository evidence before
making architectural or modernization claims. Keep the runtime compatibility library and this
preparation ecosystem as separate domains.

## Required procedure

1. Locate `../../config/binary-pointer.json` relative to this skill file.
2. Stop with a setup error when the pointer is absent, its `schema_version` is not
   `modernlink.binary-pointer/v1`, or its `analysis_schema` is not
   `modernlink.analysis/v1alpha1`.
3. Resolve `binary_path` from the pointer. Refuse to guess another installation path.
4. Ask for the repository root when it is not explicit. Confirm exclusions, generated or vendored
   trees, desired modernization outcome, and whether analysis may write `.modernlink/` artifacts.
5. Create the output directory only after the scope is coherent.
6. Execute the binary directly:

   ```text
   <binary_path> analyze <repository-root> --output <repository-root>/.modernlink/evidence/analysis.json
   ```

7. Read the emitted summary and evidence graph. Treat `artifacts`, `evidence`, `nodes`, and
   `edges` as observed syntax evidence only.
8. Surface every non-`complete` `parse_health` value before drawing conclusions.
9. Label architectural boundaries, domain groupings, and modernization seams as hypotheses until
   deterministic rules or a human review promote them.
10. Ask follow-up questions whenever repository evidence conflicts with declared architecture,
    deployment reality, ownership, or migration intent. Continue until material ambiguity is
    resolved, explicitly deferred with an owner, or recorded as a blocker.

## Initial evidence scope

Expect Java artifacts, recognized build/deployment descriptors, package and declared-type nodes,
package containment, import relationships, source byte spans, content digests, parse health,
stable content-derived IDs, deterministic JSON ordering, and rule-cited technology signals.

Interpret `signals` according to `epistemic_state`. A `derived` signal means that a deterministic
rule matched cited evidence—for example, `pom.xml`, `jboss-web.xml`, or an `org.jboss.*` import.
Do not convert that match into a claim about the production server, runtime calls, domain
ownership, bounded contexts, or safe migration seams without corroborating evidence and review.

## Migration-readiness questions

Before recommending component migration, collect answers for:

- the exact component boundary and production responsibility;
- baseline build and test commands at a pinned revision;
- component-scoped line, branch, function, and mutation coverage policy;
- white-box tests for internal decisions and failure paths;
- black-box tests for public contracts and compatibility behavior;
- integration tests for databases, messaging, application servers, security, and external systems;
- production inputs, outputs, side effects, timing, ordering, error, and rollback oracles;
- missing tests, accepted risks, owners, expiry dates, and parity approvers.

Never convert absent evidence into readiness. Record the gap and ask the next coherence question.

## Safety and reporting

- Never load, build, or modify the ModernLink runtime library to perform repository preparation.
- Never execute an unbound binary or silently update the pointer.
- Never expose credentials, payloads, or proprietary source text in summaries.
- Report machine results as observations. Leave correctness and migration approval to the human.
"######,
        ),
        text(
            "skills/modernlink-architecture/SKILL.md",
            r######"---
name: modernlink-architecture
description: Evidence-first architecture and candidate-domain reasoning for ModernLink.
---

# ModernLink Architecture

1. Require a current `modernlink.evidence/v1alpha1` graph before reasoning.
2. Run `modernlink architecture` and, where boundary behavior matters, `modernlink boundaries`.
3. Label outputs exactly as FACT, INFERENCE, HYPOTHESIS, or USER_CONFIRMED.
4. Cite evidence IDs and source locations for every layer, context, or boundary statement.
5. Do not turn package names, annotations, imports, or Git co-change into proof of business
   ownership, runtime flow, or a bounded context.

Return unknowns that need operator confirmation before planning modernization work.
"######,
        ),
        text(
            "skills/modernlink-assess/SKILL.md",
            r######"---
name: modernlink-assess
description: This skill should be used when the user asks which legacy boundary to modernize first, what blocks a target Java runtime, or what migration risk remains.
metadata:
  version: 0.1.0
---

# Assess Modernization Evidence

Require current seam, boundary, and target compatibility reports derived from one evidence graph.
Explain modernization leverage, migration risk, blast radius, isolation quality, and confidence as
decomposed evidence-backed components, never as false mathematical precision.

Separate:

- observed vendor/API coupling;
- structural inference;
- unverified operational or business assumptions;
- user-confirmed policy or priority.

Do not select a replacement broker, database, protocol, or cutover mode from static evidence alone.
Surface missing transaction, delivery, data ownership, testability, observability, and rollback
evidence before recommending a seam for implementation.
"######,
        ),
        text(
            "skills/modernlink-domains/SKILL.md",
            r######"---
name: modernlink-domains
description: This skill should be used when the user asks to discover domain vocabulary, candidate bounded contexts, ownership hypotheses, or business boundaries in a legacy repository.
metadata:
  version: 0.1.0
---

# Discover Candidate Domains

Start with a current `modernlink.evidence/v1alpha1` graph and `modernlink architecture`. Cluster
only cited structural evidence: package/module cohesion, names, data references, endpoints,
message destinations, transaction candidates, and bounded Git co-change.

Return `CandidateBoundedContext` rather than `BoundedContext`. Every proposal must include:

- confidence and epistemic state;
- supporting and contradicting evidence IDs;
- source locations and vocabulary;
- an operator question needed for confirmation.

Never infer people, team ownership, business criticality, or a DDD model as a fact from source
layout or commit history alone.
"######,
        ),
        text(
            "skills/modernlink-modernize/SKILL.md",
            r######"---
name: modernlink-modernize
description: This skill should be used when the user asks to modernize one approved ModernLink seam or implement a scoped migration task.
metadata:
  version: 0.1.0
---

# Modernize One Approved Seam

Work only on an identified migration task. First reconcile its migration record with
`modernlink migration status --id <id>`, then read its seam evidence, compatibility findings,
plan prerequisites, current lifecycle state, contract tests, and rollback condition. Stop when
the record reports a gap or the task lacks explicit approval, a behavior oracle, or a bounded
file/component scope.

Do not perform unrelated cleanup. Preserve ordering, acknowledgement, transaction, security,
and error semantics unless the approved migration specification explicitly changes them. Record
new evidence and advance lifecycle state only through the ModernLink CLI.
"######,
        ),
        text(
            "skills/modernlink-plan/SKILL.md",
            r######"---
name: modernlink-plan
description: This skill should be used when the user asks for a ModernLink migration plan, dependency DAG, or safe modernization sequence for a legacy repository.
metadata:
  version: 0.1.0
---

# Plan a ModernLink Migration

Use deterministic reports before proposing a sequence. Run `modernlink seams`,
`modernlink compatibility --target <major>`, and `modernlink plan` against the same evidence
graph. Present the resulting prerequisite DAG as evidence-backed planning material, not an
authorization to modify code or cut traffic over.

For every proposed migration task, state its evidence IDs, unknowns, required verification
oracles, rollback precondition, and whether it needs explicit human approval. Never choose a
broker, database, protocol, or target technology merely from static import evidence.
"######,
        ),
        text(
            "skills/modernlink-prepare/SKILL.md",
            r######"---
name: modernlink-prepare
description: This skill should be used when the user asks to prepare, de-risk, or establish verification for a planned ModernLink modernization seam.
metadata:
  version: 0.1.0
---

# Prepare a Modernization Seam

Before code changes, establish how present behavior will be observed and compared. Begin with
`modernlink verify --plan <path>` and state its static-only scope.

For the selected seam, request or define the necessary characterization, contract, integration,
data, observability, performance, security, failure-handling, and rollback evidence. Tie each
oracle to cited seam/compatibility evidence and name an owner and acceptance condition.

Do not claim preparation is complete until the required evidence exists or an operator explicitly
accepts the residual risk. Never advance protected lifecycle phases without recorded approval.
"######,
        ),
        text(
            "skills/modernlink-verify/SKILL.md",
            r######"---
name: modernlink-verify
description: This skill should be used when the user asks to verify a ModernLink migration, shadow phase, cutover, or detachment decision.
metadata:
  version: 0.1.0
---

# Verify a ModernLink Migration

Act independently from the implementation role. Check the selected seam's declared contract,
characterization/contract/integration tests, failure handling, observability, performance
baseline, and rollback mechanism. For messaging, explicitly review ordering, acknowledgement,
transactions, retry, dead-letter, and correlation behavior. For data boundaries, review writes,
ownership, transactions, and recovery.

When a migration record exists, begin with `modernlink migration status --id <id>`. Treat every
reported `GAP` as a verification blocker: do not reason from a plan whose recorded hash,
metadata, or lifecycle journal is inconsistent.

Report observations, gaps, and evidence IDs. Do not declare cutover or detachment safe; that is a
human approval decision and requires the protected lifecycle transition.
"######,
        ),
        text(
            "skills/drawio-skill/SKILL.md",
            r################################"---
name: drawio-skill
version: 2.1.0
description: Use when the user requests diagrams, flowcharts, architecture diagrams, ER diagrams, UML / sequence / class diagrams, SysML / MBSE diagrams (block definition, internal block, requirement, parametric), BPMN business process diagrams, swimlane / cross-functional flowcharts, network topology, cloud architecture from Terraform or Kubernetes manifests, ML/DL model figures (Transformer/CNN/LSTM), mind maps, or any visualization. Also use proactively when explaining systems with 3+ components, complex data flows, or relationships that benefit from visual representation. Best suited when the diagram needs custom styling, rich shape vocabulary, swimlanes, or exportable images (PNG/SVG/PDF/JPG). Generates .drawio XML and exports locally via the native draw.io desktop CLI.
license: MIT
homepage: https://github.com/Agents365-ai/drawio-skill
compatibility: Requires draw.io desktop app CLI on PATH (macOS/Linux/Windows). Self-check step requires a vision-enabled model (e.g., Claude Sonnet/Opus); gracefully skipped if unavailable. Optional auto-layout (scripts/autolayout.py) needs Graphviz (dot).
platforms: [macos, linux, windows]
metadata: {"openclaw":{"requires":{"anyBins":["draw.io","drawio"]},"emoji":"📐","os":["darwin","linux","win32"],"install":[{"id":"brew-drawio","kind":"brew","formula":"drawio","bins":["drawio"],"label":"Install draw.io via Homebrew","os":["darwin"]},{"id":"brew-graphviz","kind":"brew","formula":"graphviz","bins":["dot"],"label":"Install Graphviz for optional autolayout.py","os":["darwin"],"optional":true}]},"hermes":{"tags":["drawio","diagram","flowchart","architecture","visualization","uml"],"category":"design","requires_tools":["drawio","draw.io"],"related_skills":["mermaid","excalidraw","plantuml"]},"author":"Agents365-ai","version":"2.1.0"}
---

# Draw.io Diagrams

## Overview

Generate `.drawio` XML files and export to PNG/SVG/PDF/JPG locally using the native draw.io desktop app CLI.

**Supported formats:** PNG, SVG, PDF, JPG — no browser automation needed.

PNG, SVG, and PDF exports support `--embed-diagram` (`-e`) — the exported file contains the full diagram XML, so opening it in draw.io recovers the editable diagram. Use double extensions (`name.drawio.png`) to signal embedded XML.

## When to use / when NOT to use

**Use this skill for:** polished, precise diagrams (architecture, network, strict UML, ERD), anything needing solid opaque fills, 10,000+ stock/branded shapes, swimlanes, or custom geometry, exported as editable PNG/SVG/PDF.

**Do NOT use it — route elsewhere — for:**

- A casual hand-drawn / whiteboard look → **excalidraw** or **tldraw**.
- Diagrams-as-code that live in git / render in Markdown → **mermaid** (general) or **plantuml** (UML).
- Freeform infinite-canvas sketching or freehand strokes → **tldraw**.

## Bundled resources

When the workflow references one of these, read it on demand — none of them need to be in context up front.

| File | Read it when |
| --- | --- |
| `references/toolbox.md` | You're **not sure which bundled script fits** a request, or want to chain several — a map of all 31 scripts grouped by use-case (author / import code / import IaC / import API spec / live infra / compare / annotate / reverse-export / utilities) with an "I have X, I want Y → use Z" guide |
| `references/xml-authoring.md` | You're about to **hand-write `.drawio` XML** (workflow step 3) — file skeleton, shape/edge cells, containers, connection distribution, palette, spacing/grid rules. Not needed when a bundled generator writes the XML |
| `references/mermaid-authoring.md` | The diagram is a **standard type with no custom styling/icon needs** (flowchart, state, gantt, mindmap, timeline, journey, pie, …) and the CLI is **≥ v30** — author it as Mermaid text and let the CLI convert to native `.drawio` (structure only, layout free). Also documents the CLI's ELK `--layout` pass for XML |
| `references/diagram-types.md` | The user names a specific diagram type (ERD, UML class, sequence, C4, architecture, ML/DL, flowchart, SysML, BPMN, network topology, swimlane) |
| `references/shapes.md` + `scripts/shapesearch.py` | The diagram needs a **specific shape** — a cloud icon (AWS/Azure/GCP), Cisco/Kubernetes/network symbol, UML/BPMN/ER/electrical/P&ID element — or any time you'd otherwise guess a `style=` string. `shapesearch.py "<keywords>"` returns the exact official style for 10k+ shapes |
| `scripts/aiicons.py` | The diagram involves an **AI/LLM brand** (OpenAI, Claude, Gemini, Mistral, Llama, HuggingFace, Ollama, LangChain, …) — `aiicons.py "<brand>"` returns a draw.io `image` style for the brand logo (lobe-icons via CDN; `--embed` to inline). draw.io has no built-in AI logos. See `references/shapes.md` → "AI / LLM brand logos" |
| `references/style-presets.md` | The user asks to learn / save / list / set-default / delete a style preset, or you've resolved an active preset and need the application rules |
| `references/style-extraction.md` | You're inside the Learn flow and need the extraction procedure (called from `style-presets.md`) |
| `references/troubleshooting.md` | An export fails, vision rejects a PNG, or a rendering looks wrong |
| `scripts/repair_png.py` | After every `-e` PNG export — fixes draw.io's truncated IEND chunk (issue #8) |
| `scripts/encode_drawio_url.py` | The CLI is unavailable and you need a browser-fallback diagrams.net URL (`--edit` for an editable editor URL) |
| `references/autolayout.md` | The diagram is large or layout-heavy (dependency/call graph, code structure, >~15 nodes) and you want Graphviz to place nodes + route edges instead of hand-placing coordinates |
| `scripts/pyimports.py` · `jsimports.py` · `goimports.py` · `rustimports.py` | The user wants to visualize a **Python, JS/TS, Go, or Rust project** structure — extracts the import graph (transitive-reduced, optional `--group` containers, nested by sub-package) for autolayout |
| `scripts/pyclasses.py` | The user wants a **Python class hierarchy / class diagram** — extracts classes + inheritance edges (boxed by module with `--group`) for autolayout |
| `scripts/tfimports.py` · `k8simports.py` · `composeimports.py` | The user wants to visualize **declared** infrastructure (**Terraform** `.tf`, **Kubernetes** manifests, or **docker-compose**) — extracts the resource/service reference graph (official AWS/Azure/GCP/K8s icons for tf/k8s; service boxes + volume cylinders for compose) for autolayout |
| `scripts/tfstate.py` · `dockerimports.py` (+ `k8simports.py`) | The user wants to draw **what is ACTUALLY running / deployed** — pipe `terraform show -json` (deployed state), `docker inspect $(docker ps -q)` (live containers), or `kubectl get all,ing,cm,secret,pvc -o json` (live cluster, via k8simports) and get the real topology with the same official icons. See `references/live-infra.md` |
| `scripts/drawiodiff.py` | The user wants to **compare / diff two diagrams or two snapshots** ("what changed", infra drift) — `drawiodiff.py old.drawio new.drawio -o diff.json` emits a colour-coded graph (added=green, removed=red, changed=orange, same=grey) for autolayout. Matches by cell id (importer/live-snapshot output) or `--by-label` (hand-drawn) |
| `scripts/timelapse.py` | The user wants an **architecture time-lapse / to see how a codebase's structure evolved over git history** — `timelapse.py <dir> --importer pyimports` re-runs an importer at each sampled commit and assembles a self-contained HTML player (embedded frames, play/step controls). Best on a package with real import edges (point `<dir>` at the module root) |
| `scripts/explain.py` | The user wants to **describe / document / summarize an existing `.drawio` in words** (reverse of generating one) — `explain.py diagram.drawio` emits structured Markdown: components grouped by container/tier, relations (`A —label→ B`), per-page sections for multi-page/C4. Good for a README/PR summary or a text-only read-out |
| `scripts/drawio2pptx.py` | The user wants a **PowerPoint deck / slides from a diagram** — `drawio2pptx.py diagram.drawio -o deck.pptx` puts each page on its own 16:9 slide (page name as title), so a multi-page **C4 model** becomes a ready-to-present deck. Needs `python-pptx` (`pip install python-pptx`) + the draw.io CLI |
| `scripts/drawiohtml.py` | The user wants a **shareable interactive viewer** for a diagram (pan / zoom / search, no draw.io needed) — `drawiohtml.py diagram.drawio -o viewer.html` inlines every page's SVG into ONE self-contained HTML with page tabs, drag-pan, wheel-zoom, node search (Enter cycles + centres matches) and **working drill-down links** (a C4 model's `data:page/id` links switch tabs). No server, no external requests — send the file to anyone |
| `scripts/svgflow.py` | The user wants an **animated / "flowing" diagram** (data-flow, moving edges) — `svgflow.py diagram.drawio -o flow.svg` exports to SVG and makes every edge a marching-ants animation (dashes travel along the arrows). Self-contained looping `.svg` that renders on GitHub / any browser; `--speed` / `--dash` / `--reverse` |
| `scripts/drawio2mermaid.py` | The user wants to **convert a `.drawio` into Mermaid text** (diagrams-as-code for a Markdown file that GitHub renders) — `drawio2mermaid.py diagram.drawio` emits a `flowchart` (containers → `subgraph`s, edge labels kept, cylinder/rhombus shapes mapped); `--fenced` wraps in ```mermaid, multi-page → one graph per page. Structural only (styling/icons don't survive) |
| `scripts/sqlerd.py` | The user wants an **ER diagram from SQL DDL** — parses `CREATE TABLE` statements into per-table nodes (columns with PK/FK markers) and crow's-foot FK edges for autolayout |
| `scripts/ciimports.py` | The user wants a **CI pipeline diagram** (GitHub Actions workflows or GitLab CI) — `ciimports.py <repo-root>` reads `.github/workflows/*.yml` + `.gitlab-ci.yml` and emits jobs (runner, matrix size, reusable-workflow calls), `needs:` dependency edges, per-workflow trigger nodes, and stage/workflow containers for autolayout. Needs PyYAML |
| `scripts/openapiimports.py` | The user wants an **API diagram from an OpenAPI / Swagger spec** — `openapiimports.py spec.yaml` maps each operation to a node **coloured by HTTP method** (GET blue, POST green, PUT/PATCH orange, DELETE red) plus one node per component schema, with edges from operations to the schemas they use and between nested schemas. `--group` boxes by tag, `--no-schemas` shows just the endpoint surface; feeds autolayout |
| `scripts/heatmap.py` | The user wants to **colour an existing `.drawio` by data** (a cost / latency / traffic / error-rate heat map) — `heatmap.py diagram.drawio -m metrics.csv` matches each metric (CSV `key,value` or JSON `{key:value}`) to a node by id or label and recolours it along a gradient (`--palette heat\|cool\|warm`, `--reverse`), optionally scaling node size (`--size`) and adding a legend. Post-processes any diagram; export as usual |
| `scripts/seqlayout.py` | The user wants a **sequence diagram** — describe participants + messages as JSON and the script computes all lifeline/activation/arrow geometry deterministically (no hand-placed coordinates, no Graphviz needed) |
| `scripts/c4.py` | The user wants a **C4 model** (System Context / Container / Component) — levels JSON in, one multi-page `.drawio` out with official C4 shapes/colors and **click-to-drill-down** links between levels |
| `scripts/relabel.py` | The user wants a **language variant or bulk text swap of an existing `.drawio`** (e.g. an EN diagram re-labelled in Chinese for a bilingual README) — `relabel.py diagram.drawio --extract -o labels.json` dumps every label as an identity JSON map; translate the values (keep the keys), then `relabel.py diagram.drawio --map labels.json -o diagram_cn.drawio` swaps them with layout/styles/ids untouched |
| `scripts/restyle.py` | The user wants to **re-theme an EXISTING `.drawio`** ("make this dark", "apply my corporate style to this diagram") — `restyle.py diagram.drawio --preset <name>` remaps every vertex fill/stroke to the preset palette by hue, applies font/extras (dark fontColor, edge color, background), and leaves layout, shapes, and edge routing untouched. Presets resolve like Step 0 (user dir, then built-ins) |
| `scripts/edgeports.py` | Edges **stack on top of each other where they meet a shape** — the usual swimlane/cross-functional complaint, and anywhere a node has several connections leaving the same side. `edgeports.py diagram.drawio` pins `exitX/exitY`+`entryX/entryY`: it picks the side of each node facing the other endpoint, then spreads that side's edges over evenly-spaced slots **ordered by the far endpoint's position**, so they keep their relative order instead of crossing. Resolves absolute coordinates through swimlane parents, skips ends you already pinned, and is idempotent. It is a **port assigner, not a router** — it separates lines at the shape boundary, it will not stop an edge crossing an unrelated shape mid-run (add waypoints for that) |
| `scripts/validate.py` | You generated a `.drawio` (especially via autolayout or for a large hand-placed diagram) and want a fast deterministic structural lint (dangling edges, dup/reserved ids, broken parents, overlaps) before the vision self-check. `--score` prints a readability score for comparing layout variants |
| `scripts/raster2drawio.py` | The user has an **image of a diagram** (whiteboard photo, legacy PNG, Visio screenshot) and wants an **editable `.drawio`** — read the image with your own vision, extract nodes/edges as JSON (schema + full workflow in `references/derasterize.md`), then `raster2drawio.py graph.json -o out.drawio` honours those coordinates/labels/shapes; nodes missing `x`/`y` fall back to `autolayout.py` placement |
| `scripts/buildup.py` | The user wants a diagram to **build itself node-by-node** as a video/GIF (a construction time-lapse of ONE static diagram — distinct from `timelapse.py`'s git-history animation) — `buildup.py diagram.drawio` reveals cells in topological (dependency) order into a self-contained HTML player (play/pause/step/scrub); `--gif` also exports an animated GIF (needs Pillow). Needs the draw.io CLI |
| `scripts/compress.py` | The user wants an **executive / boardroom summary of a big diagram** — collapses clusters (pure-Python label propagation, no networkx) into one labeled node each with aggregated inter-cluster edges, emitting a 2-page `.drawio` (exec view + click-to-drill-down into the full original). Claude can rename clusters semantically afterward. Needs Graphviz `dot` |
| `scripts/runbook.py` | The user wants a flowchart/decision-tree `.drawio` turned into a **click-through triage app** (on-call runbook) — `runbook.py flow.drawio` reads the XML (no draw.io CLI needed) and emits a self-contained HTML runbook: current-step text, per-edge choice buttons, breadcrumb trail, Back/Restart, end-state on terminal nodes |
| `scripts/prdiff.py` | You're setting up **automated PR diagram review** in CI — for every `.drawio` changed between two git refs it renders base/head/diff PNGs and emits a Markdown report; ships with a composite GitHub Action (`.github/actions/drawio-diff/`) that posts a sticky PR comment. See `references/pr-bot.md` |
| `scripts/tubemap.py` | The user wants a **metro / subway / tube map** — a system, pipeline, or journey drawn as coloured transit lines with octilinear (H/V/45°) routing, white interchange circles, and station stops. Compose a metro JSON (lines = ordered stations on an integer grid, shared stations = interchanges), then `tubemap.py metro.json -o metro.drawio`. Stdlib-only; schema + the one grid rule in `references/tubemap.md` |

## Prerequisites

The draw.io desktop app must be installed and the CLI accessible:

**macOS sandbox / sandbox isolation note (e.g., codex.app):** In some sandboxed macOS environments, invoking the draw.io desktop CLI (even `drawio --version`) can crash the draw.io process or produce no output. If that happens, treat the CLI as **unavailable in this sandbox isolation** — do not keep retrying inside the sandbox. Prefer a **non-sandboxed host environment** (outside sandbox isolation) for any CLI export work, or use the browser fallback / XML-only outputs.

```bash
# macOS (Homebrew — recommended; CLI binary is `drawio`, not `draw.io`)
brew install --cask drawio
drawio --version

# macOS (full path if not in PATH)
/Applications/draw.io.app/Contents/MacOS/draw.io --version

# Windows
"C:\Program Files\draw.io\draw.io.exe" --version

# Linux
drawio --version
```

Install draw.io desktop if missing:

- macOS: `brew install --cask drawio` or download from <https://github.com/jgraph/drawio-desktop/releases>
- Windows: download installer from <https://github.com/jgraph/drawio-desktop/releases>
- Linux: download `.deb`/`.rpm` from <https://github.com/jgraph/drawio-desktop/releases> — **do not use snap** (AppArmor sandbox denies secrets/keyring on servers, causes crash)

## Workflow

Before starting the workflow, assess whether the user's request is specific enough. If key details are missing, ask 1-3 focused questions:

- **Diagram type** — which preset? (ERD, UML, Sequence, Architecture, ML/DL, Flowchart, SysML, BPMN, Network, Swimlane, or general)
- **Output format** — PNG (default), SVG, PDF, or JPG?
- **Output location** — default is the user's working dir; honor any explicit path the user gives (e.g. "put it in `./artifacts/`"). Don't ask if they didn't mention one.
- **Scope/fidelity** — how many components? Any specific technologies or labels?

Skip clarification if the request already specifies these details or is clearly simple (e.g., "draw a flowchart of X").

**Step 0 — Resolve active preset.** Determine which (if any) user-defined style preset applies to this generation.

- Scan the user's message for a phrase that clearly names a style preset: "use my `<name>` style", "with my `<name>` style", "in `<name>` mode", "in the style of `<name>`". A bare `with <name>` does **not** count — "draw a diagram with redis" names a component, not a style. If a clear match is found → active preset = `<name>`.
- Else, check `~/.drawio-skill/styles/` for any file with `"default": true`. If found → active preset = that one.
- Else → no preset active; fall through to the built-in color/shape/edge conventions for the rest of the workflow.

Load the preset JSON from `~/.drawio-skill/styles/<name>.json`, falling back to `<this-skill-dir>/styles/built-in/<name>.json`. If the named preset exists in neither location, tell the user the name is unknown, list the available presets (user dir + built-in), and stop — do **not** silently fall back to defaults.

When a preset loads successfully, mention it in the first line of the reply: *"Using preset `<name>` (confidence: `<level>`)."* See `references/style-presets.md` → "Applying a preset" for how the preset changes color/shape/edge/font decisions.

1. **Check deps** — **resolve which name the binary has on this system** and use that name verbatim in every subsequent command in this workflow. Try in order: (a) `drawio --version` (the canonical name for Homebrew cask, jgraph `.deb`/`.rpm`, Arch AUR), (b) `draw.io --version` (older builds, some custom symlinks, some distro packages), (c) macOS `.app` direct: `/Applications/draw.io.app/Contents/MacOS/draw.io --version`, (d) Windows: `"C:\Program Files\draw.io\draw.io.exe" --version`. The first one that prints a version is your binary; remember the exact path/name and substitute it for `drawio` in every export command below. **Do not copy the example commands verbatim if your binary is named differently** — the examples use `drawio` only because it's the most common. On macOS-Homebrew, `drawio` is just a thin wrapper script that execs `/Applications/draw.io.app/Contents/MacOS/draw.io` — they run the same engine, so candidate (c) is only needed when the `drawio` wrapper is absent (e.g. the app was installed by drag-and-drop without the cask). **Also note the major version** the command printed: **≥ 30** unlocks Mermaid→`.drawio` conversion and the ELK `--layout` pass (see `references/mermaid-authoring.md`); on **≤ 29** both are unavailable — `.mmd` input fails and `--layout` corrupts argument parsing — so never emit those flags there.
2. **Plan** — identify shapes, relationships, layout (LR or TB), group by tier/layer
3. **Generate** — produce the `.drawio` file, choosing the authoring mode: **(a) Mermaid → CLI convert** when the diagram is a standard type with no custom styling/icon needs **and** the CLI is ≥ v30 — write a `.mmd` and run `drawio -x -f xml -o <name>.drawio <name>.mmd`, see `references/mermaid-authoring.md` (structure only; layout comes free; never `--layout` afterwards). **(b) Hand-written XML** for custom styling, vendor icons, swimlanes, precise geometry — **read `references/xml-authoring.md` first** (skeleton, cell forms, palette, spacing rules). **(c) A bundled generator** for the data-driven cases below. **For large or layout-heavy diagrams (dependency/call graphs, code structure, >~15 nodes), don't hand-place** — describe the graph as JSON and run `python3 <this-skill-dir>/scripts/autolayout.py graph.json -o <name>.drawio` to compute node positions + orthogonal edge routing via Graphviz (see `references/autolayout.md`; add `--tune` to auto-pick the more readable direction). For a **Python / JS-TS / Go / Rust project**, the matching importer (`scripts/pyimports.py`, `jsimports.py`, `goimports.py`, or `rustimports.py`) extracts the import graph (transitive-reduced; add `--group` to box modules by sub-package, nested for deep trees) ready for autolayout; for a **Python class hierarchy**, `scripts/pyclasses.py` extracts classes + inheritance instead; for **Terraform / Kubernetes / docker-compose** (`scripts/tfimports.py`, `k8simports.py`, `composeimports.py`), the importer extracts the resource/service reference graph — tf/k8s nodes resolve to their official cloud icons automatically; to draw **what is actually running** rather than the declared config, pipe `terraform show -json` into `scripts/tfstate.py` or `docker inspect $(docker ps -q)` into `scripts/dockerimports.py` (`k8simports.py` already accepts live `kubectl get ... -o json`) — see `references/live-infra.md`; for an **ER diagram from SQL DDL**, `scripts/sqlerd.py` parses `CREATE TABLE` into table nodes + crow's-foot FK edges; for an **API diagram from an OpenAPI / Swagger spec**, `scripts/openapiimports.py` maps operations (coloured by HTTP method) + component schemas into a graph for autolayout; for a **CI pipeline diagram** (GitHub Actions / GitLab CI), `scripts/ciimports.py` extracts jobs, `needs:` edges, triggers, and stage/workflow containers. To turn any generated `.drawio` into a **metric heat map** — recolour nodes by a CSV/JSON of cost/latency/traffic/errors — run `python3 <this-skill-dir>/scripts/heatmap.py <name>.drawio -m metrics.csv` (matches on cell id or label; `--palette`, `--size`, legend). For a **sequence diagram**, skip autolayout entirely — describe participants + messages as JSON and run `python3 <this-skill-dir>/scripts/seqlayout.py seq.json -o <name>.drawio` (deterministic lifeline/activation/arrow geometry; see the script docstring for the JSON schema). For a **C4 model**, `python3 <this-skill-dir>/scripts/c4.py c4.json -o <name>.drawio` emits the full multi-page Context→Container→Component set with drill-down links (schema in the script docstring). For complex architecture diagrams with many visible edge labels, give labels `labelBackgroundColor=#ffffff;fontSize=11` and use edge geometry `x`/`y` offsets plus `<mxPoint as="offset" />` to move long labels into nearby whitespace instead of relying on draw.io's default midpoint placement. For hand-placed diagrams where edges cross shapes (architecture, network topology, deployment, UML), fix the routing in the XML — run `python3 <this-skill-dir>/scripts/edgeports.py <name>.drawio` to distribute stacked edges over each shape's perimeter automatically, then add `<Array as="points">` waypoints or widen node spacing for any edge still crossing a shape mid-run (see `references/xml-authoring.md`). **No CLI flag reroutes edges without moving nodes**: every `--layout` preset is an ELK *node* layout that re-places vertices, and an unrecognised value opens a modal error dialog that hangs headless runs. draw.io's obstacle-avoiding router is editor-side only. After generating any `.drawio`, run `python3 <this-skill-dir>/scripts/validate.py <name>.drawio` for a fast structural lint (dangling edges, dup ids, overlaps) before exporting. Default output dir is the user's working dir; if the user specified an output path or directory (e.g. `./artifacts/`, `docs/images/`), use that instead — `mkdir -p` the target dir first. Apply the same dir choice to PNG/SVG/PDF exports in steps 4 and 7.
4. **Export draft** — run CLI to produce a preview PNG. **Do NOT pass `-e`** at this step — the embedded `zTXt mxGraphModel` chunk it adds causes vision APIs (Claude included) to return 400 "Could not process image" in step 5. **Cap the preview width with `--width 2000` (not `-s 2`)** — Claude's vision API rejects images larger than 2576×2576px with "Unable to resize image — dimensions exceed the 2576x2576px limit", and `-s 2` on a medium-or-larger diagram easily overshoots that ceiling. Save the clean preview as `<name>.png` (single extension). Embedding and full-resolution scale are for the final export only (step 7).
5. **Self-check** — use the agent's built-in vision capability to read the exported PNG, catch obvious issues, auto-fix before showing user (requires a vision-enabled model such as Claude Sonnet/Opus). If reading the PNG returns a 400 / "Could not process image" error, you almost certainly exported with `-e` by mistake — re-export without `-e` and retry once. If it still fails, skip self-check and continue to step 6.
6. **Review loop** — show image to user, collect feedback, apply targeted XML edits, re-export, repeat until approved
7. **Final export** — re-export the approved version to all requested formats. Use `-e` here (PNG/SVG/PDF) so the deliverable stays editable in draw.io; save as `<name>.drawio.png` to signal embedded XML. **For PNG with `-e`, run `python3 <this-skill-dir>/scripts/repair_png.py <name>.drawio.png` immediately after** — draw.io's CLI truncates the IEND chunk in `-e` PNG output (8 bytes missing), producing a corrupt file that vision APIs and strict PNG decoders reject (issue #8). Report file paths.

**If `drawio --version` crashes or prints nothing (common in restricted macOS sandbox isolation like codex.app):**

- Do not keep retrying CLI invocations inside the sandbox.
- Skip steps 4, 5, 6, and 7 (CLI export + PNG-based review) and use **Browser fallback** (`scripts/encode_drawio_url.py`) or deliver the `.drawio` XML only.
- If the user needs PNG/SVG/PDF outputs, ask them to run the export commands in a **non-sandboxed host environment** (outside sandbox isolation) and share the resulting files.

Escalation rule:

- If the binary exists on PATH (or known app path exists) but execution fails with abnormal exit, empty output, Electron startup failure, display/session error, or likely sandbox restriction, prefer one escalated retry before falling back.
- If the binary is missing entirely, do not escalate just to search more aggressively; go to install guidance or fallback.

### Step 5: Self-Check

After exporting the draft PNG, use the agent's vision capability (e.g., Claude's image input) to read the image and check for these issues before showing the user. If the agent does not support vision, skip self-check and show the PNG directly.

**Important:** the draft PNG read here must have been exported **without** `-e`. Draw.io's `-e` flag emits a PNG with a truncated IEND chunk (8 bytes of type+CRC missing) that the Anthropic vision API rejects with 400 "Could not process image" (issue #8). The simplest fix for the preview step is to skip `-e` entirely; the final export in step 7 keeps `-e` and runs the repair snippet. If you see the 400 error here, re-export without `-e` and retry once; if it still fails (any other reason), skip self-check and proceed to step 6.

| Check | What to look for | Auto-fix action |
| ------- | ----------------- | ----------------- |
| Overlapping shapes | Two or more shapes stacked on top of each other | Shift shapes apart by ≥200px |
| Clipped labels | Text cut off at shape boundaries | Increase shape width/height to fit label |
| Missing connections | Arrows that don't visually connect to shapes | Verify `source`/`target` ids match existing cells |
| Off-canvas shapes | Shapes at negative coordinates or far from the main group | Move to positive coordinates near the cluster |
| Edge-shape overlap | An edge/arrow visually crosses through an unrelated shape | Add waypoints (`<Array as="points">`) to route around the shape, or increase spacing between shapes |
| Stacked edges | Multiple edges overlap each other on the same path | Distribute entry/exit points across the shape perimeter (use different exitX/entryX values) |
| Edge-label overlap | Edge text overlaps another label, line, or node in the exported PNG | Keep the label on the edge, add a white label background, and move it locally with edge geometry `x`/`y` offsets into adjacent whitespace |

- Max **2 self-check rounds** — if issues remain after 2 fixes, show the user anyway
- Re-export after each fix and re-read the new PNG

### Step 6: Review Loop

After self-check, show the exported image and ask the user for feedback.

**Targeted edit rules** — for each type of feedback, apply the minimal XML change:

| User request | XML edit action |
| ------------- | ---------------- |
| Change color of X | Find `mxCell` by `value` matching X, update `fillColor`/`strokeColor` in `style` |
| Add a new node | Append a new `mxCell` vertex with next available `id`, position near related nodes |
| Remove a node | Delete the `mxCell` vertex and any edges with matching `source`/`target` |
| Move shape X | Update `x`/`y` in the `mxGeometry` of the matching `mxCell` |
| Resize shape X | Update `width`/`height` in the `mxGeometry` of the matching `mxCell` |
| Add arrow from A to B | Append a new `mxCell` edge with `source`/`target` matching A and B ids |
| Change label text | Update the `value` attribute of the matching `mxCell` |
| Change layout direction | **Full regeneration** — rebuild XML with new orientation |

**Rules:**

- For single-element changes: edit existing XML in place — preserves layout tuning from prior iterations
- For layout-wide changes (e.g., swap LR↔TB, "start over"): regenerate full XML
- Overwrite the same `{name}.png` (no `-e`) each iteration — do not create `v1`, `v2`, `v3` files. `-e` is reserved for the final export in step 7.
- After applying edits, re-export and show the updated image
- Loop continues until user says approved / done / LGTM
- **Safety valve:** after 5 iteration rounds, suggest the user open the `.drawio` file in draw.io desktop for fine-grained adjustments

### Step 7: Final Export

Once the user approves:

- Export to all requested formats (PNG, SVG, PDF, JPG) — default to PNG if not specified
- Report file paths for both the `.drawio` source file and exported image(s)
- **Auto-launch:** offer to open the `.drawio` file in draw.io desktop for fine-tuning — `open diagram.drawio` (macOS), `xdg-open` (Linux), `start` (Windows)
- Confirm files are saved and ready to use

## Style Presets

A **style preset** is a named JSON file capturing a user's visual preferences (palette, shapes, font, edges). When active, it fully replaces the built-in color/shape conventions in this skill.

**Lookup order** when SKILL.md's Step 0 resolves a preset name:

1. `~/.drawio-skill/styles/<name>.json` — user presets (survive `git pull`)
2. `<this-skill-dir>/styles/built-in/<name>.json` — shipped built-ins (`default`, `corporate`, `handdrawn`, `colorblind-safe`, `dark`)

Always lowercase the user-provided name before any file operation — the schema enforces lowercase.

**For everything else — Learn flow (extracting a preset from a file), management ops (list/default/delete/rename), application rules (color lookup, shape keywords, edges, fonts, extras, interaction with diagram-type presets), and validation — read `references/style-presets.md`.** It's only needed when the user invokes those flows or when an active preset must be applied to the current generation.

## Authoring .drawio XML

**Before hand-writing any `.drawio` XML (step 3), read `references/xml-authoring.md`** — file skeleton, shape/edge cell forms, containers, connection-point distribution, color palette, and spacing/grid rules all live there. Skip it only when a bundled generator writes the XML for you (`autolayout.py` + importers, `seqlayout.py`).

Two rules worth stating even here: never reuse ids `0`/`1` (reserved root cells), and every edge `mxCell` needs a `<mxGeometry relative="1" as="geometry" />` child — self-closing edge cells do not render.

## Export

### Commands

There are **two** export modes:

- **Preview / self-check** (step 4 of the workflow) — no `-e`. Output `diagram.png`. Required for vision self-check; using `-e` here triggers a 400 "Could not process image" error from the vision API (issue #8).
- **Final / deliverable** (step 7) — pass `-e`. Output `diagram.drawio.png`. The embedded XML keeps the file editable in draw.io.

> All commands below write `drawio` as a placeholder for the binary you resolved in Step 1. If your binary is on PATH as `draw.io` (with dot — some older or distro-packaged installs), substitute `draw.io` throughout. If only the macOS `.app` or Windows `.exe` is available, use the full path variant shown a few lines down.

```bash
# Preview PNG (use this in step 4, before self-check) — NO -e, width-capped to stay under vision's 2576px ceiling
drawio -x -f png --width 2000 -o diagram.png input.drawio

# Final PNG (step 7, after user approval) — WITH -e, double extension
drawio -x -f png -e -s 2 -o diagram.drawio.png input.drawio

# macOS — full path (if not in PATH); preview / final variants
/Applications/draw.io.app/Contents/MacOS/draw.io -x -f png --width 2000 -o diagram.png input.drawio
/Applications/draw.io.app/Contents/MacOS/draw.io -x -f png -e -s 2 -o diagram.drawio.png input.drawio

# Windows
"C:\Program Files\draw.io\draw.io.exe" -x -f png -e -s 2 -o diagram.drawio.png input.drawio

# Linux (headless — requires xvfb-run; on servers add HOME and --disable-gpu)
export HOME=${HOME:-/tmp}
xvfb-run -a --server-args="-screen 0 1280x1024x24" \
  drawio -x -f png -e -s 2 -o diagram.drawio.png input.drawio --disable-gpu
# Running as root (CI / Docker)? Append --no-sandbox AT THE END (placing it earlier makes drawio treat it as the input filename)

# SVG export (final — -e is safe; SVG is text)
drawio -x -f svg -e -o diagram.svg input.drawio

# PDF export (final)
drawio -x -f pdf -e -o diagram.pdf input.drawio

# Custom output directory (e.g. CI artifacts dir) — create if missing, then export there
mkdir -p ./artifacts && drawio -x -f png -e -s 2 -o ./artifacts/diagram.drawio.png input.drawio
```

### Post-export PNG repair (required after `-e` PNG export)

draw.io CLI truncates the IEND chunk when emitting `-e` PNGs — the file ends with the 4-byte IEND length field but the `IEND` type + CRC (8 bytes) are missing. Result: vision APIs return 400 "Could not process image" and strict PNG decoders error out. SVG/PDF are unaffected.

Run this immediately after every `-e` PNG export:

```bash
python3 <this-skill-dir>/scripts/repair_png.py diagram.drawio.png
```

The script's `endswith(IEND)` guard makes it a no-op once draw.io fixes the bug upstream — safe to run unconditionally.

**Key flags:**

- `-x` — export mode (required)
- `-f` — format: `png`, `svg`, `pdf`, `jpg`
- `-e` — embed diagram XML in output (PNG, SVG, PDF) — exported file remains editable in draw.io. **Skip for the preview PNG used in step 5 self-check** — `-e` PNGs have a truncated IEND chunk that vision APIs reject (issue #8). For final PNG export, keep `-e` and run `scripts/repair_png.py` (see Post-export PNG repair). SVG/PDF unaffected.
- `-s` — scale: `1`, `2`, `3` (2 recommended for final PNG; do NOT use for the step-4 preview — see `--width`)
- `--width <px>` — target width in pixels (no short form; `-w` does **not** exist and silently breaks the input-file parser). Use `--width 2000` for the step-4 preview to keep the PNG under Claude's 2576×2576 vision ceiling. There's also a `--height <px>` flag for tall-narrow diagrams. Don't combine `--width` with `-s`.
- `-o` — output file path; accepts any directory (e.g. `./artifacts/diagram.drawio.png`) — `mkdir -p` the target dir first. Use `.drawio.png` double extension when embedding.
- `--layout <preset|json>` — **CLI ≥ v30 only** — Post-generate layout pass on XML input. Accepts **only** the ELK presets `verticalFlow`, `horizontalFlow`, `verticalTree`, `horizontalTree`, `radialTree`, `organic`, or a JSON layout array — all of them re-place nodes as well as routing edges; alternative to `autolayout.py` when Graphviz is missing. **Any other value opens a modal `Unknown layout:` dialog that hangs a headless run** — there is no edge-routing-only mode on the CLI. Never combine with Mermaid-converted files (already laid out). On ≤ 29 this flag breaks argument parsing — don't emit it. See `references/mermaid-authoring.md`
- `-b` — border width around diagram (default: 0, recommend 10)
- `-t` — transparent background (PNG only)
- `--page-index <n>` — export one page of a multi-page file. **1-based** in current drawio-desktop (verified on 29.7.8: `--page-index 2` exports the second page; older docs claimed 0-based). Default: first page. `--page-range 2..3` also works

### Browser fallback (no CLI needed)

When the draw.io desktop CLI is unavailable, generate a client-side URL:

```bash
python3 <this-skill-dir>/scripts/encode_drawio_url.py input.drawio          # read-only viewer
python3 <this-skill-dir>/scripts/encode_drawio_url.py --edit input.drawio    # opens in the editor
```

Default prints a `https://viewer.diagrams.net/...#R…` viewer URL; `--edit` prints a `https://app.diagrams.net/...#create=…` URL that opens straight into the editable editor. Either way the diagram XML is `encodeURIComponent`-encoded, deflate-compressed, and base64'd into the URL fragment — the fragment (after `#`) is never sent to the server, so nothing is uploaded. The `encodeURIComponent` step is mandatory: without it, any diagram containing a literal `%` or non-ASCII (e.g. CJK) label makes the browser throw "URI malformed" and the diagram never opens.

Open the URL with `open "$URL"` (macOS) / `xdg-open "$URL"` (Linux). On **WSL2 / Windows**, `cmd.exe` drops the `#fragment` — write a `.url` shortcut file and open that instead (see `references/troubleshooting.md` → "WSL2 / Windows specifics").

### Fallback chain

When tools are unavailable, degrade gracefully:

| Scenario | Behavior |
| ---------- | ---------- |
| draw.io CLI missing, Python available | Use browser fallback (diagrams.net URL) |
| draw.io CLI missing, Python missing | Generate `.drawio` XML only; instruct user to open in draw.io desktop or diagrams.net manually |
| draw.io CLI crashes / no output in macOS sandbox isolation | Treat CLI as unavailable in-sandbox; use browser fallback / XML-only; ask user to run CLI exports in a non-sandboxed host environment |
| Vision unavailable for self-check | Skip self-check (step 5); proceed directly to showing user the exported PNG |
| Export fails (Chromium/display issues) | On Linux, retry with `xvfb-run -a`; if still failing, deliver `.drawio` XML and suggest manual export |
| Export fails on Linux server (headless) | Try in order: (1) `xvfb-run -a`, (2) append `--no-sandbox` at the very end if root, (3) add `--disable-gpu`, (4) `export HOME=/tmp`, (5) install apt deps (`libgtk-3-0 libnotify4 libnss3 libgbm1 libasound2t64` etc.), (6) fall back to [tomkludy/drawio-renderer](https://hub.docker.com/r/tomkludy/drawio-renderer) Docker (REST API for headless export) |

### Checking if drawio is in PATH

```bash
# Prefer the Homebrew / Linux-package binary name (no dot)
if command -v drawio &>/dev/null; then
  DRAWIO="drawio"
# Fall back to the dot-named binary (older installs, manual symlinks)
elif command -v draw.io &>/dev/null; then
  DRAWIO="draw.io"
# macOS .app bundle (binary inside the bundle keeps the dot)
elif [ -f "/Applications/draw.io.app/Contents/MacOS/draw.io" ]; then
  DRAWIO="/Applications/draw.io.app/Contents/MacOS/draw.io"
# WSL2: the CLI is the Windows desktop exe, reached via /mnt/c (note the space)
elif grep -qi microsoft /proc/version 2>/dev/null && [ -f "/mnt/c/Program Files/draw.io/draw.io.exe" ]; then
  DRAWIO="/mnt/c/Program Files/draw.io/draw.io.exe"
else
  echo "drawio not found — install from https://github.com/jgraph/drawio-desktop/releases (Homebrew: brew install --cask drawio)"
fi
```

On **WSL2 / native Windows**, opening exported files and browser-fallback URLs needs path conversion + a `.url`-file workaround (`cmd.exe` drops URL `#fragment`s) — see the "WSL2 / Windows specifics" section in `references/troubleshooting.md`.

## Common Mistakes

When something looks wrong (export fails, vision rejects a PNG, layout broken, edges misroute), see `references/troubleshooting.md` for a row-by-row mistake → fix table.

## Diagram Type Presets

When the user requests a specific diagram type, read `references/diagram-types.md` for the matching preset (shapes, edges, layout direction). Pick by user phrasing:

| User says | Section in `references/diagram-types.md` |
| --- | --- |
| "ER diagram", "schema diagram", "data model" | ERD |
| "UML class diagram", "class diagram" | UML Class |
| "sequence diagram", "interaction diagram", "lifeline" | Sequence |
| "architecture", "system diagram", "service diagram" | Architecture |
| "neural network", "model architecture", "ML diagram", "deep learning" | ML / Deep Learning Model |
| "flowchart", "decision tree", "process flow" | Flowchart |
| "C4", "system context diagram", "container diagram", "component diagram" | C4 Model |
| "SysML", "MBSE", "block definition diagram", "internal block diagram", "requirement diagram", "parametric diagram" | SysML |
| "BPMN", "business process", "process model", "pool and lanes", "workflow diagram" | BPMN |
| "network topology", "network diagram", "LAN/WAN", "subnet", "firewall diagram" | Network Topology |
| "swimlane diagram", "cross-functional flowchart", "who does what", "handoff diagram" | Cross-Functional Flowchart |

The diagram-type preset sets **structural** style keywords. If a user style preset is also active (see `## Style Presets`), keep the structural keywords and layer color/font/edge/extras on top — read `references/style-presets.md` → "Interaction with diagram-type presets" for the merge rules.
"################################,
        ),
    ]
}

pub(crate) fn all_static_files() -> Vec<(String, Vec<u8>)> {
    let mut files = Vec::new();
    files.push((".claude-plugin/plugin.json".to_string(), r######"{
  "name": "modernlink",
  "version": "0.1.0",
  "description": "Evidence-first repository modernization preparation backed by the deterministic ModernLink Rust CLI",
  "license": "Apache-2.0"
}
"######.as_bytes().to_vec()));
    files.push((".codex-plugin/plugin.json".to_string(), r######"{
  "name": "modernlink",
  "version": "0.1.0",
  "description": "Evidence-first repository modernization preparation backed by the deterministic ModernLink Rust CLI",
  "author": {
    "name": "Inovacc"
  },
  "license": "Apache-2.0",
  "skills": "./skills/",
  "interface": {
    "displayName": "ModernLink",
    "shortDescription": "Prepare legacy Java systems for modernization",
    "longDescription": "Collect deterministic repository evidence, question migration assumptions, and prepare coherent modernization decisions.",
    "developerName": "Inovacc",
    "category": "Developer Tools",
    "capabilities": ["Interactive", "Read", "Write"],
    "defaultPrompt": [
      "Analyze this Java repository for modernization evidence."
    ]
  }
}
"######.as_bytes().to_vec()));
    files.push((
        "config/.gitignore".to_string(),
        r######"binary-pointer.json
"######
            .as_bytes()
            .to_vec(),
    ));
    files.push((
        "config/binary-pointer.schema.json".to_string(),
        r######"{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "urn:modernlink:schema:binary-pointer:v1",
  "title": "ModernLink binary pointer",
  "type": "object",
  "additionalProperties": false,
  "required": [
    "schema_version",
    "binary_path",
    "binary_sha256",
    "os",
    "arch",
    "analysis_schema"
  ],
  "properties": {
    "schema_version": {
      "const": "modernlink.binary-pointer/v1"
    },
    "binary_path": {
      "type": "string",
      "minLength": 1
    },
    "binary_sha256": {
      "type": "string",
      "pattern": "^sha256:[0-9a-f]{64}$"
    },
    "os": {
      "type": "string",
      "minLength": 1
    },
    "arch": {
      "type": "string",
      "minLength": 1
    },
    "analysis_schema": {
      "const": "modernlink.analysis/v1alpha1"
    }
  }
}
"######
            .as_bytes()
            .to_vec(),
    ));
    files.push(("harnesses/claude.json".to_string(), r######"{
  "harness": "claude",
  "adapter_version": "v1alpha1",
  "canonical_resources": ["skills", "agents", "commands"],
  "materialization": "pending-ownership-contract",
  "safety": "Do not overwrite CLAUDE.md or user-managed skills; install only files owned by a future adapter manifest."
}
"######.as_bytes().to_vec()));
    files.push(("harnesses/codex.json".to_string(), r######"{
  "harness": "codex",
  "adapter_version": "v1alpha1",
  "canonical_resources": ["skills", "agents", "commands"],
  "materialization": "pending-ownership-contract",
  "safety": "Do not overwrite AGENTS.md or user-managed skills; install only files owned by a future adapter manifest."
}
"######.as_bytes().to_vec()));
    files.push((
        "WORKFLOWS.md".to_string(),
        r######"# ModernLink Canonical Lifecycle

This bundle is harness-neutral. Adapters may translate names and syntax, but must preserve the
evidence and approval rules below.

```text
SETUP → DISCOVER → UNDERSTAND → MODEL → ASSESS → DESIGN → PLAN → PREPARE
      → MODERNIZE → MIGRATE → VERIFY → CUTOVER → DETACH → CLEANUP
```

The Rust CLI owns facts, report schemas, and lifecycle transitions. Agents may add an
`INFERENCE` or `HYPOTHESIS`, but must cite CLI evidence and cannot promote it to a fact.
`MODERNIZE`, `CUTOVER`, and `DETACH` require an explicit human approval recorded through
`modernlink lifecycle advance --approve`.

Canonical intentions are `analyze`, `architecture`, `domains`, `assess`, `plan`, `prepare`,
`modernize`, `verify`, `migrate`, and `status`. Before an adapter exposes an intention it must
have a harness-specific ownership contract; no adapter may overwrite user-authored instructions.
"######
            .as_bytes()
            .to_vec(),
    ));
    files
}

pub(crate) fn static_files(harness: Harness) -> Vec<(String, Vec<u8>)> {
    let mut files = all_static_files();
    match harness {
        Harness::Claude => files.retain(|(path, _)| {
            path == ".claude-plugin/plugin.json"
                || path == "harnesses/claude.json"
                || path == "config/.gitignore"
                || path == "config/binary-pointer.schema.json"
                || path == "WORKFLOWS.md"
        }),
        Harness::Codex => files.retain(|(path, _)| {
            path == ".codex-plugin/plugin.json"
                || path == "harnesses/codex.json"
                || path == "config/.gitignore"
                || path == "config/binary-pointer.schema.json"
                || path == "WORKFLOWS.md"
        }),
        Harness::Gemini => files.retain(|(path, _)| {
            path == "config/.gitignore"
                || path == "config/binary-pointer.schema.json"
                || path == "WORKFLOWS.md"
        }),
    }
    files
}
