//! This barrel registers the asset bundle and every concrete host explicitly:
//! every asset group (via `assets::all`) and every host (claude/codex/gemini).

/// Register every portable asset and every host into the global registries.
/// Call exactly once before querying them (a second call APPENDS again,
/// matching Rust).
pub fn register_all() {
    crate::assets::all::register();
    crate::claude::register();
    crate::codex::register();
    crate::gemini::register();
}
