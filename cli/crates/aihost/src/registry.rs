//! Process-global asset registry (Rust package-level `assetRegistry []Asset`).
//!
//! Modeled as a `Mutex<Vec<Asset>>` behind a `OnceLock`. Rust's semantics
//! reproduced exactly: `RegisterAsset` APPENDS (no dedup, insertion order
//! preserved); `AssetByPath` returns the FIRST insertion-order match;
//! `AssetsByKind` / `AllAssets` return path-sorted copies.
//!
//! Rust runs tests in parallel, so any test that mutates the registry must
//! serialize on [`test_support::REGISTRY_TEST_LOCK`] and reset via
//! [`test_support::reset_registry`].

use crate::asset::{Asset, Kind, kind_for_path};
use std::sync::{Mutex, OnceLock};

fn registry() -> &'static Mutex<Vec<Asset>> {
    static REGISTRY: OnceLock<Mutex<Vec<Asset>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(Vec::new()))
}

/// Add one or more assets to the global registry. An asset whose kind is
/// [`Kind::Unknown`] has its kind inferred from its path (Rust `RegisterAsset`).
pub fn register_asset(assets: &[Asset]) {
    let mut reg = registry().lock().unwrap();
    for a in assets {
        let mut a = a.clone();
        if a.kind == Kind::Unknown {
            a.kind = kind_for_path(&a.path);
        }
        reg.push(a);
    }
}

/// Return the asset matching `(kind, path)` — first insertion-order match.
pub fn asset_by_path(kind: Kind, path: &str) -> Option<Asset> {
    let reg = registry().lock().unwrap();
    reg.iter()
        .find(|a| a.kind == kind && a.path == path)
        .cloned()
}

/// Return every asset of the given kind, sorted by path.
pub fn assets_by_kind(kind: Kind) -> Vec<Asset> {
    let reg = registry().lock().unwrap();
    let mut out: Vec<Asset> = reg.iter().filter(|a| a.kind == kind).cloned().collect();
    out.sort_by(|a, b| a.path.cmp(&b.path));
    out
}

/// Return every registered asset, sorted by path.
pub fn all_assets() -> Vec<Asset> {
    let reg = registry().lock().unwrap();
    let mut out: Vec<Asset> = reg.clone();
    out.sort_by(|a, b| a.path.cmp(&b.path));
    out
}

#[cfg(test)]
pub(crate) mod test_support {
    use super::registry;
    use std::sync::Mutex;

    /// Serializes all registry-mutating tests against each other.
    pub(crate) static REGISTRY_TEST_LOCK: Mutex<()> = Mutex::new(());

    /// Clear the global registry so a test starts from a known-empty state.
    pub(crate) fn reset_registry() {
        registry().lock().unwrap().clear();
    }
}
