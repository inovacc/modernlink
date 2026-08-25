//! rendered asset tree, patch `marketplace.json` + `settings.json`, clean legacy
//! state, and register the local marketplace via the `claude` CLI.

use super::manifest::{DESCRIPTION, NAME, VERSION, marshal_indent};
use super::{host::Host, mcpagents};
use crate::error::{Error, Result};
use crate::host_registry::{run_cli, user_home_dir};
use crate::write_tree::write_tree_atomic;
use serde_json::{Map, Value, json};
use std::io::Write;
use std::path::{Path, PathBuf};

const PLUGIN_MARKETPLACE: &str = "modernlink";
const PLUGIN_ENABLED_KEY: &str = "modernlink@modernlink";
const PLUGIN_SOURCE_PATH: &str = "./";
const PLUGIN_CATEGORY: &str = "developer-tools";

/// Run the full Claude-side install ritual. Returns the file-count written.
pub fn install(target: &str) -> Result<i32> {
    if target.is_empty() {
        return Err(Error::Io("target must not be empty".to_string()));
    }
    let n = write_tree_atomic(&Host, target, &["commands", "agents", "skills"])
        .map_err(|e| Error::Io(format!("write assets: {e}")))?;
    patch_marketplace().map_err(|e| Error::Io(format!("patch marketplace.json: {e}")))?;
    patch_settings().map_err(|e| Error::Io(format!("patch settings.json: {e}")))?;
    if unpatch_mcp_servers().is_ok() {
        eprintln!(
            "[install] cleaned legacy settings.json mcpServers[modernlink] (CLI-first: no global MCP registration; structured MCP is flow-scoped via the *-mcp agents)"
        );
    }
    migrate_legacy_local_plugin();
    register_local_marketplace(target);
    match user_home_dir() {
        Ok(home) => match mcpagents::write_mcp_scoped_agents(&home) {
            Ok(paths) => {
                eprintln!("[install] wrote {} flow-scoped MCP agents:", paths.len());
                for p in paths {
                    eprintln!("  {}", p.display());
                }
            }
            Err(e) => eprintln!("[install] mcp-scoped agents: {e} (continuing)"),
        },
        Err(e) => eprintln!("[install] home dir: {e} (continuing)"),
    }
    Ok(n)
}

/// Remove the install target and undo the JSON patches.
pub fn uninstall(target: &str) -> Result<()> {
    if let Err(e) = unpatch_settings() {
        eprintln!("[uninstall] settings.json: {e} (continuing)");
    }
    if let Err(e) = unpatch_mcp_servers() {
        eprintln!("[uninstall] mcpServers: {e} (continuing)");
    }
    if let Ok(home) = user_home_dir() {
        if let Err(e) = mcpagents::remove_mcp_scoped_agents(&home) {
            eprintln!("[uninstall] mcp-scoped agents: {e} (continuing)");
        }
    }
    std::fs::remove_dir_all(target).map_err(|e| Error::Io(format!("remove {target}: {e}")))?;
    eprintln!("[uninstall] removed {target}");
    Ok(())
}

/// Write the owned `marketplace.json` declaring a single-plugin marketplace.
pub fn patch_marketplace() -> Result<()> {
    let path = marketplace_json_path()?;
    let doc = json!({
        "name": PLUGIN_MARKETPLACE,
        "description": "modernlink — JS module enrichment plugin (private marketplace).",
        "owner": { "name": "modernlink" },
        "plugins": [{
            "name": NAME,
            "description": DESCRIPTION,
            "version": VERSION,
            "source": PLUGIN_SOURCE_PATH,
            "caterustry": PLUGIN_CATEGORY,
        }],
    });
    atomic_write_json(&path, &doc)
}

/// Flip `~/.claude/settings.json` enabledPlugins[modernlink@modernlink]=true.
pub fn patch_settings() -> Result<()> {
    let path = settings_json_path()?;
    let mut doc = read_json_object_or_empty(&path)?;
    let enabled = doc
        .entry("enabledPlugins")
        .or_insert_with(|| Value::Object(Map::new()));
    if !enabled.is_object() {
        *enabled = Value::Object(Map::new());
    }
    enabled
        .as_object_mut()
        .unwrap()
        .insert(PLUGIN_ENABLED_KEY.to_string(), Value::Bool(true));
    atomic_write_json(&path, &Value::Object(doc))
}

/// Remove the enabledPlugins entry.
pub fn unpatch_settings() -> Result<()> {
    let path = settings_json_path()?;
    let mut doc = match read_json_object_if_exists(&path)? {
        Some(d) => d,
        None => return Ok(()),
    };
    if let Some(enabled) = doc.get_mut("enabledPlugins").and_then(Value::as_object_mut) {
        enabled.remove(PLUGIN_ENABLED_KEY);
    }
    atomic_write_json(&path, &Value::Object(doc))
}

