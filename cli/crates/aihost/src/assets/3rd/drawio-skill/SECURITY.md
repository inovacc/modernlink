# Third-party drawio-skill security review

<!-- review: 2026-08-25; source: https://github.com/Agents365-ai/drawio-skill -->

This is a source review of the upstream `drawio-skill` repository. It does not
certify the upstream project or any future revision. Only the reviewed
`SKILL.md` text is embedded in the Rust asset bundle.

## Provenance

- Source: [Agents365-ai/drawio-skill](https://github.com/Agents365-ai/drawio-skill)
- Reviewed upstream HEAD: `7a83e221714ed0e6c9be25bc500f05153518ed91`
- Embedded `SKILL.md` SHA-256: `95653730E181C7DDEF73B97F562524038BCDD9F47CF77F88154D564A5100D122`
- Upstream skill version: `2.1.1`
- Reviewed upstream inventory: 61 files; 38 Python scripts; one compressed shape-index file.

## Scope decision

Only the skill instructions are embedded at
`skills/drawio-skill/SKILL.md`. The executable scripts, compressed index,
images, tests, and reference tree are deliberately not bundled. This prevents
third-party executable code from becoming an implicit ModernLink runtime
dependency. The skill may describe those optional tools, but they are not
present or executable through `aihost`.

## Findings

### High — scripts execute external programs

The upstream scripts invoke `drawio`, `dot`/`tred`, `git`, and other tools via
Python subprocess calls. Importer scripts can read a repository, Terraform
state, Docker state, Kubernetes output, or Git history. `timelapse.py` invokes
multiple external tools and writes generated frames. These operations can
expose credentials or private source data if run against sensitive paths.

**Control:** do not ship or auto-run the scripts from `aihost`. If a future
integration adds them, require explicit operator approval, argument allowlists,
workspace-root confinement, `shell=False`, bounded output, and environment
scrubbing.

### High — optional network egress in `aiicons.py`

The skill directs `aiicons.py` to fetch SVG logos from unpkg/simple-icons CDNs.
The default output can leave remote image URLs in generated diagrams; `--embed`
performs a network fetch and stores the returned SVG as a data URI. This is an
external data-egress and supply-chain boundary, not a credential leak found in
the reviewed file.

**Control:** network is disabled by default for ModernLink skill execution.
Require an explicit network policy and permitlist the requested host before
fetching. Prefer offline, reviewed assets or `--embed` only after approval.

### Medium — untrusted input becomes generated XML/HTML or command arguments

The importers transform repository and infrastructure input into Draw.io XML,
SVG, HTML, Mermaid, and command arguments. Labels and generated markup may
contain untrusted text. Several scripts also call subprocesses with generated
paths or arguments.

**Control:** treat generated diagrams as untrusted output; escape XML/HTML,
confine all input/output paths beneath an approved workspace, reject path
traversal and special device paths, and never execute generated text.

### Medium — credential and sensitive-data exposure by requested input

The skill explicitly supports live infrastructure and repository analysis. That
can include Kubernetes secrets, Terraform state, Docker metadata, CI files,
Git remotes, and local configuration. The review found no hard-coded secret
value in the reviewed source, but the requested inputs themselves can contain
secrets.

**Control:** deny secret-bearing files by default (`.env`, credential stores,
private keys, Kubernetes Secret objects, Terraform state with secrets, and
CI secret dumps). Require an explicit redaction review before diagram output
is persisted or shared.

### Low — instruction and parameter trust

The skill contains natural-language operational instructions and management
examples that mutate user style files (delete, rename, or set defaults). Those
instructions are not executable in the embedded Rust asset, but an agent that
follows them must still confirm destructive operations and validate names.

**Control:** keep the skill advisory; require confirmation for writes/deletes,
normalize names, reject traversal, and never mutate shipped resources.

## Leakage review

- No API keys, bearer tokens, passwords, private-key material, or credential
  values were found in the reviewed source scan.
- The skill does contain external URLs and documents network fetch behavior.
- The skill can cause source, infrastructure, or Git metadata to be read when
  optional scripts are run; this is the primary practical leakage risk.
- The embedded `aihost` asset contains instructions only. It does not include
  the upstream Python scripts or binary data.

## Acceptance status

**Accepted for instruction-only embedding with restrictions.** Do not embed or
execute the upstream scripts/data bundle without a new review. Any future
change to the upstream source, skill version, network hosts, subprocess surface, or bundled
support files requires a new hash and this review to be updated.
