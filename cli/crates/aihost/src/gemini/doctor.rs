//! gemini-extension.json + GEMINI.md presence).

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
            "run `modernlink plugin install --host gemini`",
        );
    }
    let ext = Path::new(&target).join("gemini-extension.json");
    if ext.exists() {
        add("extension_manifest", "PASS", ext.display().to_string(), "");
    } else {
        add(
            "extension_manifest",
            "FAIL",
            format!("missing {}", ext.display()),
            "reinstall via `modernlink plugin install --host gemini`",
        );
    }
    let ctx = Path::new(&target).join("GEMINI.md");
    if ctx.exists() {
        add("context_file", "PASS", ctx.display().to_string(), "");
    } else {
        add(
            "context_file",
            "WARN",
            format!("missing {}", ctx.display()),
            "reinstall to restore GEMINI.md",
        );
    }
    add(
        "cli_register",
        "WARN",
        "gemini extensions install auto-registration not wired".to_string(),
        &format!("manually run `gemini extensions install {target}` or symlink"),
    );

    let v = verdict(&checks);
    DoctorReport {
        host: h.name(),
        target,
        checks,
        verdict: v,
    }
}

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
