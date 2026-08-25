//! + manifest presence; marketplace/enable checks land when the CLI stabilises).

use super::host::{Host, install_target};
use crate::host::{DoctorCheck, DoctorReport, Host as HostTrait};
use std::path::Path;

pub fn doctor(h: &Host) -> DoctorReport {
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
            "run `modernlink plugin install --host codex`",
        );
    }
    let plugin_json = Path::new(&target).join(".codex-plugin").join("plugin.json");
    if plugin_json.exists() {
        add(
            "plugin_manifest",
            "PASS",
            plugin_json.display().to_string(),
            "",
        );
    } else {
        add(
            "plugin_manifest",
            "FAIL",
            format!("missing {}", plugin_json.display()),
            "reinstall via `modernlink plugin install --host codex`",
        );
    }
    let mcp_json = Path::new(&target).join(".mcp.json");
    if mcp_json.exists() {
        add("mcp_manifest", "PASS", mcp_json.display().to_string(), "");
    } else {
        add(
            "mcp_manifest",
            "FAIL",
            format!("missing {}", mcp_json.display()),
            "reinstall via `modernlink plugin install --host codex`",
        );
    }
    add(
        "marketplace_register",
        "WARN",
        "codex CLI auto-registration not wired".to_string(),
        "manually add to ~/.agents/plugins/marketplace.json",
    );

    let v = verdict(&checks);
    DoctorReport {
        host: h.name(),
        target,
        checks,
        verdict: v,
    }
}

/// Roll individual check verdicts up to the report verdict (Rust per-host `verdict`).
fn verdict(checks: &[DoctorCheck]) -> String {
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