/// Remove the legacy settings.json mcpServers entry.
pub fn unpatch_mcp_servers() -> Result<()> {
    let path = settings_json_path()?;
    let mut doc = match read_json_object_if_exists(&path)? {
        Some(d) => d,
        None => return Ok(()),
    };
    if let Some(servers) = doc.get_mut("mcpServers").and_then(Value::as_object_mut) {
        servers.remove(NAME);
    }
    atomic_write_json(&path, &Value::Object(doc))
}

/// Best-effort cleanup of the old `~/.claude/local-plugins/` layout. Soft errors.
pub fn migrate_legacy_local_plugin() {
    let home = match user_home_dir() {
        Ok(h) => h,
        Err(_) => return,
    };
    let legacy_dir = Path::new(&home)
        .join(".claude")
        .join("local-plugins")
        .join("modernlink");
    if legacy_dir.exists() && std::fs::remove_dir_all(&legacy_dir).is_ok() {
        eprintln!(
            "[install] migrated: removed legacy {}",
            legacy_dir.display()
        );
    }
    let legacy_market = Path::new(&home)
        .join(".claude")
        .join("local-plugins")
        .join(".claude-plugin")
        .join("marketplace.json");
    if let Ok(raw) = std::fs::read(&legacy_market) {
        if let Ok(Value::Object(mut doc)) = serde_json::from_slice::<Value>(&raw) {
            if let Some(Value::Array(plugins)) = doc.get("plugins").cloned() {
                let kept: Vec<Value> = plugins
                    .into_iter()
                    .filter(|p| p.get("name").and_then(Value::as_str) != Some("modernlink"))
                    .collect();
                let dropped = kept.len() != doc["plugins"].as_array().map_or(0, Vec::len);
                if dropped {
                    doc.insert("plugins".to_string(), Value::Array(kept));
                    if atomic_write_json(&legacy_market, &Value::Object(doc)).is_ok() {
                        eprintln!(
                            "[install] migrated: removed legacy local-plugins marketplace entry"
                        );
                    }
                }
            }
        }
    }
    let settings_path = match settings_json_path() {
        Ok(p) => p,
        Err(_) => return,
    };
    if let Ok(Some(mut doc)) = read_json_object_if_exists(&settings_path) {
        if let Some(enabled) = doc.get_mut("enabledPlugins").and_then(Value::as_object_mut) {
            if enabled.remove("modernlink@local-plugins").is_some()
                && atomic_write_json(&settings_path, &Value::Object(doc)).is_ok()
            {
                eprintln!("[install] migrated: removed legacy enabledPlugins key");
            }
        }
    }
}

/// Shell out `claude plugin marketplace add <target>` so CC indexes our source.
pub fn register_local_marketplace(target: &str) {
    use crate::host_registry::CliOutcome;
    match run_cli("claude", &["plugin", "marketplace", "add", target]) {
        CliOutcome::Success => {
            eprintln!("[install] registered marketplace via claude plugin marketplace add");
        }
        CliOutcome::NotFound => {
            eprintln!(
                "[install] claude CLI not on PATH — run manually:\n  claude plugin marketplace add {target:?}"
            );
        }
        CliOutcome::TimedOut => {
            eprintln!(
                "[install] claude marketplace add timed out — run manually:\n  claude plugin marketplace add {target:?}"
            );
        }
        CliOutcome::Failed(msg) => eprintln!("[install] marketplace add said: {msg}"),
    }
}

/// Whether a GLOBAL modernlink MCP server is registered (plugin `.mcp.json` spec
/// form, or legacy settings.json). Returns the command if present.
pub fn mcp_servers_has_modernlink() -> Option<String> {
    let home = user_home_dir().ok()?;
    let plugin_mcp = Path::new(&home)
        .join(".claude")
        .join("plugins")
        .join("marketplaces")
        .join(NAME)
        .join(".mcp.json");
    if let Ok(raw) = std::fs::read(&plugin_mcp) {
        if let Ok(doc) = serde_json::from_slice::<Value>(&raw) {
            if let Some(entry) = doc.get("mcpServers").and_then(|s| s.get(NAME)) {
                return Some(
                    entry
                        .get("command")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                );
            }
        }
    }
    let settings_path = settings_json_path().ok()?;
    let raw = std::fs::read(&settings_path).ok()?;
    let doc: Value = serde_json::from_slice(&raw).ok()?;
    let entry = doc.get("mcpServers")?.get(NAME)?;
    Some(
        entry
            .get("command")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
    )
}

