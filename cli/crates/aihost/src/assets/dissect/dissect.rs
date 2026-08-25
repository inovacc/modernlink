//! frontmatter/body embedded from the source implementation's rendered plugin. Maintained as Rust-owned embedded assets.

use crate::assets::mk;
use crate::registry::register_asset;

pub(super) fn register() {
    register_asset(&[
        mk(
            "agents/modernlink-dissector.md",
            r#"name: modernlink-dissector
description: |
  File-system dissector for modernlink. Spawned by /modernlink:dissect,
  auto-detects the bundle type (ASAR, MSIX, IPA, APK, .node, WASM) and
  extracts all contained files into a structured output directory.
  Populates the modernlink Postgres KB if an app slug is provided.
"#,
            r#"# modernlink-dissector

Orchestrator for multi-format extraction. You delegate the heavy
lifting to format-specific modernlink CLI commands via Bash.

## Required commands (run via Bash)

- `modernlink app detect <path>` — identify the bundle type
- `modernlink app dissect <path> --output <dir>` — extract all contents
- `modernlink asar extract`, `modernlink msix extract`, `modernlink android extract`,
  `modernlink ios extract`, `modernlink nodeaddon info`, `modernlink wasm info` for format-specific work
- Use `--json` / `--output <dir>` and summarise; never paste raw dumps.

Tools: Bash, Read, Write.
"#,
            "2026-05-24",
        ),
        mk(
            "commands/dissect.md",
            r#"description: Auto-detect bundle type and extract all contents via modernlink-dissector subagent
argument-hint: [path=<file>] [out=<dir>] [app=<slug>] [help=0|1]
allowed-tools: [Task, Bash, Read, Write]
"#,
            r#"# /modernlink:dissect

Auto-detect bundle type and extract all contents. Delegates to the
`modernlink-dissector` subagent.

## Arguments

| key  | default | meaning                                              |
|------|---------|------------------------------------------------------|
| path | (none)  | absolute path to bundle (required)                   |
| out  | (none)  | extraction target dir (required)                     |
| app  | (none)  | KB app slug to populate during dissect               |
| help | 0       | print this table and exit                            |

## Execute

1. Validate args.
2. Dispatch one Task call with `subagent_type="modernlink:modernlink-dissector"` and the parsed args in the prompt.
3. Stream the subagent's report verbatim.
"#,
            "2026-05-24",
        ),
    ]);
}
