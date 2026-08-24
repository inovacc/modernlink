---
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
