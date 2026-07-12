//! Effective-policy merge across scopes (D4) — see `docs/effective-policy.md`.
//!
//! Evaluation is `deny > ask > allow` (first match), and `deny` wins across all
//! scopes. Paths with no matching rule fall back to `defaultMode`
//! (`dontAsk` => deny, otherwise => ask/prompt).

use crate::model::{AppliesTo, Policy, PolicyRule, Scope};
use globset::{Glob, GlobSetBuilder};
use serde::{Deserialize, Serialize};

/// Neutral rules grouped by scope, plus the effective `defaultMode`.
#[derive(Debug, Default, Clone)]
pub struct ScopedRules {
    pub user: Vec<PolicyRule>,
    pub project: Vec<PolicyRule>,
    pub local: Vec<PolicyRule>,
    pub default_mode: Option<String>,
}

impl ScopedRules {
    fn tagged(&self) -> Vec<(Scope, &PolicyRule)> {
        let mut v = Vec::new();
        v.extend(self.user.iter().map(|r| (Scope::User, r)));
        v.extend(self.project.iter().map(|r| (Scope::Project, r)));
        v.extend(self.local.iter().map(|r| (Scope::Local, r)));
        v
    }
}

/// Final decision for a path, with provenance for the Preview / Conflicts tabs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EffectivePolicy {
    pub path: String,
    pub effective: Policy,
    pub source_scope: Option<Scope>,
    /// A rule on this exact path decided it (vs. inherited from an ancestor rule).
    pub explicit: bool,
    /// Both allow and a (more specific or cross-scope) deny/ask matched.
    pub conflict: bool,
}

/// Normalize a path for matching: strip leading `./`, use forward slashes.
fn norm(path: &str) -> String {
    path.trim_start_matches("./").replace('\\', "/")
}

/// The glob patterns a rule matches against a (normalized) target path.
fn rule_patterns(rule: &PolicyRule) -> Vec<String> {
    let p = norm(&rule.path);
    match rule.applies_to {
        AppliesTo::File => vec![p],
        AppliesTo::Folder => vec![p.clone(), format!("{p}/*")],
        AppliesTo::FolderAndChildren => vec![p.clone(), format!("{p}/**")],
        AppliesTo::Pattern => vec![p],
    }
}

fn rule_matches(rule: &PolicyRule, target: &str) -> bool {
    let mut b = GlobSetBuilder::new();
    for pat in rule_patterns(rule) {
        if let Ok(g) = Glob::new(&pat) {
            b.add(g);
        }
    }
    match b.build() {
        Ok(set) => set.is_match(target),
        Err(_) => false,
    }
}

/// Fallback decision when no rule matches, based on `defaultMode`.
pub(crate) fn fallback(default_mode: Option<&str>) -> Policy {
    match default_mode {
        Some("dontAsk") => Policy::Deny,
        _ => Policy::Ask, // "default"/acceptEdits/etc. prompt at runtime
    }
}

/// Every rule (with its scope) matching `target_path`, in Local > Project > User
/// order. Used by the simulator to show *why* a path got its decision.
pub fn matching_rules(rules: &ScopedRules, target_path: &str) -> Vec<(Scope, PolicyRule)> {
    let target = norm(target_path);
    let mut matched: Vec<(Scope, PolicyRule)> = rules
        .tagged()
        .into_iter()
        .filter(|(_, r)| rule_matches(r, &target))
        .map(|(s, r)| (s, r.clone()))
        .collect();
    fn scope_rank(s: Scope) -> u8 {
        match s {
            Scope::Local => 0,
            Scope::Project => 1,
            Scope::User => 2,
        }
    }
    matched.sort_by_key(|(s, _)| scope_rank(*s));
    matched
}

/// Compute the effective policy for a single `target_path`.
pub fn compute_for(rules: &ScopedRules, target_path: &str) -> EffectivePolicy {
    let target = norm(target_path);
    let tagged = rules.tagged();

    let mut matched: Vec<(Scope, &PolicyRule)> = tagged
        .into_iter()
        .filter(|(_, r)| rule_matches(r, &target))
        .collect();

    if matched.is_empty() {
        return EffectivePolicy {
            path: target,
            effective: fallback(rules.default_mode.as_deref()),
            source_scope: None,
            explicit: false,
            conflict: false,
        };
    }

    // Local > Project > User for source attribution within the same policy.
    fn scope_rank(s: Scope) -> u8 {
        match s {
            Scope::Local => 0,
            Scope::Project => 1,
            Scope::User => 2,
        }
    }
    matched.sort_by_key(|(s, _)| scope_rank(*s));

    let has = |p: Policy| matched.iter().any(|(_, r)| r.policy == p);
    let has_allow = has(Policy::Allow);
    let has_ask = has(Policy::Ask);
    let has_deny = has(Policy::Deny);

    // deny > ask > allow
    let effective = if has_deny {
        Policy::Deny
    } else if has_ask {
        Policy::Ask
    } else {
        Policy::Allow
    };

    // Source = highest-precedence-scope rule carrying the winning policy.
    let source_scope = matched
        .iter()
        .find(|(_, r)| r.policy == effective)
        .map(|(s, _)| *s);

    // Explicit if a rule for the *exact* path decided it.
    let explicit = matched
        .iter()
        .any(|(_, r)| r.policy == effective && norm(&r.path) == target);

    // Conflict: allow coexists with a deny or ask that overrides it.
    let conflict = has_allow && (has_deny || has_ask);

    EffectivePolicy {
        path: target,
        effective,
        source_scope,
        explicit,
        conflict,
    }
}

