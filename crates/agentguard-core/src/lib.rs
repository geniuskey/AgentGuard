//! Agent Guard core logic.
//!
//! This crate is intentionally free of any Tauri / UI dependency so that all the
//! security-critical logic (policy conversion, effective-policy merge, risk
//! scoring, round-trip-safe settings parsing, local storage) can be unit-tested
//! on any platform — including CI hosts without a WebView.
//!
//! See `docs/architecture.md` (module responsibilities) and `docs/policy-model.md`.

pub mod backup;
pub mod db;
pub mod effective;
pub mod env;
pub mod fs_scan;
pub mod gitignore;
pub mod model;
pub mod paths;
pub mod policy;
pub mod profiles;
pub mod report;
pub mod risk;
pub mod settings;

pub use model::{AppliesTo, Policy, PolicyRule, Scope, Tool, FILE_ACCESS_TOOLS};

/// Crate-wide error type.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("invalid rule: {0}")]
    InvalidRule(String),
    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;
