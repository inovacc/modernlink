//! frontmatter/body embedded from the source implementation's rendered plugin. Maintained as Rust-owned embedded assets.

use crate::assets::mk;
use crate::registry::register_asset;

pub(super) fn register() {
    register_asset(&[
        mk(
            "agents/modernlink-enricher-poly.md",
            r#"name: modernlink-enricher-poly
description: |
  Non-JS module enricher for modernlink. Reads decompiled or disassembled
  bodies for Java, .NET, Kotlin, Android smali/dex, native addons (.node),
  and WASM, and emits the SAME structured summary contract as
  modernlink-enricher: role, inputs, outputs, side-effects, dependencies.
  Routed to by modernlink-kb-builder for any pending module whose language is
  not JavaScript/TypeScript.
"#,
            r#"# modernlink-enricher-poly

Reverse-engineering specialist for NON-JavaScript bodies. Your role is to read
decompiled/disassembled source (Java, C#/.NET, Kotlin, smali, native, WASM) and
describe its purpose and behavior in natural language — identical output
contract to `modernlink-enricher`, different input languages.

## Required commands (run via Bash)

Read pending + write back (same data plane as modernlink-enricher):
`modernlink kb enrich pending`, `modernlink kb enrich write-enrichment`.

Pull body context by language (do NOT call any model yourself — these are I/O):
- Java:    `modernlink java decompile`
- .NET:    `modernlink dotnet decompile`
- Android: `modernlink android static smali`, `kotlin`, `dex`, `native`
- Native:  `modernlink nodeaddon symbols`
- WASM:    `modernlink wasm info`

## Contract (identical to modernlink-enricher)

For each pending module you enrich, write via `modernlink kb enrich write-enrichment`:
- role: one-line purpose
- inputs / outputs: what it consumes and produces
- side_effects: filesystem, network, IPC, process, registry
- deps: modules/libraries it calls

Never invent behavior the body does not show. If a body is empty or
unrecoverable, mark it needs_human_verification rather than guessing.
"#,
            "2026-05-24",
        ),
        mk(
            "agents/modernlink-enricher.md",
            r#"name: modernlink-enricher
description: |
  Low-level JS module enricher for modernlink. Reads one or more
  minified/obfuscated JavaScript module bodies and emits structured
  summaries: role, inputs, outputs, side-effects, and dependencies.
  Usually fanned out as Task subagents by modernlink-kb-builder.
"#,
            r#"# modernlink-enricher

Reverse-engineering specialist. Your role is to look at minified or
obfuscated JavaScript source code and describe its purpose and
behavior in natural language.

## Required commands (run via Bash)

`modernlink kb enrich pending`, `modernlink kb enrich write-enrichment`,
`modernlink jsdeob deobfuscate`.
"#,
            "2026-05-24",
        ),
        mk(
            "commands/enrich.md",
            r#"description: Summarise and classify minified modules in the KB via modernlink-enricher subagents
argument-hint: [app=<name>] [limit=N] [batch_size=N] [re_audit=0|1] [help=0|1]
allowed-tools: [Bash, Task]
"#,
            r#"# /modernlink:enrich

Summarise and classify pending modules in the KB. Delegates the work
to `modernlink-enricher` subagents.

## Arguments

| key      | default | meaning                                              |
|----------|---------|------------------------------------------------------|
| app      | (none)  | KB app slug (required)                               |
| limit    | 100     | max modules to enrich                                |
| re_audit | 0       | if 1, prioritize modules with open contradictions     |
| help     | 0       | print this table and exit                            |

## Workflow

1. If `re_audit=1`, query `modernlink kb findings list` for open `contradict` findings for the app.
2. Batch modules: first modules with open contradictions, then modules with `summary IS NULL`.
3. For each batch, fan out `modernlink-enricher` subagents.
4. Record results.
"#,
            "2026-05-24",
        ),
    ]);
}
