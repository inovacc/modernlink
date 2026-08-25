//! `hooks/hooks.json` (a manifest file, not an aihost asset).

use super::manifest::{MCP_COMMAND, marshal_indent};
use crate::error::Result;
use serde::Serialize;
use std::collections::BTreeMap;

/// One hook entry — a direct modernlink-binary command.
#[derive(Serialize)]
struct HookInvocation {
    #[serde(rename = "type")]
    typ: &'static str,
    command: String,
}

/// An optional tool matcher bound to a list of invocations. Lifecycle events
/// omit `matcher`; tool events set it (Rust `omitempty`).
#[derive(Serialize)]
struct HookGroup {
    #[serde(skip_serializing_if = "str::is_empty")]
    matcher: &'static str,
    hooks: Vec<HookInvocation>,
}

#[derive(Serialize)]
struct HooksDoc {
    hooks: BTreeMap<&'static str, Vec<HookGroup>>,
}

/// The direct-binary command string for a hook (`<bin> hook <name>`).
fn hook_command(name: &str) -> String {
    format!("{MCP_COMMAND} hook {name}")
}

fn cmd_group(name: &str) -> HookGroup {
    HookGroup {
        matcher: "",
        hooks: vec![HookInvocation {
            typ: "command",
            command: hook_command(name),
        }],
    }
}

fn match_group(matcher: &'static str, name: &str) -> HookGroup {
    HookGroup {
        matcher,
        hooks: vec![HookInvocation {
            typ: "command",
            command: hook_command(name),
        }],
    }
}

/// The hooks modernlink ships — small and high-value (infrequent lifecycle events).
/// A `BTreeMap` reproduces Rust's sorted-map JSON key order.
fn hooks_spec() -> BTreeMap<&'static str, Vec<HookGroup>> {
    let mut m: BTreeMap<&'static str, Vec<HookGroup>> = BTreeMap::new();
    m.insert("SessionStart", vec![cmd_group("resume")]);
    m.insert("Stop", vec![cmd_group("heal")]);
    m.insert(
        "PostToolUse",
        vec![match_group(
            "mcp__modernlink__modernlink_app_dissect",
            "kb-capture",
        )],
    );
    m
}

/// Bytes for `hooks/hooks.json` (pretty-printed, trailing newline).
pub fn hooks_json() -> Result<Vec<u8>> {
    marshal_indent(
        &HooksDoc {
            hooks: hooks_spec(),
        },
        "hooks.json",
    )
}

/// The hook handler names referenced by the manifest (last field of each
/// command), de-duplicated in first-seen order.
// Retained public API (Rust `claude.HookNames`); the CLI's hook dispatcher — the
// consumer — is not in this crate.
#[allow(dead_code)]
pub fn hook_names() -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut names = Vec::new();
    for groups in hooks_spec().values() {
        for g in groups {
            for h in &g.hooks {
                if let Some(name) = h.command.split_whitespace().next_back() {
                    if seen.insert(name.to_string()) {
                        names.push(name.to_string());
                    }
                }
            }
        }
    }
    names
}
