# Runtime Observation Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task.

**Goal:** Add an executable, evidence-first runtime observation subsystem to the isolated ModernLink Rust CLI/plugin domain, including safe profiles, capability probes, metadata collection, server adapters, and static/runtime correlation.

**Architecture:** A new `runtime-observer` crate owns transport-neutral contracts, safety policy, redaction, connector adapters, and deterministic snapshot/correlation models. The `modernlink-cli` crate owns command parsing, human approval boundaries, credential resolution, and process orchestration. The existing ModernLink runtime library remains untouched and has no dependency on this subsystem.

**Tech Stack:** Rust 1.96 workspace, Serde/JSON, SHA-256 canonical digests, blocking HTTP transport behind an injectable trait, `std::process::Command` for allowlisted installed-tool adapters, and local controlled HTTP/process fixtures for executable probes.

---

## Task 1: Shared runtime contracts and first executable HTTP path

**Files:**
- Create: `cli/crates/runtime-observer/Cargo.toml`
- Create: `cli/crates/runtime-observer/src/lib.rs`
- Create: `cli/crates/runtime-observer/src/model.rs`
- Create: `cli/crates/runtime-observer/src/profile.rs`
- Create: `cli/crates/runtime-observer/src/redaction.rs`
- Create: `cli/crates/runtime-observer/src/http.rs`
- Create: `cli/crates/runtime-observer/src/generic_http.rs`
- Create: `cli/crates/runtime-observer/tests/generic_http.rs`
- Modify: `cli/Cargo.toml`
- Modify: `cli/crates/modernlink-cli/Cargo.toml`
- Modify: `cli/crates/modernlink-cli/src/main.rs`
- Create: `cli/crates/modernlink-cli/src/runtime_command.rs`
- Create: `cli/crates/modernlink-cli/tests/runtime_command.rs`

**Interfaces and behavior:**
- Define `RuntimeProfile`, `ConnectorKind`, `CredentialRef`, `ObservationDepth`, `Capability`, `RuntimeEvidence`, `RuntimeObservation`, and `ObservationSnapshot` using the approved `modernlink.runtime-profile/v1` and `modernlink.runtime-observation/v1alpha1` schemas.
- Define injectable `HttpTransport` and `RuntimeConnector` traits.
- Reject inline credentials, unknown deep categories, non-loopback tunnel binds, and state-changing HTTP methods.
- Redact accepted structured projections before canonical digest computation; exclude capture time from the digest.
- Add `runtime profile validate`, `runtime probe`, and `runtime observe` commands. `observe` writes only to stdout unless `--output` is explicit.
- Implement a minimal generic HTTP metadata connector so one real local endpoint can execute end to end.

**Executable checks:**
- Start a controlled loopback HTTP fixture and invoke the built CLI with a profile; inspect the emitted observation and repeat it to compare content digests.
- Run `cargo test -p runtime-observer -p modernlink-cli` and report only the neutral machine result plus what remains unproven.

**Commit:** `feat(cli): add runtime observation core`

## Task 2: Kubernetes, SSH, credentials, and tunnel lifecycle

**Files:**
- Create: `cli/crates/runtime-observer/src/command.rs`
- Create: `cli/crates/runtime-observer/src/kubernetes.rs`
- Create: `cli/crates/runtime-observer/src/ssh.rs`
- Create: `cli/crates/runtime-observer/src/tunnel.rs`
- Create: `cli/crates/runtime-observer/tests/installed_tools.rs`
- Modify: `cli/crates/runtime-observer/src/lib.rs`
- Modify: `cli/crates/modernlink-cli/src/runtime_command.rs`
- Modify: `cli/crates/modernlink-cli/tests/runtime_command.rs`

**Interfaces and behavior:**
- Add an injectable `CommandExecutor`; production execution uses fixed argument vectors without a shell.
- Kubernetes permits only discovery/read calls and an explicitly approved loopback `port-forward`; no apply/exec/secret/configmap retrieval.
- SSH permits only a fixed metadata script, existing SSH agent/config, and optional approved local forwarding; no arbitrary remote command input.
- Resolve credential references from environment variables, external helpers, or masked prompt without serializing secret values.
- Ensure spawned tunnels are loopback-only and terminated by an RAII guard on normal and error paths.

**Executable checks:**
- Exercise argument construction and cleanup with controlled fake executables, then run `runtime probe` using any installed `kubectl`/`ssh` in discovery-only mode if present.

**Commit:** `feat(cli): add safe cluster and host observation`

## Task 3: JBoss EAP and WildFly adapter

**Files:**
- Create: `cli/crates/runtime-observer/src/jboss.rs`
- Create: `cli/crates/runtime-observer/tests/jboss.rs`
- Modify: `cli/crates/runtime-observer/src/lib.rs`
- Modify: `cli/crates/modernlink-cli/src/runtime_command.rs`

