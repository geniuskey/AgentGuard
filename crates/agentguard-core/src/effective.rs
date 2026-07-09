//! Effective-policy merge across scopes (D4) — see `docs/effective-policy.md`.
//!
//! STUB (Iteration 0): signatures fixed; the merge/conflict engine lands in a
//! later iteration. Evaluation is `deny > ask > allow` (first match), deny wins
//! across all scopes; unmatched paths fall back to `defaultMode`.

use crate::model::{Policy, Scope};
use serde::{Deserialize, Serialize};

/// Final decision for a path, with provenance for the Preview / Conflicts tabs.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EffectivePolicy {
    pub path: String,
    pub effective: Policy,
    pub source_scope: Option<Scope>,
    pub explicit: bool,
    pub conflict: bool,
}

/// Merged rule set feeding the computation (allow/ask/deny per scope).
#[derive(Debug, Default, Clone)]
pub struct ScopedRules {
    pub user: crate::policy::Permissions,
    pub project: crate::policy::Permissions,
    pub local: crate::policy::Permissions,
    pub default_mode: Option<String>,
}

/// Compute the effective policy for `target_path`.
pub fn compute(_rules: &ScopedRules, _target_path: &str) -> crate::Result<EffectivePolicy> {
    // TODO(iteration-1/2): implement deny>ask>allow first-match, deny-global-wins,
    // explicit-vs-inherited, and defaultMode fallback.
    Err(crate::Error::Other(
        "effective::compute not yet implemented".into(),
    ))
}
