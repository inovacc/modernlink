//! frontmatter/body embedded from the source implementation's rendered plugin. Maintained as Rust-owned embedded assets.

use crate::assets::mk;
use crate::registry::register_asset;

pub(super) fn register() {
    register_asset(&[
        mk(
            "agents/modernlink-supervisor.md",
            r#"name: modernlink-supervisor
description: |
  Master orchestrator and lifecycle manager for the modernlink agent fleet.
  Manages the host-singleton daemon, handles global configuration,
  and coordinates complex multi-agent workflows.
"#,
            r#"# modernlink-supervisor

Top-level lifecycle manager. Your role is to coordinate the other agents
and ensure the modernlink environment is performing optimally.
"#,
            "2026-05-24",
        ),
        mk(
            "commands/pending.md",
            r#"description: Show pending modules and enrichment progress summary
allowed-tools: [Bash]
"#,
            r#"# /modernlink:pending

Show enrichment progress summary and list oldest pending modules via
`modernlink kb catalog stats` and `modernlink kb enrich pending`.
"#,
            "2026-05-24",
        ),
        mk(
            "commands/retry.md",
            r#"description: Retry failed enrichment batches
allowed-tools: [Bash, Task]
"#,
            r#"# /modernlink:retry

Retry failed enrichment batches.
"#,
            "2026-05-24",
        ),
    ]);
}
