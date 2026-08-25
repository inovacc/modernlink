//! registration, settings.json enable flip, CLI-first global-MCP absence,
//! `claude` CLI availability).

use super::host::install_target;
use super::install::{marketplace_has_entry, mcp_servers_has_modernlink, settings_has_enabled};
use crate::host::{DoctorCheck, DoctorReport};
use crate::host_registry::look_path;
use std::path::Path;

/// Run the Claude-side health checks.
pub fn doctor() -> DoctorReport {
    let target = install_target().unwrap_or_default();
    let mut checks: Vec<DoctorCheck> = Vec::new();
    let mut add = |name: &str, verdict: &str, detail: String, fix: &str| {
        checks.push(DoctorCheck {
            name: name.to_string(),
            verdict: verdict.to_string(),
            detail,
            fix: fix.to_string(),
        });
    };

    if Path::new(&target).exists() {
        add("install_target", "PASS", target.clone(), "");
    } else {
        add(
            "install_target",
            "FAIL",
            format!("missing {target}"),
            "run `modernlink plugin install --host claude`",
        );
    }
    if marketplace_has_entry() {
        add(
            "marketplace_entry",
            "PASS",
            "marketplace.json declares modernlink".to_string(),
            "",
        );
    } else {
        add(
            "marketplace_entry",
            "FAIL",
            "marketplace.json missing modernlink entry".to_string(),
            "run `modernlink plugin install --host claude`",
        );
    }
    if settings_has_enabled() {
        add(
            "settings_enabled",
            "PASS",
            "settings.json enabledPlugins[modernlink@modernlink]=true".to_string(),
            "",
        );
    } else {
        add(
            "settings_enabled",
            "FAIL",
            "settings.json missing enabledPlugins[modernlink@modernlink]".to_string(),
            r#"add "modernlink@modernlink": true to enabledPlugins in ~/.claude/settings.json"#,
        );
    }
    // CLI-first hard rule #1: a global MCP registration is the failure mode; its
    // absence is healthy.
    match mcp_servers_has_modernlink() {
        Some(cmd) => add(
            "mcp_global",
            "WARN",
            format!(
                "global modernlink MCP server registered (command={cmd}) — CLI-first forbids this"
            ),
            "reinstall to clear the global .mcp.json entry; structured MCP is flow-scoped (*-mcp agents)",
        ),
        None => add(
            "mcp_global",
            "PASS",
            "no global modernlink MCP server (CLI-first; MCP via flow-scoped *-mcp agents)"
                .to_string(),
            "",
        ),
    }
    if look_path("claude").is_none() {
        add(
            "claude_cli",
            "WARN",
            "claude CLI not on PATH".to_string(),
            "required for `claude plugin marketplace add` during install",
        );
    } else {
        add("claude_cli", "PASS", "claude CLI on PATH".to_string(), "");
    }

    let verdict = compute_verdict(&checks);
    DoctorReport {
        host: "claude".to_string(),
        target,
        checks,
        verdict,
    }
}

/// Roll individual check verdicts up to the report verdict.
pub(crate) fn compute_verdict(checks: &[DoctorCheck]) -> String {
    let mut has_fail = false;
    let mut has_warn = false;
    for c in checks {
        match c.verdict.as_str() {
            "FAIL" => has_fail = true,
            "WARN" => has_warn = true,
            _ => {}
        }
    }
    if has_fail {
        "FAILED".to_string()
    } else if has_warn {
        "DEGRADED".to_string()
    } else {
        "OK".to_string()
    }
}
