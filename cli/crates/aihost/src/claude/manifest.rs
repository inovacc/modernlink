//! Metadata shared by the host adapters.

pub use crate::{
    PLUGIN_DESCRIPTION as DESCRIPTION, PLUGIN_MCP_COMMAND as MCP_COMMAND, PLUGIN_NAME as NAME,
    PLUGIN_VERSION as VERSION,
};

pub fn marshal_indent(value: &serde_json::Value, path: &str) -> crate::Result<Vec<u8>> {
    let mut bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| crate::Error::Io(format!("serialize {path}: {error}")))?;
    bytes.push(b'\n');
    Ok(bytes)
}
