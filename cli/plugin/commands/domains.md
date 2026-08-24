---
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
