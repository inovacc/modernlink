//! (`factory` / `Register` / `All` / `ByName`).
//!
//! Rust wires hosts lazily via a factory func to avoid a circular import and
//! registers them from each host package's `init()`. Rust has no package-init
//! side effects, so [`register_host`] is called explicitly from the top-level
//! `all` barrel. Also holds the small OS-compat helpers every host needs
//! (`user_home_dir`, `look_path`) that Rust gets from its stdlib.

use crate::error::{Error, Result};
use crate::host::Host;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

/// A host factory: instantiates a fresh boxed host (Rust `type factory func() Host`).
pub type Factory = fn() -> Box<dyn Host>;

fn registry() -> &'static Mutex<Vec<Factory>> {
    static REGISTRY: OnceLock<Mutex<Vec<Factory>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(Vec::new()))
}

/// Register a host factory (Rust `Register`). Called from the `all` barrel.
pub fn register_host(f: Factory) {
    registry().lock().unwrap().push(f);
}

/// Instantiate every registered host (Rust `All`).
pub fn all_hosts() -> Vec<Box<dyn Host>> {
    registry().lock().unwrap().iter().map(|f| f()).collect()
}

/// Return the host whose `name()` matches (case-sensitive; Rust `ByName`).
pub fn host_by_name(name: &str) -> Option<Box<dyn Host>> {
    all_hosts().into_iter().find(|h| h.name() == name)
}

/// Resolve the user's home directory (Rust `os.UserHomeDir`): `%USERPROFILE%` on
/// Windows, `$HOME` elsewhere.
pub(crate) fn user_home_dir() -> Result<String> {
    let var = if cfg!(windows) { "USERPROFILE" } else { "HOME" };
    match std::env::var(var) {
        Ok(v) if !v.is_empty() => Ok(v),
        _ => Err(Error::Io(format!("resolve home: ${var} not set"))),
    }
}

/// Find an executable on `PATH` (Rust `exec.LookPath`). On Windows each `PATHEXT`
/// suffix is tried. Returns the first match.
pub(crate) fn look_path(bin: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    let exts: Vec<String> = if cfg!(windows) {
        let raw = std::env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".to_string());
        std::iter::once(String::new())
            .chain(raw.split(';').map(|e| e.to_string()))
            .collect()
    } else {
        vec![String::new()]
    };
    for dir in std::env::split_paths(&path) {
        for ext in &exts {
            let cand = dir.join(format!("{bin}{ext}"));
            if cand.is_file() {
                return Some(cand);
            }
        }
    }
    None
}

/// Outcome of a best-effort external host-CLI registration call.
pub(crate) enum CliOutcome {
    /// CLI exited 0.
    Success,
    /// CLI exited non-zero or could not be spawned.
    Failed(String),
    /// CLI ran past the deadline and was killed.
    TimedOut,
    /// CLI is not on `PATH`.
    NotFound,
}

/// Run an external host CLI (`claude` / `gemini`) best-effort and BOUNDED.
///
/// Robustness over the source implementation's plain `exec.Command`:
/// - On Windows the call is routed through `cmd /C` so launcher shims
///   (`.cmd` / `.ps1` / `.bat`) resolve — `CreateProcess` cannot exec them
///   directly (OS error 193).
/// - `stdin` is `/dev/null` and output is discarded, so an interactive CLI gets
///   EOF instead of blocking on a prompt.
/// - A 15s deadline kills a CLI that still hangs, so plugin install never stalls
///   on a slow/interactive host CLI.
pub(crate) fn run_cli(bin: &str, args: &[&str]) -> CliOutcome {
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};

    if look_path(bin).is_none() {
        return CliOutcome::NotFound;
    }
    let mut cmd = if cfg!(windows) {
        let mut c = Command::new("cmd");
        c.arg("/C").arg(bin).args(args);
        c
    } else {
        let mut c = Command::new(bin);
        c.args(args);
        c
    };
    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => return CliOutcome::Failed(e.to_string()),
    };
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => return CliOutcome::Success,
            Ok(Some(status)) => return CliOutcome::Failed(format!("exit {status}")),
            Ok(None) => {
                if Instant::now() >= deadline {
                    // Kill the whole tree: `cmd /C <shim>` spawns grandchildren
                    // (e.g. node) that `child.kill()` alone would orphan, leaving
                    // the parent shell waiting on them.
                    if cfg!(windows) {
                        let _ = Command::new("taskkill")
                            .args(["/T", "/F", "/PID", &child.id().to_string()])
                            .stdout(Stdio::null())
                            .stderr(Stdio::null())
                            .status();
                    } else {
                        let _ = child.kill();
                    }
                    let _ = child.wait();
                    return CliOutcome::TimedOut;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => return CliOutcome::Failed(e.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn look_path_and_home_are_usable() {
        // home resolves (CI always has one of the vars)
        assert!(user_home_dir().is_ok());
        // a bin that will not exist returns None
        assert!(look_path("definitely-not-a-real-binary-xyzzy").is_none());
    }
}
