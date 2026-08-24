---
description: Review ModernLink migration lifecycle state and prepare the next controlled transition.
argument-hint: [run-id]
---

First run `modernlink migration status --id <run-id>`. Do not recommend a lifecycle transition
when it reports any `GAP`: reconcile the immutable plan, record metadata, or journal first.

When the record is consistent, run `modernlink status --run-id <run-id>` for the current phase.
Use the phase and cited migration-plan evidence to state the next allowable action. Protected
transitions require the human to authorize `--approve`; an observed reconciliation result is not
that authorization.