/// Whether marketplace.json declares modernlink.
pub fn marketplace_has_entry() -> bool {
    let path = match marketplace_json_path() {
        Ok(p) => p,
        Err(_) => return false,
    };
    let raw = match std::fs::read(&path) {
        Ok(r) => r,
        Err(_) => return false,
    };
    let doc: Value = match serde_json::from_slice(&raw) {
        Ok(d) => d,
        Err(_) => return false,
    };
    doc.get("plugins")
        .and_then(Value::as_array)
        .is_some_and(|plugins| {
            plugins
                .iter()
                .any(|p| p.get("name").and_then(Value::as_str) == Some(NAME))
        })
}

/// Whether enabledPlugins flips us on.
pub fn settings_has_enabled() -> bool {
    let path = match settings_json_path() {
        Ok(p) => p,
        Err(_) => return false,
    };
    let raw = match std::fs::read(&path) {
        Ok(r) => r,
        Err(_) => return false,
    };
    let doc: Value = match serde_json::from_slice(&raw) {
        Ok(d) => d,
        Err(_) => return false,
    };
    doc.get("enabledPlugins")
        .and_then(|e| e.get(PLUGIN_ENABLED_KEY))
        .and_then(Value::as_bool)
        == Some(true)
}

/// Write the install-state reintegration.
pub fn print_status(w: &mut dyn Write) -> std::io::Result<()> {
    let home = user_home_dir().map_err(|e| std::io::Error::other(e.to_string()))?;
    let target = Path::new(&home)
        .join(".claude")
        .join("plugins")
        .join("marketplaces")
        .join(NAME);
    let mcp = mcp_servers_has_modernlink();
    writeln!(w, "embedded plugin    : {NAME} v{VERSION}")?;
    writeln!(w, "install target     : {}", target.display())?;
    writeln!(w, "target exists      : {}", target.exists())?;
    writeln!(w, "marketplace entry  : {}", marketplace_has_entry())?;
    writeln!(w, "settings enabled   : {}", settings_has_enabled())?;
    match mcp {
        Some(cmd) => writeln!(
            w,
            "mcp global reg     : WARN present ({cmd}) — CLI-first forbids; reinstall to clear"
        )?,
        None => writeln!(
            w,
            "mcp global reg     : none (CLI-first; structured MCP via flow-scoped *-mcp agents)"
        )?,
    }
    Ok(())
}

// ---- private helpers ----

fn marketplace_json_path() -> Result<PathBuf> {
    Ok(Path::new(&user_home_dir()?)
        .join(".claude")
        .join("plugins")
        .join("marketplaces")
        .join(PLUGIN_MARKETPLACE)
        .join(".claude-plugin")
        .join("marketplace.json"))
}

fn settings_json_path() -> Result<PathBuf> {
    Ok(Path::new(&user_home_dir()?)
        .join(".claude")
        .join("settings.json"))
}

/// Read a JSON object from `path`, treating a missing file as `{}` (Rust's
/// `os.IsNotExist` -> `{}` path).
fn read_json_object_or_empty(path: &Path) -> Result<Map<String, Value>> {
    match std::fs::read(path) {
        Ok(raw) => serde_json::from_slice::<Map<String, Value>>(&raw)
            .map_err(|e| Error::Io(format!("parse {}: {e}", path.display()))),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Map::new()),
        Err(e) => Err(Error::Io(format!("read {}: {e}", path.display()))),
    }
}

/// Like [`read_json_object_or_empty`] but returns `None` for a missing file (the
/// unpatch/migrate no-op-on-absent path).
fn read_json_object_if_exists(path: &Path) -> Result<Option<Map<String, Value>>> {
    match std::fs::read(path) {
        Ok(raw) => serde_json::from_slice::<Map<String, Value>>(&raw)
            .map(Some)
            .map_err(|e| Error::Io(format!("parse {}: {e}", path.display()))),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(Error::Io(format!("read {}: {e}", path.display()))),
    }
}

fn atomic_write_json(path: &Path, doc: &Value) -> Result<()> {
    let out = marshal_indent(doc, &path.display().to_string())?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| Error::Io(format!("mkdir {}: {e}", parent.display())))?;
    }
    let tmp = {
        let mut s = path.as_os_str().to_owned();
        s.push(".tmp");
        PathBuf::from(s)
    };
    std::fs::write(&tmp, &out).map_err(|e| Error::Io(format!("write {}: {e}", tmp.display())))?;
    std::fs::rename(&tmp, path).map_err(|e| Error::Io(format!("rename {}: {e}", path.display())))
}
