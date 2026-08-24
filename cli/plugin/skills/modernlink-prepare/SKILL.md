---
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
