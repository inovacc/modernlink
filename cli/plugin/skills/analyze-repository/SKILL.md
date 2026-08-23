---
name: modernlink-analyze-repository
description: This skill should be used when the user asks to "analyze a legacy Java repository", "discover architecture boundaries", "build a modernization evidence graph", or "find where to modernize a Java system" with ModernLink.
metadata:
  version: 0.1.0
---

# Analyze a Repository with ModernLink

Use the installed ModernLink Rust binary to collect deterministic repository evidence before
making architectural or modernization claims. Keep the runtime compatibility library and this
preparation ecosystem as separate domains.

## Required procedure

1. Locate `../../config/binary-pointer.json` relative to this skill file.
2. Stop with a setup error when the pointer is absent, its `schema_version` is not
   `modernlink.binary-pointer/v1`, or its `analysis_schema` is not
   `modernlink.analysis/v1alpha1`.
3. Resolve `binary_path` from the pointer. Refuse to guess another installation path.
4. Ask for the repository root when it is not explicit. Confirm exclusions, generated or vendored
   trees, desired modernization outcome, and whether analysis may write `.modernlink/` artifacts.
5. Create the output directory only after the scope is coherent.
6. Execute the binary directly:

   ```text
   <binary_path> analyze <repository-root> --output <repository-root>/.modernlink/evidence/analysis.json
   ```

7. Read the emitted summary and evidence graph. Treat `artifacts`, `evidence`, `nodes`, and
   `edges` as observed syntax evidence only.
8. Surface every non-`complete` `parse_health` value before drawing conclusions.
9. Label architectural boundaries, domain groupings, and modernization seams as hypotheses until
   deterministic rules or a human review promote them.
10. Ask follow-up questions whenever repository evidence conflicts with declared architecture,
    deployment reality, ownership, or migration intent. Continue until material ambiguity is
    resolved, explicitly deferred with an owner, or recorded as a blocker.

## Initial evidence scope

Expect Java artifacts, package and declared-type nodes, package containment, import relationships,
source byte spans, content digests, parse health, stable content-derived IDs, and deterministic JSON
ordering. Do not imply that imports alone establish runtime calls, domain ownership, bounded
contexts, or safe migration seams.

## Migration-readiness questions

Before recommending component migration, collect answers for:

- the exact component boundary and production responsibility;
- baseline build and test commands at a pinned revision;
- component-scoped line, branch, function, and mutation coverage policy;
- white-box tests for internal decisions and failure paths;
- black-box tests for public contracts and compatibility behavior;
- integration tests for databases, messaging, application servers, security, and external systems;
- production inputs, outputs, side effects, timing, ordering, error, and rollback oracles;
- missing tests, accepted risks, owners, expiry dates, and parity approvers.

Never convert absent evidence into readiness. Record the gap and ask the next coherence question.

## Safety and reporting

- Never load, build, or modify the ModernLink runtime library to perform repository preparation.
- Never execute an unbound binary or silently update the pointer.
- Never expose credentials, payloads, or proprietary source text in summaries.
- Report machine results as observations. Leave correctness and migration approval to the human.
