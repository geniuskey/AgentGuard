//! Neutral policy model shared across the app.
//!
//! Agent Guard keeps a *path-centric* neutral model (this module). Claude Code's
//! `settings.json` is *tool-centric* (`Read(./src/**)` etc.); conversion happens
//! in [`crate::policy`]. See `docs/policy-model.md`.

use serde::{Deserialize, Serialize};

/// A policy decision for a path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Policy {
    Allow,
    Ask,
    Deny,
}

/// Which settings scope a rule belongs to. Merge precedence: Local > Project > User
/// (but a `deny` in any scope wins over an `allow` in another — see [`crate::effective`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Scope {
    User,
    Project,
    Local,
}

/// How a rule's path is interpreted when expanded to a Claude Code specifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AppliesTo {
    /// A single file: `./src` (no glob).
    File,
    /// Direct children only: `./src/*`.
    Folder,
    /// Recursive: `./src/**`.
    FolderAndChildren,
    /// The `path` is already a raw gitignore-style pattern; used verbatim.
    Pattern,
}

/// Claude Code tools that accept a path/pattern specifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Tool {
    Read,
    Edit,
    Write,
    Grep,
    Glob,
    NotebookEdit,
}

impl Tool {
    pub fn as_str(self) -> &'static str {
        match self {
            Tool::Read => "Read",
            Tool::Edit => "Edit",
            Tool::Write => "Write",
            Tool::Grep => "Grep",
            Tool::Glob => "Glob",
            Tool::NotebookEdit => "NotebookEdit",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Tool> {
        Some(match s {
            "Read" => Tool::Read,
            "Edit" => Tool::Edit,
            "Write" => Tool::Write,
            "Grep" => Tool::Grep,
            "Glob" => Tool::Glob,
            "NotebookEdit" => Tool::NotebookEdit,
            _ => return None,
        })
    }
}

/// The fixed set of file-access tools a path rule fans out into (D1).
/// Order is significant: it defines the deterministic emission order used when
/// writing `settings.json`, keeping diffs stable.
pub const FILE_ACCESS_TOOLS: [Tool; 6] = [
    Tool::Read,
    Tool::Edit,
    Tool::Write,
    Tool::Grep,
    Tool::Glob,
    Tool::NotebookEdit,
];

/// Risk level annotation (app metadata, stored in SQLite only — see D3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

/// A single neutral policy rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PolicyRule {
    /// Project-root-relative POSIX path (or raw pattern when `applies_to == Pattern`).
    pub path: String,
    pub policy: Policy,
    pub applies_to: AppliesTo,
    /// `None` = fan out to all [`FILE_ACCESS_TOOLS`]; `Some(..)` = tool-specific subset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
    // --- app metadata (persisted in SQLite, never written to settings.json) ---
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub risk_level: Option<RiskLevel>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

impl PolicyRule {
    /// Convenience constructor for a fan-out-all rule with no metadata.
    pub fn new(path: impl Into<String>, policy: Policy, applies_to: AppliesTo) -> Self {
        PolicyRule {
            path: path.into(),
            policy,
            applies_to,
            tools: None,
            reason: None,
            risk_level: None,
            notes: None,
        }
    }
}
