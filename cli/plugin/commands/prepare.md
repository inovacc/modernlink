---
description: Establish characterization and rollback evidence before modernization work begins.
argument-hint: [plan-path]
---

Run `modernlink verify --plan $1` first. It verifies only static plan integrity and declared
approval gates. Invoke the preparation-auditor role to identify the missing behavioral, contract,
data, observability, performance, security, and rollback oracles for the selected seam.

Produce a preparation checklist with owners, evidence to capture, acceptance criteria, and the
explicit approval still required. Do not claim that a plan is safe to implement merely because its
static structure passes. Do not advance `MODERNIZE`, `CUTOVER`, or `DETACH` without the required
recorded human approval.
