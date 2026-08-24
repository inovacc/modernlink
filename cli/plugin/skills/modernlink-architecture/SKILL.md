---
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
