//! Native Rust implementation of the ModernLink AI-host integration layer.
//!
//! Defines modernlink's cross-host plugin-packaging contract: the [`Host`] trait
//! and its optional capability traits ([`Installer`], [`Status`], [`Doctor`]),
//! the [`Asset`] registry ([`register_asset`], [`asset_by_path`],
//! [`assets_by_kind`], [`all_assets`]), the [`TreeWriter`] + [`write_tree_atomic`]
//! shared writer, and the synthesized [`portable_library_skills`].
//!
//! Scope: host contracts, registries, rendering, and built-in asset groups under [`assets`]
//! and the two registration barrels ([`all`],
//! `assets::all`). Concrete hosts (Claude/Codex/Gemini) live in source implementation
//! subpackages.

mod all;
mod asset;
pub mod assets;
mod claude;
mod codex;
mod error;
mod gemini;
mod host;
mod host_registry;
mod portable_libraries;
mod registry;
mod write_tree;

pub const PLUGIN_NAME: &str = "modernlink";
pub const PLUGIN_VERSION: &str = "0.1.0";
pub const PLUGIN_DESCRIPTION: &str = "Evidence-first repository modernization preparation backed by the deterministic ModernLink Rust CLI";
pub const PLUGIN_MCP_COMMAND: &str = "modernlink";

pub use all::register_all;
pub use asset::{
    Asset, DEFAULT_CREATED, Kind, TEMPLATE_DELIMS_END, TEMPLATE_DELIMS_START, TemplateData,
    current_date, kind_for_path,
};
pub use error::{Error, Result};
pub use host::{Doctor, DoctorCheck, DoctorReport, Host, Installer, Status};
pub use host_registry::{Factory, all_hosts, host_by_name, register_host};
pub use portable_libraries::{
    AGENT_LIBRARY_SKILL_PATH, COMMAND_LIBRARY_SKILL_PATH, portable_library_skills,
};
pub use registry::{all_assets, asset_by_path, assets_by_kind, register_asset};
pub use write_tree::{TreeWriter, write_tree_atomic};

/// Render the complete plugin bundle into relative paths and bytes.
pub fn render_plugin(data: TemplateData) -> Result<Vec<(String, Vec<u8>)>> {
    let mut files = assets::bundle::assets()
        .into_iter()
        .map(|asset| {
            let path = asset.path.clone();
            asset.render(data.clone()).map(|bytes| (path, bytes))
        })
        .collect::<Result<Vec<_>>>()?;
    files.extend(assets::bundle::all_static_files());
    Ok(files)
}

#[cfg(test)]
mod tests;
