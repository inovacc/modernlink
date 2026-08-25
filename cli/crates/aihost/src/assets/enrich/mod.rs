//! Built-in group `enrich` — mirrors `pkg/aihost/assets/enrich/`.

mod embedded;
mod enrich;

pub(super) fn register() {
    enrich::register();
    embedded::register();
}