/// Compute effective policy for every distinct rule path across scopes.
/// This is the "rules view" used by the Effective Preview tabs.
pub fn compute_all(rules: &ScopedRules) -> Vec<EffectivePolicy> {
    let mut seen = std::collections::BTreeSet::new();
    let mut out = Vec::new();
    for (_, r) in rules.tagged() {
        let key = norm(&r.path);
        if seen.insert(key.clone()) {
            out.push(compute_for(rules, &key));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::PolicyRule;

    fn rule(path: &str, policy: Policy, applies: AppliesTo) -> PolicyRule {
        PolicyRule::new(path, policy, applies)
    }

    #[test]
    fn allow_island_is_allowed_children_inherit() {
        let rules = ScopedRules {
            project: vec![rule("src", Policy::Allow, AppliesTo::FolderAndChildren)],
            default_mode: Some("dontAsk".into()),
            ..Default::default()
        };
        let folder = compute_for(&rules, "src");
        assert_eq!(folder.effective, Policy::Allow);
        assert!(folder.explicit);

        let child = compute_for(&rules, "src/app.ts");
        assert_eq!(child.effective, Policy::Allow);
        assert!(!child.explicit); // inherited
    }

    #[test]
    fn dont_ask_denies_unmatched_paths() {
        let rules = ScopedRules {
            project: vec![rule("src", Policy::Allow, AppliesTo::FolderAndChildren)],
            default_mode: Some("dontAsk".into()),
            ..Default::default()
        };
        let other = compute_for(&rules, "vendor/lib.js");
        assert_eq!(other.effective, Policy::Deny);
        assert_eq!(other.source_scope, None);
    }

    #[test]
    fn deny_overrides_allow_across_scopes_conflict_example() {
        // docs/effective-policy.md §4: Deny .env (user) + Allow .env (local) -> Deny, conflict
        let rules = ScopedRules {
            user: vec![rule(".env", Policy::Deny, AppliesTo::Pattern)],
            local: vec![rule(".env", Policy::Allow, AppliesTo::Pattern)],
            ..Default::default()
        };
        let r = compute_for(&rules, ".env");
        assert_eq!(r.effective, Policy::Deny);
        assert_eq!(r.source_scope, Some(Scope::User));
        assert!(r.conflict);
    }

    #[test]
    fn partial_block_more_specific_deny() {
        // Allow src/** (project) + Deny src/secret/** (local) -> src/secret denied
        let rules = ScopedRules {
            project: vec![rule("src", Policy::Allow, AppliesTo::FolderAndChildren)],
            local: vec![rule(
                "src/secret",
                Policy::Deny,
                AppliesTo::FolderAndChildren,
            )],
            ..Default::default()
        };
        assert_eq!(compute_for(&rules, "src/app.ts").effective, Policy::Allow);
        let blocked = compute_for(&rules, "src/secret/key.pem");
        assert_eq!(blocked.effective, Policy::Deny);
        assert!(blocked.conflict);
    }

    #[test]
    fn ask_when_no_deny() {
        let rules = ScopedRules {
            project: vec![
                rule("docs", Policy::Ask, AppliesTo::FolderAndChildren),
                rule("docs", Policy::Allow, AppliesTo::FolderAndChildren),
            ],
            ..Default::default()
        };
        let r = compute_for(&rules, "docs/readme.md");
        assert_eq!(r.effective, Policy::Ask);
        assert!(r.conflict); // allow + ask overlap
    }

    #[test]
    fn compute_all_one_entry_per_distinct_path() {
        let rules = ScopedRules {
            project: vec![rule("src", Policy::Allow, AppliesTo::FolderAndChildren)],
            local: vec![rule("src", Policy::Allow, AppliesTo::FolderAndChildren)],
            ..Default::default()
        };
        let all = compute_all(&rules);
        assert_eq!(all.len(), 1);
    }
}
