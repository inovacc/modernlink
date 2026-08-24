# Features
<!-- rev:022 (RFC 3339) 2026-08-24T00:00:00Z -->

What exists in the current tree, and what is proposed. "Implemented" means the code is present;
it does **not** mean the behavior satisfies the intended runtime contract. See
[VERIFICATION.md](VERIFICATION.md) for recorded command and runtime reach; the vendor Java 6
host product remains outside every recorded run. See [ISSUES.md](ISSUES.md) I-010.

## Implemented

### HTTPS / TLS

| Feature | Where |
|---|---|
| Java 6 `HttpsURLConnection`-style facade | `java/.../ModernHttpsURLConnection.java` |
| Request methods, properties, buffered output | `java/.../LegacyHttpRequest.java` |
| Connect + read timeouts covering TCP **and** TLS handshake | `crates/http` |
| Response status, reason phrase, headers, body streams | `java/.../LegacyHttpResponse.java` |
| Indexed header access with status line at index 0 | `ModernHttpsURLConnection` |
| Typed content metadata (`getContentType`, `getContentLength`) | `ModernHttpsURLConnection` |
| Redirect policy + `maxRedirects(int)` | `LegacyHttpRequest` |
| TLS floor 1.2, selectable 1.2 / 1.3 | `crates/tls` |
| Peer certificate + cipher suite access | `LegacyTlsInfo`, `crates/core` (`modernlink-core`) |
| Capability bitmask for feature discovery | `LegacyHttpClient.getCapabilities()` |

### Messaging

| Feature | Where |
|---|---|
| JMS-shaped facade: ConnectionFactory / Connection / Session / Producer / Consumer | `java/.../messaging/` |
| Provider selection: `LEGACY_JMS`, `NATS`, `NATS_JETSTREAM`, Kafka, Pulsar, RabbitMQ | `crates/jni`, `crates/messaging` |
| Uniform transport boundary across providers | `crates/messaging` (`MessageTransportKind`) |
| Durable JetStream pull consumer with server-side ack | `NatsJetStreamTransport` |
| In-process `LEGACY_JMS` transparent transport | `InMemoryTransport` |
| Typed delivery receipts + acknowledgement modes | `ModernDeliveryReceipt`, `AcknowledgementMode` |
| Trace context as first-class envelope data (trace/span/parent/state/sampling) | `ModernTraceContext`, `crates/messaging` |
| Routing dispatch with policy, mismatch rejection, auditable receipt | `crates/messaging` |
| Read-only JMX metrics MBean, Java 6-compatible | `ModernMessagingMetricsMBean` |
| Per-provider guarantee table, queryable **before** connecting | `Provider::guarantees()`, `ModernMessagingClient.guaranteesFor(...)` |
| Fail-closed refusal of an unhonourable delivery / ack mode | `ProviderGuarantees::require_*`, `DomainError::Unsupported` |
| TEXT, BYTES and MAP payload categories across the JNI boundary | `ModernPayload`, `messaging_build_payload` |
| Provider transports behind cargo features; a provider compiled out is refused, not rerouted | `crates/messaging` `[features]`, `build_transport` |
| Panics contained at all 28 JNI entry points; a panic becomes a reported error, not UB in the JVM | `jni_guard`, `crates/jni` |
| Broker connects bounded by a deadline, overridable with `MODERNLINK_BROKER_TIMEOUT_SECS` | `block_on_with_timeout`, `crates/messaging` |
| Credentials scrubbed from every transport error before it can reach a Java exception or log | `redact_credentials`, `transport_error` |
| Native handles are registry ids, not raw pointers — a stale handle misses instead of dereferencing | `CLIENTS` / `RESPONSES`, `crates/jni` |
| A contained panic cannot leave a NATS transport permanently broken | `RestoreOnDrop`, `crates/messaging` |
| `receive()` blocking semantics declared per provider and queryable from Java 6 | `ReceiveSemantics`, `ModernReceiveSemantics` |
| Kafka refuses a TLS endpoint rather than connecting in plaintext | `endpoint_requests_tls`, `crates/messaging` |

