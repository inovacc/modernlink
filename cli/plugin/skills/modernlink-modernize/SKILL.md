---
name: modernlink-modernize
description: This skill should be used when the user asks to modernize one approved ModernLink seam or implement a scoped migration task.
metadata:
  version: 0.1.0
---

# Modernize One Approved Seam

Work only on an identified migration task. First reconcile its migration record with
`modernlink migration status --id <id>`, then read its seam evidence, compatibility findings,
plan prerequisites, current lifecycle state, contract tests, and rollback condition. Stop when
the record reports a gap or the task lacks explicit approval, a behavior oracle, or a bounded
file/component scope.

Do not perform unrelated cleanup. Preserve ordering, acknowledgement, transaction, security,
and error semantics unless the approved migration specification explicitly changes them. Record
new evidence and advance lifecycle state only through the ModernLink CLI.
