//! Built-in group `ops` — mirrors `pkg/aihost/assets/ops/`.

mod embedded;
mod ops;

pub(super) fn register() {
    ops::register();
    embedded::register();
}
