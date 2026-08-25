//! Implementation of `pkg/aihost/claude` — the modernlink plugin host for Anthropic's Claude
//! Code. Asset bodies live in the shared `crate::assets` registry; this host
//! renders them and adds the synthesised manifest files (plugin.json / .mcp.json
//! / hooks.json) and the flow-scoped MCP subagents.
//!
//! Rust registers the host from `init()`; Rust registers it explicitly via
//! [`register`], called from the top-level `all` barrel.

mod assets;
mod doctor;
mod hooks;
mod host;
mod install;
mod manifest;
mod mcpagents;

pub use host::Host;
// Identity consts reused by the codex/gemini hosts (Rust: `claude.Version` etc.).
pub(crate) use manifest::{DESCRIPTION, MCP_COMMAND, VERSION};

// Shared slash->OS path helper, reached as `super::from_slash` by submodules.
use crate::write_tree::from_slash;

fn factory() -> Box<dyn crate::host::Host> {
    Box::new(Host)
}

/// Register the Claude host into the global host registry.
pub(crate) fn register() {
    crate::host_registry::register_host(factory);
}
