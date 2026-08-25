//! Built-in group `knowledge` — mirrors `pkg/aihost/assets/knowledge/`.

mod brainstorm;
mod findings;
mod knowledge;

pub(super) fn register() {
    brainstorm::register();
    findings::register();
    knowledge::register();
}
