//! to `~/.claude/agents/` (NOT aihost assets). Each declares the modernlink MCP
//! server INLINE so its tools exist only inside that subagent's run, never in
//! the main session.

use super::manifest::MCP_COMMAND;
use crate::error::{Error, Result};
use std::path::{Path, PathBuf};

/// Directory (relative to home) Claude Code reads subagent markdown from.
const MCP_AGENTS_DIR: &str = ".claude/agents";

/// The allowlisted flow-scoped subagents (name stem + full markdown body).
/// Kept small and deliberate — each is a flow where structured MCP access earns
/// its keep over the plain CLI.
fn mcp_scoped_agents() -> Vec<(&'static str, String)> {
    let cmd = MCP_COMMAND;
    vec![
        (
            "modernlink-enricher-mcp",
            format!(
                r#"---
name: modernlink-enricher-mcp
description: KB enrichment flow that uses structured modernlink MCP tools (flow-scoped)
tools: Read, Grep, Bash, Task, Skill
mcpServers:
  modernlink:
    type: stdio
    command: {cmd}
    args: ["mcp", "serve"]
---

# modernlink-enricher-mcp

Flow-scoped subagent for knowledge-base enrichment. The modernlink MCP server
is declared inline above — its tools are available only inside this
subagent's run, not in the main session.

Use the modernlink MCP tools to read pending modules, summarize role /
inputs / outputs / side-effects / dependencies, and persist enrichment
back into the knowledge base.
"#
            ),
        ),
        (
            "modernlink-kb-query-mcp",
            format!(
                r#"---
name: modernlink-kb-query-mcp
description: Natural-language query flow over the modernlink KB using structured MCP tools (flow-scoped)
tools: Read, Grep, Task
mcpServers:
  modernlink:
    type: stdio
    command: {cmd}
    args: ["mcp", "serve"]
---

# modernlink-kb-query-mcp

Flow-scoped subagent for answering natural-language questions over the
modernlink knowledge base. The modernlink MCP server is declared inline above —
its tools are available only inside this subagent's run, not in the main
session.

Translate the question into a sequence of modernlink MCP KB-query tool calls,
aggregate the results, and answer with citations to module ids.
"#
            ),
        ),
        (
            "modernlink-kb-drift-mcp",
            format!(
                r#"---
name: modernlink-kb-drift-mcp
description: KB drift-detection flow using structured modernlink MCP tools (flow-scoped)
tools: Read, Grep, Bash, Task
mcpServers:
  modernlink:
    type: stdio
    command: {cmd}
    args: ["mcp", "serve"]
---

# modernlink-kb-drift-mcp

Flow-scoped subagent for detecting drift between the modernlink knowledge base
and the underlying source it describes. The modernlink MCP server is declared
inline above — its tools are available only inside this subagent's run,
not in the main session.

Use the modernlink MCP drift tools to compare recorded enrichment against
current source hashes and report stale or contradicted entries.
"#
            ),
        ),
        (
            "modernlink-insights-mcp",
            format!(
                r#"---
name: modernlink-insights-mcp
description: Self-improvement rustal-tracking flow using structured modernlink MCP tools (flow-scoped)
tools: Read, Grep, Bash, Task
mcpServers:
  modernlink:
    type: stdio
    command: {cmd}
    args: ["mcp", "serve"]
---

# modernlink-insights-mcp

Flow-scoped subagent for the self-improvement insights surface that has
no CLI verb. The modernlink MCP server is declared inline above — its tools
are available only inside this subagent's run, not in the main session.

Use the modernlink MCP insights tools —
`modernlink_insights_record`, `modernlink_insights_start_rustal`, and
`modernlink_insights_complete_rustal` — to record a new insight and to
start/complete a tracked improvement rustal. The CLI's
`modernlink insights rollup/suggest/status` remain plain Bash calls for
reading the existing digest; only recording a new insight or driving a
rustal's lifecycle routes through this subagent.
"#
            ),
        ),
    ]
}

/// Write the allowlisted flow-scoped subagents to `<home>/.claude/agents/`.
/// Returns the full paths written.
pub fn write_mcp_scoped_agents(home: &str) -> Result<Vec<PathBuf>> {
    let dir = Path::new(home).join(super::from_slash(MCP_AGENTS_DIR));
    std::fs::create_dir_all(&dir)
        .map_err(|e| Error::Io(format!("mkdir {}: {e}", dir.display())))?;
    let mut paths = Vec::new();
    for (name, body) in mcp_scoped_agents() {
        let p = dir.join(format!("{name}.md"));
        std::fs::write(&p, body.as_bytes())
            .map_err(|e| Error::Io(format!("write {}: {e}", p.display())))?;
        paths.push(p);
    }
    Ok(paths)
}

/// Delete the allowlisted flow-scoped subagents, ignoring any already missing.
/// Best-effort: attempts all deletions, returns the first real error.
pub fn remove_mcp_scoped_agents(home: &str) -> Result<()> {
    let dir = Path::new(home).join(super::from_slash(MCP_AGENTS_DIR));
    let mut first_err: Option<Error> = None;
    for (name, _) in mcp_scoped_agents() {
        let p = dir.join(format!("{name}.md"));
        if let Err(e) = std::fs::remove_file(&p) {
            if e.kind() != std::io::ErrorKind::NotFound && first_err.is_none() {
                first_err = Some(Error::Io(format!("remove {}: {e}", p.display())));
            }
        }
    }
    match first_err {
        Some(e) => Err(e),
        None => Ok(()),
    }
}
