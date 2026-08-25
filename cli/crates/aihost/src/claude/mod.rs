//! Claude host adapter. Asset bodies live in the shared `crate::assets` registry;
//! this host renders them and adds Claude-specific manifests and configuration.
//!
//! The host is registered explicitly via [`register`] from the top-level barrel.

mod doctor;
mod host;
mod install;
mod manifest;

pub use host::Host;
fn factory() -> Box<dyn crate::host::Host> {
    Box::new(Host)
}

/// Register the Claude host into the global host registry.
pub(crate) fn register() {
    crate::host_registry::register_host(factory);
}
