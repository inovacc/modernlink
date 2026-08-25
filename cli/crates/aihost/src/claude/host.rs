//! contract for Anthropic's Claude Code, plus the asset-walk helpers.

use super::manifest::{DESCRIPTION, MCP_COMMAND, NAME, VERSION};
use crate::assets::{self, bundle::Harness};
use crate::error::Result;
use crate::host::{Doctor, DoctorReport, Host as HostTrait, Installer, Status};
use crate::host_registry::user_home_dir;
use crate::write_tree::TreeWriter;
use crate::{Kind, TemplateData, all_assets, assets_by_kind};
use std::collections::BTreeMap;
use std::path::Path;

/// The Claude Code host.
#[derive(Debug, Default, Clone, Copy)]
pub struct Host;

/// The substitution map every Claude asset is rendered with at install time.
pub(crate) fn template_data() -> TemplateData {
    TemplateData {
        name: NAME.to_string(),
        version: VERSION.to_string(),
        description: DESCRIPTION.to_string(),
        mcp_command: MCP_COMMAND.to_string(),
        created: crate::current_date(),
    }
}

/// The canonical install target: `~/.claude/plugins/marketplaces/modernlink`.
pub fn install_target() -> Result<String> {
    Ok(Path::new(&user_home_dir()?)
        .join(".claude")
        .join("plugins")
        .join("marketplaces")
        .join(NAME)
        .to_string_lossy()
        .into_owned())
}

/// Asset counts grouped by top-level dir, plus the total.
// Retained public API (Rust `claude.Count`); the CLI consumer is not in this crate.
#[allow(dead_code)]
pub fn count() -> Result<(BTreeMap<String, i32>, i32)> {
    let mut counts: BTreeMap<String, i32> = BTreeMap::new();
    let mut total = 0;
    Host.walk(&mut |p: &str, _: &[u8]| {
        let top = p.split_once('/').map_or(p, |(a, _)| a);
        *counts.entry(top.to_string()).or_insert(0) += 1;
        total += 1;
        Ok(())
    })?;
    Ok((counts, total))
}

/// Sorted list of top-level slash-command stems.
// Retained public API (Rust `claude.CommandNames`); CLI consumer not in this crate.
#[allow(dead_code)]
pub fn command_names() -> Vec<String> {
    let mut names: Vec<String> = assets_by_kind(Kind::Command)
        .into_iter()
        .filter_map(|a| {
            let stem = a
                .path
                .strip_prefix("commands/")
                .unwrap_or(&a.path)
                .strip_suffix(".md")
                .unwrap_or(&a.path);
            if stem.is_empty() || stem.contains('/') {
                None
            } else {
                Some(stem.to_string())
            }
        })
        .collect();
    names.sort();
    names
}

impl TreeWriter for Host {
    fn walk(&self, f: &mut dyn FnMut(&str, &[u8]) -> Result<()>) -> Result<()> {
        let d = template_data();
        for a in all_assets() {
            let data = a.render(d.clone())?;
            f(&a.path, &data)?;
        }
        Ok(())
    }

    fn manifest_files(&self) -> Result<Vec<(String, Vec<u8>)>> {
        Ok(assets::static_files(Harness::Claude))
    }
}

impl HostTrait for Host {
    fn name(&self) -> String {
        "claude".to_string()
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
        super::install::install(target)
    }
    fn uninstall(&self, target: &str) -> Result<()> {
        super::install::uninstall(target)
    }
}

impl Status for Host {
    fn print_status(&self, w: &mut dyn std::io::Write) -> std::io::Result<()> {
        super::install::print_status(w)
    }
}

impl Doctor for Host {
    fn doctor(&self) -> DoctorReport {
        super::doctor::doctor()
    }
}
