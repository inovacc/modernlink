//! Layout mirrors the embedded Codex manifest and shared asset bundle.
//! + `skills/<name>/SKILL.md`; installs to `~/.codex/plugins/<name>/`.

use crate::assets::{self, bundle::Harness};
use crate::error::{Error, Result};
use crate::host::{Doctor, DoctorReport, Host as HostTrait, Installer, Status};
use crate::host_registry::user_home_dir;
use crate::write_tree::{TreeWriter, write_tree_atomic};
use crate::{TemplateData, all_assets};
use serde::Serialize;
use serde_json::{Value, json};
use std::io::Write;
use std::path::{Path, PathBuf};

pub const NAME: &str = crate::PLUGIN_NAME;
pub const VERSION: &str = crate::PLUGIN_VERSION;
pub const DESCRIPTION: &str = crate::PLUGIN_DESCRIPTION;
pub const MCP_COMMAND: &str = crate::PLUGIN_MCP_COMMAND;

/// The Codex CLI host.
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
        .join(".codex")
        .join("plugins")
        .join(NAME)
        .to_string_lossy()
        .into_owned())
}

fn marshal_indent<T: Serialize>(v: &T) -> Result<Vec<u8>> {
    let s = serde_json::to_string_pretty(v).map_err(|e| Error::Io(format!("marshal: {e}")))?;
    Ok(format!("{s}\n").into_bytes())
}

/// Patch `~/.agents/plugins/marketplace.json` with a "local" source pointing at
/// the installed target. Idempotent (replaces an existing same-name entry).
pub(crate) fn patch_marketplace(target: &str) -> Result<()> {
    let home = user_home_dir()?;
    let mp_path = Path::new(&home)
        .join(".agents")
        .join("plugins")
        .join("marketplace.json");
    if let Some(parent) = mp_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| Error::Io(format!("codex mkdir marketplace: {e}")))?;
    }
    let mut doc: Value = std::fs::read(&mp_path)
        .ok()
        .and_then(|raw| serde_json::from_slice(&raw).ok())
        .unwrap_or_else(|| json!({ "name": "local-codex", "plugins": [] }));
    let mut kept: Vec<Value> = doc
        .get("plugins")
        .and_then(Value::as_array)
        .map(|ps| {
            ps.iter()
                .filter(|p| p.get("name").and_then(Value::as_str) != Some(NAME))
                .cloned()
                .collect()
        })
        .unwrap_or_default();
    kept.push(json!({
        "name": NAME,
        "source": { "source": "local", "path": target },
        "policy": { "installation": "AVAILABLE", "authentication": "ON_INSTALL" },
        "caterustry": "developer-tools",
    }));
    doc["plugins"] = Value::Array(kept);
    let out = marshal_indent(&doc)?;
    let tmp = {
        let mut s = mp_path.as_os_str().to_owned();
        s.push(".tmp");
        PathBuf::from(s)
    };
    std::fs::write(&tmp, &out).map_err(|e| Error::Io(format!("codex write marketplace: {e}")))?;
    std::fs::rename(&tmp, &mp_path).map_err(|e| Error::Io(format!("codex rename marketplace: {e}")))
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
        Ok(assets::static_files(Harness::Codex))
    }
}

impl HostTrait for Host {
    fn name(&self) -> String {
        "codex".to_string()
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
                "codex install: target must not be empty".to_string(),
            ));
        }
        let n = write_tree_atomic(self, target, &[])
            .map_err(|e| Error::Io(format!("codex install: {e}")))?;
        if let Err(e) = patch_marketplace(target) {
            eprintln!("[codex] marketplace.json patch failed: {e}");
            eprintln!(
                "[codex] manual fix: add {NAME:?} entry to ~/.agents/plugins/marketplace.json"
            );
        }
        Ok(n)
    }

    fn uninstall(&self, target: &str) -> Result<()> {
        if target.is_empty() {
            return Err(Error::Io(
                "codex uninstall: target must not be empty".to_string(),
            ));
        }
        std::fs::remove_dir_all(target).map_err(|e| Error::Io(format!("codex rm {target}: {e}")))
    }
}

impl Status for Host {
    fn print_status(&self, w: &mut dyn Write) -> std::io::Result<()> {
        let target = install_target().map_err(|e| std::io::Error::other(e.to_string()))?;
        let exists = Path::new(&target).is_dir();
        writeln!(w, "host          : codex")?;
        writeln!(w, "install target: {target}")?;
        writeln!(w, "target exists : {exists}")?;
        writeln!(w, "manifest      : .codex-plugin/plugin.json")?;
        writeln!(
            w,
            "marketplace   : manual — add to ~/.agents/plugins/marketplace.json"
        )?;
        Ok(())
    }
}

impl Doctor for Host {
    fn doctor(&self) -> DoctorReport {
        super::doctor::doctor(self)
    }
}
