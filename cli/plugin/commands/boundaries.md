---
description: List static transaction, messaging, HTTP, SOAP, and batch boundary candidates.
argument-hint: [evidence-json]
---

Run `modernlink boundaries --evidence $1 --output <temporary-boundaries-path>`. Explain that the
result is annotation-backed static evidence only: it does not establish runtime activation,
transaction resources, message destinations, or delivery guarantees. Escalate those gaps to the
appropriate specialist before a migration recommendation.
