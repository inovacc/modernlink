//! frontmatter/body embedded from the source implementation's rendered plugin. Maintained as Rust-owned embedded assets.

use crate::assets::mk;
use crate::registry::register_asset;

pub(super) fn register() {
    register_asset(&[
        mk(
            "agents/modernlink-cross-ref.md",
            r#"name: modernlink-cross-ref
description: |
  Cross-reference reasoning agent. Reads N related modules together
  (sibling webpack chunks, IPC partners, dep cluster) and emits
  relational tags (calls / calledBy / ipc_partner / shares_state).
  Unlocks graph queries beyond per-module enrichment.
"#,
            r#"# modernlink-cross-ref

Relational enrichment pass. Picks groups of likely-related modules,
reads them together, persists cross-references back to KB.

## Required commands (run via Bash)

`modernlink kb catalog apps`, `modernlink kb catalog search`, `modernlink kb catalog dump`,
`modernlink kb catalog facts`, `modernlink kb enrich write-enrichment`, Task.
"#,
            "2026-05-24",
        ),
        mk(
            "commands/xref.md",
            r#"description: Cross-reference reasoning over module groups via modernlink-cross-ref subagent
argument-hint: [app=<slug>] [group=webpack_chunk|ipc|dep_cluster|role:<R>] [max_groups=N] [group_size=N] [help=0|1]
allowed-tools: [Task, Bash]
"#,
            r#"# /modernlink:xref

Cross-reference enrichment. Reads N related modules together, persists
relational tags. Delegates to `modernlink-cross-ref`.
"#,
            "2026-05-24",
        ),
    ]);
}
