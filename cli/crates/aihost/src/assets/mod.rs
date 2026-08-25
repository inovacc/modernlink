//! Runtime materialization of the embedded plugin bundle.

use crate::registry::register_asset;

pub(crate) mod all;
pub(crate) mod bundle;

pub(crate) fn register() {
    register_asset(&bundle::assets());
}

pub(crate) fn static_files(harness: bundle::Harness) -> Vec<(String, Vec<u8>)> {
    bundle::static_files(harness)
}
