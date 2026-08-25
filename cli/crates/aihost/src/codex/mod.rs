//! Implementation of `pkg/aihost/codex` — the modernlink plugin host for OpenAI's Codex CLI.
//! Registered explicitly (Rust uses `init()`) via [`register`] from the `all`
//! barrel.

mod doctor;
mod host;

pub use host::Host;

fn factory() -> Box<dyn crate::host::Host> {
    Box::new(Host)
}

/// Register the Codex host into the global host registry.
pub(crate) fn register() {
    crate::host_registry::register_host(factory);
}
