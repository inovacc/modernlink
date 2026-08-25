//! Built-in group `cli` — mirrors `pkg/aihost/assets/cli/`.

mod cli;

pub(super) fn register() {
    cli::register();
}
