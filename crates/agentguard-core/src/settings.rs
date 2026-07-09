//! Round-trip-safe Claude Code `settings.json` load/save (D5).
//!
//! STUB (Iteration 0): the preserving parse/serialize is implemented in a later
//! iteration. The contract is fixed here so the Tauri command layer can be wired.
//!
//! Design (see `docs/policy-model.md` §6):
//! - Parse the whole file into `serde_json::Value`, touching only
//!   `permissions.{allow,ask,deny,defaultMode}`; every other key is preserved.
//! - Managed rules are re-emitted deterministically; unmanaged rules kept verbatim.
//! - On any parse/convert failure, DO NOT write — preserve the original (req §9.4).

use crate::model::{PolicyRule, Scope};
use crate::policy::{Permissions, UnmanagedRules};
use serde_json::Value;
use std::path::{Path, PathBuf};

/// Locations of the three settings files for a project (req §7).
#[derive(Debug, Clone)]
pub struct ScopePaths {
    pub user: PathBuf,
    pub project: PathBuf,
    pub local: PathBuf,
}

/// Resolve the three settings file paths for `project_root`.
pub fn scope_paths(project_root: &Path, home_dir: &Path) -> ScopePaths {
    ScopePaths {
        user: home_dir.join(".claude").join("settings.json"),
        project: project_root.join(".claude").join("settings.json"),
        local: project_root.join(".claude").join("settings.local.json"),
    }
}

/// A parsed settings file: the full JSON tree plus extracted permissions.
#[derive(Debug, Clone)]
pub struct LoadedSettings {
    pub scope: Scope,
    /// Full original JSON tree (preserved for round-trip).
    pub raw: Value,
    pub permissions: Permissions,
    pub default_mode: Option<String>,
}

/// Parse settings text into [`LoadedSettings`] without losing unknown fields.
pub fn parse(_scope: Scope, _text: &str) -> crate::Result<LoadedSettings> {
    // TODO(iteration-2): implement preserving parse + fold via policy::from_permissions.
    Err(crate::Error::Other(
        "settings::parse not yet implemented".into(),
    ))
}

/// Merge managed `rules` (+ preserved `unmanaged`) back into `base` JSON and return
/// the serialized text to write. Never mutates keys outside `permissions`.
pub fn render(
    _base: &Value,
    _rules: &[PolicyRule],
    _unmanaged: &UnmanagedRules,
    _default_mode: Option<&str>,
) -> crate::Result<String> {
    // TODO(iteration-2): implement deterministic re-emit + unmanaged preservation.
    Err(crate::Error::Other(
        "settings::render not yet implemented".into(),
    ))
}
