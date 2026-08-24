# Runtime Observation Implementation Checkpoint
<!-- rev:001 (RFC 3339) 2026-08-24T00:00:06Z -->

## Stop boundary

Implementation stopped after Tasks 1 and 2 of `plans/2026-08-23-runtime-observation-plan.md`. The shared runtime-observation core and the Kubernetes/SSH installed-tool boundary are committed. JBoss/WildFly, WebLogic, GlassFish/Payara, static/runtime correlation, deep-mode approvals, and the runtime plugin skills have not started.

All implementation remains under `cli/`. The root ModernLink runtime library was not changed and has no dependency on the CLI/plugin preparation ecosystem.

## Implemented surface

### Shared observation core

- `runtime-observer` Rust crate in the isolated CLI workspace.
- Strict `modernlink.runtime-profile/v1` profiles with connector-specific target validation and unknown-field rejection.
- `modernlink.runtime-observation/v1alpha1` evidence/snapshot model.
- GET-only bounded HTTP transport with timeouts, redirect rejection, content-type checks, and response-size limits.
- One executable generic loopback HTTP path for profile validation, capability probe, and metadata observation.
- Central structured redaction before evidence/payload/content digests.
- One shared sensitive-key classifier across profile URLs, evidence references, and structured payloads.
- Content digest excludes capture time and incorporates only the accepted redacted projection.
- Observation is written to stdout unless `--output` is explicitly supplied.
- Runtime command diagnostics distinguish input, network, output, and internal failures.

### Kubernetes and SSH preparation connectors

- Fixed-argv process execution without caller-provided shell command strings.
- Exact connector-specific Kubernetes context and SSH destination targets; no lossy URL-host derivation.
- Profile validation repeated at every public execution/tunnel-plan boundary.
- Kubernetes closed read-only resource allowlist; no apply, exec, Secret, or ConfigMap retrieval.
- SSH fixed reduced metadata script; no arbitrary remote commands or process arguments.
- Typed, allowlisted, redacted evidence parsing for Kubernetes and SSH output.
- Overall command deadlines, bounded concurrent stdout/stderr capture, and sanitized nonzero-exit diagnostics.
- Cross-platform process groups/Windows Job Objects, including descendant-held-pipe termination.
- External credential helpers are lazy and require an approval matching the configured executable; credential values are non-serializable and redact their debug representation.
- Opaque connector-built tunnel plans encode loopback binding and prevent raw argv launch.
- Tunnel startup rejects immediate exits and cleanup owns the whole process group/job, including wrappers whose leader exits before the real forwarding descendant.

## Commits in this checkpoint

- `560ccdd` — `feat(cli): add runtime observation core`
- `9d93b29` — `fix(cli): enforce runtime observation redaction`
- `93c95dd` — `fix(cli): reject runtime credential key variants`
- `e47d5d2` — `fix(cli): unify runtime sensitive key redaction`
- `3f89558` — `fix(cli): redact runtime credential suffixes`
- `0195027` — `feat(cli): add safe cluster and host observation`
- `2d91c00` — `fix(cli): harden runtime observation boundaries`
- `fdcec8c` — `fix(cli): bound runtime tool process groups`
- `e397191` — `fix(cli): own tunnel process groups`
- `6db52ef` — `fix(cli): terminate exited tunnel groups`

Task-scoped independent reviews found no remaining Critical, Important, or Minor findings for Task 1 and no remaining Critical or Important findings for Task 2 at this stop boundary.

## Machine facts and unproven behavior

The implementer reports that focused `runtime-observer`/`modernlink-cli` tests, formatting checks, doc compile-fail checks, and strict Clippy invocations exited with code 0. Controlled fixtures exercised:

- a loopback HTTP probe plus two observations with matching redacted content digests;
- fixed local executable argument capture;
- timeout and bounded stdout/stderr behavior;
- descendant-held output handles;
- immediate and long-lived tunnel wrapper cleanup;
- typed Kubernetes/SSH parsers; and
- credential-helper approval and non-serialization.

These machine results do not establish correctness. No real Kubernetes cluster, SSH host, JBoss/WildFly, WebLogic, GlassFish, or Payara target was contacted. Real authentication, TLS/proxy behavior, RBAC/roles, legacy server variants, partial failures, and production network behavior remain unproven and require human-authorized targets.

## Resume point

Resume with Task 3 in `plans/2026-08-23-runtime-observation-plan.md`:

1. Implement the JBoss EAP 6+/WildFly GET/typed-read DMR adapter against controlled realistic fixtures.
2. Add the reduced-capability EAP 5 SSH-local metadata path.
3. Run one human-authorized JBoss/WildFly metadata observation before widening the adapter.
4. Continue to WebLogic REST-family negotiation, then GlassFish/Payara REST/asadmin.
5. Finish with static/runtime correlation, per-category deep approvals, and plugin skills/readiness questions.

Do not start installer/release packaging until the application-server observation and correlation paths have executed end to end. The npm/bun launcher and release-binary downloader remain later work.