### Utilities and packaging

| Feature | Where |
|---|---|
| `ModernUuid.v4()` / `.v7()`, `ModernBase64`, `ModernJson` | `crates/core` via JNI |
| Single JAR with per-platform native resources | `docker/java6/Dockerfile` |
| SHA-256 content-addressed native extraction with cleanup on failure | `NativeLoader.java` |
| Cross-compilation to linux-x86_64, linux-aarch64, windows-x86_64 | `cargo-zigbuild` |
| Executable cross-application contract fixtures | `hacks/` |

### Modernization CLI foundation (separate product workspace)

| Feature | Where | Scope / limitation |
|---|---|---|
| Rust-first deterministic Java repository analysis | `cli/crates/analyzer` | Static source facts include packages, imports, declarations, annotations, parsed `extends`/`implements`, and method-call candidates; dispatch and runtime call-graph resolution remain out of scope. |
| Compiled class/archive inventory | `modernlink inspect` | Reads `.class` headers plus validated `CONSTANT_Class` references from direct and JAR/WAR/EAR entries up to 16 MiB uncompressed. Recognized archive XML descriptors are streamed up to 4 MiB and expose JMS/JNDI plus declarative transaction references; evidence identifies the exact `archive!entry`, and malformed/oversized content contributes no content facts. It neither executes nor semantically decompiles bytecode. |
| Unified static + evolution evidence graph | `modernlink inspect [--history]` | Merges versioned Java and Git adapters into `modernlink.evidence/v1alpha1`; deeper detectors remain pending. |
| Architectural layer and candidate-context report | `modernlink architecture` | Inference/hypothesis states, confidence, evidence IDs, and reasoning are explicit; not a DDD fact assertion. |
| Candidate-domain report | `modernlink domains` | Emits only `CandidateContext` hypotheses plus explicit limitations for agents and reviewers; it is not a declaration of bounded contexts or business ownership. |
| Static boundary report | `modernlink boundaries` | Lists annotation-backed transaction, messaging-consumer, SOAP, HTTP, and batch candidates with source locations and limitations; it does not establish runtime activation or transaction resources. |
| Decomposed modernization seam report | `modernlink seams` | Scores observed vendor, JMS, JNDI, JDBC/persistence, and SOAP import boundaries, plus validated bytecode class references from partial-source deployments, with evidence-linked components; call/data-flow-specific seam families are pending. |
| Legacy infrastructure inventory | `modernlink inspect` | Emits deterministic import-derived signals for JMS, JNDI, JDBC/persistence, EJB, JTA, JAX-WS, JAXB, JMX, Servlet, RMI, Spring, and server APIs; imports do not prove runtime use. |
| Application-server detector | `modernlink inspect` | Detects explicit WebLogic, JBoss, WildFly, WebSphere, and Tomcat descriptors/API prefixes. Generic `web.xml`, Servlet APIs, and JBoss deployment descriptors are not misrepresented as proof of a specific newer server. |
| Declared Java build levels | `modernlink inspect` | Cites literal Maven `maven.compiler.{source,target,release}` properties and Gradle `sourceCompatibility`/`targetCompatibility` assignments. It does not resolve properties, inheritance, toolchains, plugins, or execute builds. |
| Deployment-descriptor boundary references | `modernlink inspect` | Streams recognized repository XML descriptors and bounded recognized XML entries inside JAR/WAR/EAR archives to cite JNDI/data-source, JMS queue/topic, and declarative-transaction values. Malformed XML contributes no partial content facts. It does not traverse archives nested inside an EAR/WAR. |
| Static boundary annotations | `modernlink inspect` | Emits cited, derived signals for recognized transaction, messaging-consumer, SOAP, HTTP, and scheduled-batch annotations; annotation use does not prove an active runtime entry point. |
| Target-runtime compatibility review | `modernlink compatibility --target <major>` | Emits evidence-linked review findings for observed internal-JDK, Java EE, and application-server imports; it does not claim a readiness percentage or inspect resolved dependencies/runtime behavior yet. |
| Lifecycle status | `modernlink status --run-id <id>` | Replays the setup-owned `.modernlink/state/migrations/<id>.jsonl` journal into a machine-readable snapshot; an explicit `--journal` remains available for externally managed journals. An absent default journal reports the initial state without writing. |
| Lifecycle transition | `modernlink lifecycle advance --run-id <id>` | Appends only the next valid phase to the setup-owned journal, requires `--approve` for Modernize/Cutover/Detach, and records explicit artifact digests. Run IDs are constrained so the default journal cannot escape its workspace directory. |
| Reviewable migration record | `modernlink migration create --id <id> --plan <plan.json>` | Requires a setup-owned workspace and a plan that passes static integrity checks. It writes an immutable canonical plan plus `status.json` under `modernlink/migrations/<id>/`, hashes the plan, records approval-gated task IDs, and links the mutable local journal. It does not authorize implementation or cutover. |
| Migration record reconciliation | `modernlink migration status --id <id>` | Read-only reconciliation checks the recorded plan hash, plan contract, summary metadata, lifecycle-journal link, and journal replay. It reports gaps for tampering or inconsistency; an observed result is not behavioral verification or cutover approval. |
| Evidence-linked migration DAG | `modernlink plan --seams … --compatibility …` | Turns existing seam and compatibility evidence into prerequisite tasks; it proposes no target technology or automatic cutover. |
| Static migration-plan verification | `modernlink verify --plan …` | Checks task uniqueness, dependency references, and declared migration approval gates; it explicitly does not verify behavior, tests, runtime safety, or cutover readiness. |
| Command receipt formats | `modernlink --format json\|yaml\|human <command>` | Emits JSON by default, YAML for machine consumers that prefer it, or a compact human receipt. Explicit report files remain canonical JSON, so their schema and overwrite behavior do not vary by terminal format. |
| Safe local workspace setup | `modernlink setup [repo] [--tools <ids\|all\|none>]` | Interactive terminals get a Space/Enter multi-select with detected harnesses preselected; automated callers must specify `--tools`. Setup writes a nested `.modernlink/.gitignore` for machine-local state and refuses to alter incompatible existing ignore files. |
| Harness selection management | `modernlink harness add|remove|refresh|doctor` | Changes only the ModernLink-owned workspace manifest, validates registry IDs, and reports adapter-materialization as absent rather than guessing harness paths or touching user-owned files. |
| Local prerequisite report | `modernlink doctor [repo]` | Read-only structured report for binary, repository access, Rust Git availability, workspace state, Java/Maven/Gradle launchability, and known harness markers; it intentionally does not parse tool-version text. |
| Canonical lifecycle plugin bundle | `cli/plugin/` | Includes harness-neutral analyze/architecture/domains/assess/plan/prepare/modernize/verify/migrate/status command contracts, evidence-first skills, specialist-role descriptors, and lifecycle rules. Migration, modernization, and verification workflows must reconcile an existing migration record before reasoning about phase transitions; Codex/Claude materialization remains intentionally pending ownership contracts. |
| Explicit plugin materialization | `modernlink plugin install --destination <dir>` | Embeds and writes the canonical bundle only into a new caller-selected directory; it creates no binary pointer and does not guess a harness install location. |
| Local Git evolution evidence | `cli/crates/git`, `modernlink history` | Uses structured `gix` APIs; default artifacts fingerprint commit messages instead of storing message text. |
| Path deltas, bounded co-change, and knowledge signals | `modernlink.git-history/v1alpha1` | Co-change expansion and commit traversal report caps explicitly; no contributor productivity ranking. |
| Git evolution x-ray | `modernlink evolution --history …` | Produces changed-path hotspots, co-change relationships, and contributor continuity from structured Git facts; it deliberately avoids productivity/quality rankings. |
| Repository-local Git cache | `.modernlink/cache/git/` | Cache is ignored, input-keyed, and local only. |
| `analyze --history` sibling artifact | `cli/crates/modernlink-cli` | Writes `analysis.json` and `git-history.json` without merging schemas; publication refuses to replace reviewed outputs. |
| Plugin binary pointer binding | `modernlink plugin bind`, `cli/plugin/` | Binds an explicitly materialized plugin bundle to one local binary without overwriting a pointer; harness-path installation remains pending. |
| Native npm/Bun launcher scaffold | `npm/modernlink/` | Private, non-published wrapper resolves only an exact platform package, validates a release-owned SHA-256 manifest, then hands execution to Rust; native artifact generation and npm publication remain pending. |
| Release-version anchor | `LATEST`, `scripts/release_version.mjs` | `LATEST` is the single semantic-version anchor. The tool atomically updates or verifies shipped Rust package manifests, npm wrapper/platform pins, managed plugin-skill versions, and Codex/Claude plugin manifests; CI refuses release drift. |
| npm distribution CI | `.github/workflows/npm.yml` | Validates wrapper syntax/package contents and release-version alignment on changes; an explicit dispatch reads `LATEST`, builds supported native packages, creates manifests, and publishes the `@inovacc` scope to GitHub Packages with the workflow-scoped `GITHUB_TOKEN` (`packages: write`). The workflow has not yet been dispatched against a release version. |
| Structured issue intake | `.github/ISSUE_TEMPLATE/` | Bug, feature, and documentation forms capture reproducible evidence and route sensitive reports to `SECURITY.md`; public issues are disabled outside those forms. |

