//! Layout: `gemini-extension.json` at the extension root (mcpServers inline) +
//! `GEMINI.md` context + `skills/<name>/SKILL.md`; installs to
//! `~/.gemini/extensions/<name>/`.

use crate::claude;
use crate::error::{Error, Result};
use crate::host::{Doctor, DoctorReport, Host as HostTrait, Installer, Status};
use crate::host_registry::{run_cli, user_home_dir};
use crate::write_tree::{TreeWriter, write_tree_atomic};
use crate::{Kind, TemplateData, asset_by_path, portable_library_skills};
use serde::Serialize;
use serde_json::json;
use std::io::Write;
use std::path::Path;

pub const NAME: &str = "modernlink";
pub const VERSION: &str = claude::VERSION;
pub const DESCRIPTION: &str = claude::DESCRIPTION;
pub const MCP_COMMAND: &str = claude::MCP_COMMAND;

/// The Gemini CLI host.
#[derive(Debug, Default, Clone, Copy)]
pub struct Host;

fn template_data() -> TemplateData {
    TemplateData {
        name: NAME.to_string(),
        version: VERSION.to_string(),
        description: DESCRIPTION.to_string(),
        mcp_command: MCP_COMMAND.to_string(),
        created: String::new(),
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

fn marshal_indent<T: Serialize>(v: &T) -> Result<Vec<u8>> {
    let s = serde_json::to_string_pretty(v).map_err(|e| Error::Io(format!("marshal: {e}")))?;
    Ok(format!("{s}\n").into_bytes())
}

fn extension_json() -> Result<Vec<u8>> {
    marshal_indent(&json!({
        "name": NAME,
        "version": VERSION,
        "description": DESCRIPTION,
        "contextFileName": "GEMINI.md",
        "mcpServers": { NAME: { "command": MCP_COMMAND, "args": ["mcp"] } },
    }))
}

/// The GEMINI.md context file body, sourced from the shared registry.
fn gemini_context() -> String {
    "# modernlink\n\nModernLink host integration context.".to_string()
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
        let skill = asset_by_path(Kind::Skill, "skills/enrich/SKILL.md").ok_or_else(|| {
            Error::Io("gemini: missing source skill enrich/SKILL.md in shared registry".to_string())
        })?;
        let body = skill.render(td.clone())?;
        f("skills/enrich/SKILL.md", &body)?;
        for lib in portable_library_skills() {
            let lb = lib.render(td.clone())?;
            f(&lib.path, &lb)?;
        }
        Ok(())
    }

    fn manifest_files(&self) -> Result<Vec<(String, Vec<u8>)>> {
        Ok(vec![
            ("gemini-extension.json".to_string(), extension_json()?),
            ("GEMINI.md".to_string(), gemini_context().into_bytes()),
        ])
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
