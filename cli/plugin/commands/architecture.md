---
description: Infer labeled architectural layers and candidate contexts from ModernLink evidence.
argument-hint: [evidence-json]
---

Run `modernlink architecture --evidence $1 --output <temporary-architecture-path>`. Treat every
layer as an inference and every candidate context as a hypothesis. Present confidence, evidence
IDs, locations, and unknowns; never rename a candidate context into a confirmed business boundary
without user confirmation.
