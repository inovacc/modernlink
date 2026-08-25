//! Built-in group `dissect` — mirrors `pkg/aihost/assets/dissect/`.

mod dissect;

pub(super) fn register() {
    dissect::register();
}
