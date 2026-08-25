//! frontmatter/body embedded from the source implementation's rendered plugin. Maintained as Rust-owned embedded assets.

use crate::assets::mk;
use crate::registry::register_asset;

pub(super) fn register() {
    register_asset(&[
        mk(
            "commands/kb/brainstorm.md",
            r#"description: KB-evidence-grounded brainstorming — fan out a fleet of KB-gathering subagents to pull cited facts from the enriched knowledge base, then run /superpowers:brainstorming over that evidence to converge on a concrete, corpus-grounded direction (with enrich as part of the loop)
argument-hint: [task=<what to brainstorm>] [apps=<slug,slug>] [depth=1|2]
allowed-tools: [Task, Bash, Read, Skill, AskUserQuestion]
"#,
            r#"# /modernlink:kb:brainstorm

Ground a design brainstorm in what the modernlink KB already knows. Given a TASK or
question, fan out a fleet of KB-gathering subagents that pull real, cited evidence
from the enriched knowledge base, then run `/superpowers:brainstorming` over
that evidence to converge on a concrete, corpus-grounded direction — not vibes.

Read the kb-brainstorm skill first (Skill tool: kb-brainstorm) — it holds the
evidence-question decomposition, the gather-fleet recipe, the support/contradict/
unknown synthesis rubric, and the enrich-in-the-loop protocol.

## Arguments

| key   | default | meaning                                                     |
|-------|---------|-------------------------------------------------------------|
| task  | (none)  | the task / question / feature to brainstorm (required)      |
| apps  | (all)   | limit evidence gathering to these app slugs (CSV)           |
| depth | 1       | 1 = focused fleet; 2 = wider fan-out + an adversarial refute pass |

## Execute (orchestration runs HERE, in the main context)

Subagents cannot spawn subagents, so YOU orchestrate and fan out via Task.

1. Load the kb-brainstorm skill AND the `superpowers:brainstorming` skill.
2. Decompose <task> into 3-6 EVIDENCE QUESTIONS the KB can answer — e.g. "which
   apps implement X", "how does <app> do Y", "what IPC channels / endpoints /
   crypto / stealth patterns exist for Z". Confirm/adjust with the user (one
   AskUserQuestion) only if the task is ambiguous.
3. Fan out the GATHER FLEET in PARALLEL (one Task call each, all in ONE message):
   - modernlink:modernlink-kb-query — one per evidence question. It translates the NL
     question into modernlink CLI (kb catalog search / query --sql / stats / show)
     and returns FACTS with module-id citations. Never let it dump raw bodies.
   - depth=2 also: modernlink:modernlink-cross-ref (relational — calls / ipc_partner /
     shares_state across modules) and modernlink:modernlink-kb-auditor (adversarial —
     is each claimed piece of evidence actually supcovered?).
4. Aggregate the cited facts. Run the brainstorming loop over them: candidate
   approaches, tradeoffs, and for each claim mark it SUPPORTED / CONTRADICTED /
   UNKNOWN by the corpus (with the citation). Rank options by evidence weight.
5. ENRICH IN THE LOOP: for every UNKNOWN the KB could not answer, name the apps +
   modules that would answer it and offer to run `/modernlink:enrich` on them,
   then re-gather those questions. (Thin KB -> enrich -> re-ask is the whole point.)
6. Emit a concrete DIRECTION: the recommended approach, the cited evidence behind
   it, the open questions, and a spec-seed ready to hand to
   `/superpowers:writing-plans`. Keep raw KB output out of the final note —
   cite module ids, summarize.

## Next

- /superpowers:writing-plans — turn the direction into a plan
- /modernlink:enrich — fill the KB gaps this brainstorm surfaced
"#,
            "2026-05-24",
        ),
        mk(
            "skills/kb-brainstorm/SKILL.md",
            r#"name: kb-brainstorm
description: |
  Methodology engine for /modernlink:kb:brainstorm. Turns a task into KB evidence
  questions, drives a parallel fleet of KB-gathering subagents (kb-query, cross-ref,
  kb-auditor) to pull cited facts from the enriched knowledge base, synthesizes them
  with a SUPPORTED/CONTRADICTED/UNKNOWN rubric into a corpus-grounded brainstorm, and
  loops enrichment in to fill the gaps it surfaces. Invoke when driving a KB-grounded
  brainstorm or when a gather subagent needs the evidence-question / citation rules.
"#,
            r#"# kb-brainstorm — KB-evidence-grounded brainstorming engine

Drives /modernlink:kb:brainstorm. Purpose: make a design conversation stand on what the
torn-down corpus ACTUALLY shows. Every claim is either cited to a KB module or marked
an explicit UNKNOWN that enrichment can rust answer. Pairs with the superpowers
brainstorming skill (this adds the evidence spine; that adds the divergence loop).

## 1. Decompose the task into evidence questions

Turn the task into 3-6 questions the KB can answer. Rustod shapes:
- existence:   "which apps do X / use library Y / set flag Z?"
- mechanism:   "how does <app> implement <feature>? which modules, what flow?"
- topology:    "what IPC channels / endpoints / gRPC methods exist for <area>?"
- comparison:  "how do <appA> and <appB> differ on <dimension>?"
- posture:     "what crypto / stealth / telemetry / auth patterns appear for <area>?"
Bad questions are ones the KB can't ground (pure opinion, future-only). Reframe those
to design options to weigh, not evidence questions.

## 2. Gather fleet (parallel, cited, no raw dumps)

Fan out one modernlink:modernlink-kb-query per evidence question. Each MUST:
- Use the CLI (run via Bash), not guesswork:
  - modernlink kb catalog search "<term>" --app <slug>     (trigram module search)
  - modernlink kb catalog query --sql "<SELECT ...>"        (structured facts / counts)
  - modernlink kb catalog stats                             (coverage — is the app even enriched?)
  - modernlink kb catalog show <kb_id>                      (identity + snapshot)
- Return FACTS + module-id citations + a confidence, NOT raw module bodies.
- If the app it needs is identity-only (0 enriched modules per stats), say so — that
  is itself a finding (route to the enrich loop, section 4).
depth=2 adds modernlink:modernlink-cross-ref (relational edges) and modernlink:modernlink-kb-auditor
(adversarial: does the cited evidence really support the claim?).

## 3. Synthesize — the SUPPORTED / CONTRADICTED / UNKNOWN rubric

For every candidate approach or claim in the brainstorm, tag it:
- SUPPORTED    — the corpus shows it; cite the module id(s).
- CONTRADICTED — the corpus shows the opposite; cite it.
- UNKNOWN      — the KB has no evidence either way (thin coverage or wrong app set).
Rank options by evidence weight (count + strength of SUPPORTED citations, minus
CONTRADICTED). Never present an UNKNOWN as if it were SUPPORTED — honesty is the value.

## 4. Enrich in the loop

Every UNKNOWN is an enrichment target. Name the apps + module areas that would answer
it; if they are identity-only or thinly enriched, offer /modernlink:enrich on them, then
re-run just those evidence questions. Thin-KB -> enrich -> re-ask is the core loop the
command exists for; do not silently accept UNKNOWNs when enrichment could resolve them.

## 5. Output — a corpus-grounded direction

Emit: the recommended approach; the cited evidence (module ids) behind it; the ranked
alternatives with their tags; the open UNKNOWNs (+ which enrich runs would close them);
and a spec-seed for /superpowers:writing-plans. Cite ids, summarize — keep raw KB
output out of context.
"#,
            "2026-05-24",
        ),
    ]);
}
