//! Rust blank-imports each group package to trigger its `init()` registration;
//! Rust has no package-init side effects, so this barrel calls each group's
//! `register()` explicitly. Registers all thirteen groups.

pub(crate) fn register() {
    super::cli::register();
    super::dissect::register();
    super::enrich::register();
    super::knowledge::register();
    super::ops::register();
    super::supervisor::register();
    super::xref::register();
}
