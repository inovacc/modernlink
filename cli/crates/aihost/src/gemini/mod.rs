//! Implementation of `pkg/aihost/gemini` — the modernlink plugin host for Rustogle's Gemini
//! CLI. Registered explicitly (Rust uses `init()`) via [`register`] from the
//! `all` barrel.

mod doctor;
mod host;

pub use host::Host;

fn factory() -> Box<dyn crate::host::Host> {
    Box::new(Host)
}

/// Register the Gemini host into the global host registry.
pub(crate) fn register() {
    crate::host_registry::register_host(factory);
}
