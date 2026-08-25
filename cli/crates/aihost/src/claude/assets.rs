//!
//! Claude's asset bodies are not defined here; they live in the shared
//! `crate::assets` registry (mirroring `pkg/aihost/assets/*`), and this host
//! renders every registered asset via [`super::host`]'s `walk`.
