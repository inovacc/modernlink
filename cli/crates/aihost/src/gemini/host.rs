//! Layout: `gemini-extension.json` at the extension root (mcpServers inline) +
//! `GEMINI.md` context + `skills/<name>/SKILL.md`; installs to
//! `~/.gemini/extensions/<name>/`.

use crate::assets::{self, bundle::Harness};
use crate::error::{Error, Result};
use crate::host::{Doctor, DoctorReport, Host as HostTrait, Installer, Status};
use crate::host_registry::{run_cli, user_home_dir};
use crate::write_tree::{TreeWriter, write_tree_atomic};
use crate::{TemplateData, all_assets};
use std::io::Write;
use std::path::Path;

pub const NAME: &str = crate::PLUGIN_NAME;
pub const VERSION: &str = crate::PLUGIN_VERSION;
pub const DESCRIPTION: &str = crate::PLUGIN_DESCRIPTION;
pub const MCP_COMMAND: &str = crate::PLUGIN_MCP_COMMAND;

/// The Gemini CLI host.
#[derive(Debug, Default, Clone, Copy)]
pub struct Host;

fn template_data() -> TemplateData {
    TemplateData {
        name: NAME.to_string(),
        version: VERSION.to_string(),
        description: DESCRIPTION.to_string(),
        mcp_command: MCP_COMMAND.to_string(),
        created: crate::current_date(),
    }
}

pub(crate) fn install_target() -> Result<String> {
    Ok(Path::new(&user_home_dir()?)
        .join(".gemini")
        .join("extensions")
        .join(NAME)
        .to_string_lossy()
        .into_owned())
}

/// Shell out `gemini extensions install <target>` so the CLI indexes our
/// extension. Best-effort with a manual hint on failure.
fn register_via_cli(target: &str) {
    use crate::host_registry::CliOutcome;
    match run_cli("gemini", &["extensions", "install", target]) {
        CliOutcome::Success => eprintln!("[gemini] registered via gemini extensions install"),
        CliOutcome::NotFound => eprintln!(
            "[gemini] gemini CLI not on PATH — run manually:\n  gemini extensions install {target:?}"
        ),
        CliOutcome::TimedOut => eprintln!(
            "[gemini] extensions install timed out — run manually:\n  gemini extensions install {target:?}"
        ),
        CliOutcome::Failed(msg) => eprintln!("[gemini] extensions install said: {msg}"),
    }
}

impl TreeWriter for Host {
    fn walk(&self, f: &mut dyn FnMut(&str, &[u8]) -> Result<()>) -> Result<()> {
        let td = template_data();
        for asset in all_assets() {
            f(&asset.path, &asset.render(td.clone())?)?;
        }
        Ok(())
    }

    fn manifest_files(&self) -> Result<Vec<(String, Vec<u8>)>> {
        Ok(assets::static_files(Harness::Gemini))
    }
}

impl HostTrait for Host {
    fn name(&self) -> String {
        "gemini".to_string()
    }
    fn install_target(&self) -> Result<String> {
        install_target()
    }
    fn as_installer(&self) -> Option<&dyn Installer> {
        Some(self)
    }
    fn as_status(&self) -> Option<&dyn Status> {
        Some(self)
    }
    fn as_doctor(&self) -> Option<&dyn Doctor> {
        Some(self)
    }
}

impl Installer for Host {
    fn install(&self, target: &str) -> Result<i32> {
        if target.is_empty() {
            return Err(Error::Io(
                "gemini install: target must not be empty".to_string(),
            ));
        }
        let n = write_tree_atomic(self, target, &[])
            .map_err(|e| Error::Io(format!("gemini install: {e}")))?;
        register_via_cli(target);
        Ok(n)
    }

    fn uninstall(&self, target: &str) -> Result<()> {
        if target.is_empty() {
            return Err(Error::Io(
                "gemini uninstall: target must not be empty".to_string(),
            ));
        }
        std::fs::remove_dir_all(target).map_err(|e| Error::Io(format!("gemini rm {target}: {e}")))
    }
}

impl Status for Host {
    fn print_status(&self, w: &mut dyn Write) -> std::io::Result<()> {
        let target = install_target().map_err(|e| std::io::Error::other(e.to_string()))?;
        let exists = Path::new(&target).is_dir();
        writeln!(w, "host          : gemini")?;
        writeln!(w, "install target: {target}")?;
        writeln!(w, "target exists : {exists}")?;
        writeln!(
            w,
            "manifest      : gemini-extension.json (mcpServers inline) + GEMINI.md"
        )?;
        writeln!(
            w,
            "register      : manual — `gemini extensions install <target>` or symlink"
        )?;
        Ok(())
    }
}

impl Doctor for Host {
    fn doctor(&self) -> DoctorReport {
        super::doctor::doctor(self)
    }
}
