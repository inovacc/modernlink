//! Rust blank-imports `assets/all` plus every concrete host package so their
//! `init()` functions register into the global registries. Rust has no
//! package-init side effects, so this calls each registration explicitly:
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
