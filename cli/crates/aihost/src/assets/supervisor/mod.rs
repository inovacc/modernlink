//! Built-in group `supervisor` — mirrors `pkg/aihost/assets/supervisor/`.

mod supervisor;

pub(super) fn register() {
    supervisor::register();
}
