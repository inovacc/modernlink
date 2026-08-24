---
name: modernlink-domains
description: This skill should be used when the user asks to discover domain vocabulary, candidate bounded contexts, ownership hypotheses, or business boundaries in a legacy repository.
metadata:
  version: 0.1.0
---

# Discover Candidate Domains

Start with a current `modernlink.evidence/v1alpha1` graph and `modernlink architecture`. Cluster
only cited structural evidence: package/module cohesion, names, data references, endpoints,
message destinations, transaction candidates, and bounded Git co-change.

Return `CandidateBoundedContext` rather than `BoundedContext`. Every proposal must include:

- confidence and epistemic state;
- supporting and contradicting evidence IDs;
- source locations and vocabulary;
- an operator question needed for confirmation.

Never infer people, team ownership, business criticality, or a DDD model as a fact from source
layout or commit history alone.
