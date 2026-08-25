//! files synthesised from them (`plugin.json`, `.mcp.json`, `hooks/hooks.json`).

use crate::error::{Error, Result};
use serde::Serialize;

// Single source of truth for plugin manifest metadata.
pub const NAME: &str = "modernlink";
pub const VERSION: &str = "0.1.0";
pub const DESCRIPTION: &str = "JS module reverse-engineering enrichment via Claude Code subagent fanout (Task tool) with Postgres-backed knowledge base.";
pub const AUTHOR: &str = "Security Research";
pub const REPOSITORY: &str = "https://github.com/dyammarcano/modernlink";
pub const LICENSE: &str = "BSD-3-Clause";

/// Canonical modernlink MCP server identity: the binary on PATH invoked as
/// `modernlink mcp`. NOT registered globally (CLI-first hard rule #1).
// MCP_SERVER_NAME / MCP_ARGS are retained manifest identity consts (Rust package
// consts consumed by the CLI, which is not in this crate).
#[allow(dead_code)]
pub const MCP_SERVER_NAME: &str = "modernlink";
pub const MCP_COMMAND: &str = "modernlink";
#[allow(dead_code)]
pub const MCP_ARGS: [&str; 1] = ["mcp"];
pub const KEYWORDS: [&str; 6] = [
    "reverse-engineering",
    "enrichment",
    "javascript",
    "minified",
    "deobfuscation",
    "knowledge-base",
];

#[derive(Serialize)]
struct PluginAuthor {
    name: &'static str,
}

/// Mirrors `.claude-plugin/plugin.json`. Serde serialises struct fields in
/// declaration order (matching Rust's struct marshaling — the consts are always
/// non-empty so the `omitempty` fields always render).
#[derive(Serialize)]
struct PluginManifest {
    name: &'static str,
    version: &'static str,
    description: &'static str,
    author: PluginAuthor,
    repository: &'static str,
    license: &'static str,
    keywords: &'static [&'static str],
}

/// Pretty-print a serde value with a trailing newline (Rust `MarshalIndent` + `\n`).
pub(crate) fn marshal_indent<T: Serialize>(v: &T, ctx: &str) -> Result<Vec<u8>> {
    let s =
        serde_json::to_string_pretty(v).map_err(|e| Error::Io(format!("marshal {ctx}: {e}")))?;
    Ok(format!("{s}\n").into_bytes())
}

/// Bytes for `.claude-plugin/plugin.json`.
pub fn plugin_json() -> Result<Vec<u8>> {
    let m = PluginManifest {
        name: NAME,
        version: VERSION,
        description: DESCRIPTION,
        author: PluginAuthor { name: AUTHOR },
        repository: REPOSITORY,
        license: LICENSE,
        keywords: &KEYWORDS,
    };
    if m.name.is_empty() || m.version.is_empty() {
        return Err(Error::Io("plugin: const Name/Version empty".to_string()));
    }
    marshal_indent(&m, "plugin.json")
}

/// Bytes for the plugin's `.mcp.json`. CLI-first doctrine: an EMPTY mcpServers
/// map — structured MCP is provided only by the flow-scoped `*-mcp` subagents.
pub fn mcp_json() -> Result<Vec<u8>> {
    let doc = serde_json::json!({ "mcpServers": {} });
    marshal_indent(&doc, ".mcp.json")
}

/// path -> contents for files synthesised from consts (written alongside the
/// embedded markdown; `hooks/` and `.mcp.json` survive the reinstall sweep).
pub fn manifest_files() -> Result<Vec<(String, Vec<u8>)>> {
    Ok(vec![
        (".claude-plugin/plugin.json".to_string(), plugin_json()?),
        (".mcp.json".to_string(), mcp_json()?),
        ("hooks/hooks.json".to_string(), super::hooks::hooks_json()?),
    ])
}
