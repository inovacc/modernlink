//! Codex host adapter, registered explicitly by the host registry.

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
