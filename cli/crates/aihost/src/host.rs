//! Host contract + optional capabilities.
//!
//! TRAIT-VS-ENUM: `Host` is an OPEN set — concrete hosts (Claude Code, Codex,
//! Gemini) live in packages OUTSIDE this one, and the registry discovers optional
//! capabilities via Rust type assertion (`h.(Installer)`). A closed enum cannot
//! name an out-of-crate host, so `Host` is a **trait** (`Box<dyn Host>`), and
//! the optional capabilities are **separate traits** reached through
//! default-`None` downcast accessors — the Rust analogue of Rust's type
//! assertion (mirrors the w58/w63 type-switch handling).
//!
//! The capability traits keep host adapters small and explicit.

use crate::write_tree::TreeWriter;
use serde::{Deserialize, Serialize};

/// Minimum surface a plugin host implementation must expose so the CLI can
/// drive install/uninstall uniformly.
pub trait Host: TreeWriter {
    /// Short host identifier ("claude", "gemini", ...).
    fn name(&self) -> String;

    /// Absolute filesystem path where the rendered plugin tree is written.
    fn install_target(&self) -> crate::error::Result<String>;

    /// Optional [`Installer`] capability (Rust `h.(Installer)`), `None` unless
    /// the concrete host overrides it.
    fn as_installer(&self) -> Option<&dyn Installer> {
        None
    }

    /// Optional [`Status`] capability (Rust `h.(Status)`).
    fn as_status(&self) -> Option<&dyn Status> {
        None
    }

    /// Optional [`Doctor`] capability (Rust `h.(Doctor)`).
    fn as_doctor(&self) -> Option<&dyn Doctor> {
        None
    }
}

/// Optional capability for hosts that wire themselves into their CLI's
/// marketplace / settings layer.
pub trait Installer {
    /// Write plugin files to `target` and patch host-side state. Returns the
    /// file count written.
    fn install(&self, target: &str) -> crate::error::Result<i32>;
    /// Remove plugin files at `target` and undo patches.
    fn uninstall(&self, target: &str) -> crate::error::Result<()>;
}

/// Optional capability for hosts that report install health checks.
pub trait Status {
    /// Print a human-readable status report to `w`.
    fn print_status(&self, w: &mut dyn std::io::Write) -> std::io::Result<()>;
}

/// One host-side health check result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoctorCheck {
    /// Check name.
    pub name: String,
    /// PASS | WARN | FAIL.
    pub verdict: String,
    /// Detail (omitted when empty).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub detail: String,
    /// Suggested fix (omitted when empty).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub fix: String,
}

/// Structured output of a host's self-diagnosis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoctorReport {
    /// Host identifier.
    pub host: String,
    /// Install target path.
    pub target: String,
    /// Per-check results.
    pub checks: Vec<DoctorCheck>,
    /// OK | DEGRADED | FAILED.
    pub verdict: String,
}

/// Optional capability that returns host-side health findings.
pub trait Doctor {
    /// Run the host self-diagnosis.
    fn doctor(&self) -> DoctorReport;
}
