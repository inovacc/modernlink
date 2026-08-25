//! Built-in group `xref` — mirrors `pkg/aihost/assets/xref/`.

mod xref;

pub(super) fn register() {
    xref::register();
}