**Interfaces and behavior:**
- Negotiate the HTTP management endpoint supplied by profile; never scan ports.
- Allow only `read-attribute`, bounded `read-resource`, `read-resource-description`, `read-operation-description`, `read-children-types`, and `read-children-names` operations.
- Project server identity, operating mode, deployments, hosts/servers, and subsystem names into typed evidence; discard unexpected fields.
- Support EAP 5 only through the fixed SSH-local metadata path and label reduced confidence/capability.

**Executable checks:**
- Run the CLI against a controlled DMR fixture that returns realistic WildFly envelopes, including denial and malformed-response cases.

**Commit:** `feat(cli): observe JBoss and WildFly metadata`

## Task 4: WebLogic REST family negotiation

**Files:**
- Create: `cli/crates/runtime-observer/src/weblogic.rs`
- Create: `cli/crates/runtime-observer/tests/weblogic.rs`
- Modify: `cli/crates/runtime-observer/src/lib.rs`
- Modify: `cli/crates/modernlink-cli/src/runtime_command.rs`

**Interfaces and behavior:**
- Probe only the configured base URL using ordered families `/management/weblogic`, `/management/wls`, then `/management/tenant-monitoring`.
- Use GET only and explicit response projections for domain/server/deployment/cluster identity and health.
- Reject edit/search operations, security resources, JDBC URLs, system properties, logs, and arbitrary paths.
- Represent the optional operator-supplied Java JMX helper as an unsupported/deferred capability, never as a bundled dependency.

**Executable checks:**
- Exercise each family and fallback order against controlled loopback fixtures; invoke `runtime observe` for one fixture.

**Commit:** `feat(cli): observe WebLogic metadata`

## Task 5: GlassFish and Payara REST/asadmin adapter

**Files:**
- Create: `cli/crates/runtime-observer/src/glassfish.rs`
- Create: `cli/crates/runtime-observer/tests/glassfish.rs`
- Modify: `cli/crates/runtime-observer/src/lib.rs`
- Modify: `cli/crates/modernlink-cli/src/runtime_command.rs`

**Interfaces and behavior:**
- Prefer GET-only DAS REST under the configured `/management/domain` base.
- Fall back only to an explicitly configured installed `asadmin` and the closed command enum: `version` with local fallback disabled, `list-applications --long=false`, `list-clusters`, `list-instances`, and approved-cluster `get-health`.
- Reuse the operator's existing asadmin login cache; never put a password in argv or create a password file.
- Project application, cluster, instance, and health metadata; discard unexpected fields.

**Executable checks:**
- Exercise REST projection and fake-asadmin argument vectors/output; invoke `runtime observe` for one controlled fixture.

**Commit:** `feat(cli): observe GlassFish and Payara metadata`

## Task 6: Static/runtime correlation, deep approvals, and plugin lifecycle

**Files:**
- Create: `cli/crates/runtime-observer/src/correlation.rs`
- Create: `cli/crates/runtime-observer/tests/correlation.rs`
- Modify: `cli/crates/runtime-observer/src/lib.rs`
- Modify: `cli/crates/modernlink-cli/src/runtime_command.rs`
- Modify: `cli/crates/modernlink-cli/tests/runtime_command.rs`
- Create: `cli/plugin/skills/observe-runtime/SKILL.md`
- Create: `cli/plugin/skills/correlate-runtime/SKILL.md`
- Create: `cli/plugin/config/runtime-profile.schema.json`
- Modify: `cli/plugin/.claude-plugin/plugin.json`
- Modify: `cli/plugin/.codex-plugin/plugin.json`
- Modify: `cli/THIRD_PARTY_NOTICES.md`
- Modify: `docs/modernization/2026-08-23-runtime-observation-design.md`

**Interfaces and behavior:**
- Add `runtime correlate --analysis ... --runtime ... --output ...`; each link carries static evidence ID, runtime evidence ID, and deterministic rule ID.
- Require a separate `--approve` for each deep category: `logs`, `jvm-metrics`, `deployment-descriptors`, `sanitized-configuration`; reject all prohibited categories.
- Add plugin skills that ask coherence/readiness questions, require white-box, black-box, integration, and coverage evidence before migration recommendations, then call the binary via the pointer file.
- Record dependencies and provenance without adding release/installer work in this slice.

**Executable checks:**
- Analyze a controlled Java fixture, observe a controlled runtime fixture, correlate the two, and inspect that links cite both evidence IDs and a rule ID.
- Run the whole CLI workspace test suite and an end-to-end command transcript; state explicitly that live external clusters/servers remain unproven until a human supplies authorized targets.

**Commit:** `feat(cli): correlate runtime evidence with modernization analysis`

