---
description: Collect deterministic ModernLink repository and Git evidence before architectural reasoning.
argument-hint: [repository]
---

Run `modernlink inspect $1 --history --output <temporary-evidence-path>`. Report parse health,
collection scope, and evidence counts. Then invoke the repository-archaeologist role; do not infer
domains or migration priority before the evidence is available.
