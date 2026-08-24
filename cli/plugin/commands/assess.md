---
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
