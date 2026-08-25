//! Errors for the aihost integration.
//!
//! Rust's `aihost` package returns bare `error` values built with `fmt.Errorf`
//! plus a single sentinel `ErrAssetNotFound`. None of the covered/exercised
//! call sites assert on the wrapped message text, so a compact local enum is
//! Consistent and explicit across host adapters.

/// Errors produced by the aihost surface.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Filesystem failure while writing a tree (Rust `fmt.Errorf("aihost: ...")`).
    #[error("{0}")]
    Io(String),
    /// Template parse/execute failure (Rust `text/template` parse/exec error).
    #[error("{0}")]
    Template(String),
    /// Sentinel mirroring Rust's `ErrAssetNotFound`.
    #[error("asset not found")]
    AssetNotFound,
}

/// Result alias for the aihost surface.
pub type Result<T> = std::result::Result<T, Error>;