## Proposed

Derived from [BACKLOG.md](BACKLOG.md); not started unless noted.

| Feature | Milestone | Note |
|---|---|---|
| Versioned envelope schema with documented per-provider mappings | M1 | partial — types exist, schema not versioned |
| JMS API compatibility matrix (every supported method + semantic) | M1 | blocked on identifying the vendor's JMS version |
| Class-loading / packaging model for Java 6 application servers | M1 | |
| Broker-backed transparent pass-through prototype | M1 | needs rollback, redelivery, dead-letter, selectors |
| Full JMX management model (health, route decisions, retries, dead letters) | M1 | metrics MBean exists; management surface does not |
| Routing + redirect policy config (patterns, tenants, predicates, dry-run) | M2 | `dry_run` field exists; behavior unverified |
| Transform envelope: serialization, schema versioning, idempotency keys, replay | M2 | |
| STREAM and OBJECT payload categories | — | **not planned by default** — see "Explicitly not planned" |
| Per-adapter guarantee declarations for TLS, auth and DLQ | M2 | ordering / persistence / ack / transactions / redelivery / replay are **done** — see [providers.md](providers.md). TLS and auth are deliberately absent until broker connections terminate through `crates/tls` |
| Migration controls: shadow publish, dual delivery, cutover, pause/resume, rollback | M2 | |
| JNDI lookup compatibility | M2 | required for true transparent mode |
| Transactions, selectors, rollback / redelivery, dead-letter | M2 | |

## Explicitly not planned

- **`OBJECT` payloads (JMS `ObjectMessage`).** Reconstructing one means deserializing
  broker-supplied bytes into Java objects, a remote-code-execution surface. A compatibility
  layer fronting a locked-down legacy application must not open that by default. Callers who
  accept the risk can use `BYTES` and deserialize explicitly.
- **`STREAM` payloads.** The frame does not encode the typed field ordering a `StreamMessage`
  exists to carry, and delivering it as opaque bytes would lose that structure silently.
- Honouring custom Java `HostnameVerifier` / `SSLSocketFactory` — rejected by design (I-008).
- A Java-side JSON object model — would put a modern dependency on the legacy class path (I-007).
- Registering a global URL handler — the HTTPS adapter is constructed explicitly.
