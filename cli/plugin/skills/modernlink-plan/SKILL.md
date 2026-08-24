---
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
