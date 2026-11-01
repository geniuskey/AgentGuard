//! Comparison of explicit path rules, not a prediction of runtime approval.
//! Runtime modes, trust, hooks, CLI flags and symlink resolution are not modeled.
use crate::model::{AppliesTo, Policy, PolicyRule, Scope, Tool, FILE_ACCESS_TOOLS};
use globset::GlobBuilder;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone)]
pub struct ScopedRules {
    pub managed: Vec<PolicyRule>,
    pub user: Vec<PolicyRule>,
    pub project: Vec<PolicyRule>,
    pub local: Vec<PolicyRule>,
}

impl ScopedRules {
    fn tagged(&self) -> Vec<(Scope, &PolicyRule)> {
        let mut result = Vec::new();
        result.extend(self.managed.iter().map(|r| (Scope::Managed, r)));
        result.extend(self.local.iter().map(|r| (Scope::Local, r)));
        result.extend(self.project.iter().map(|r| (Scope::Project, r)));
        result.extend(self.user.iter().map(|r| (Scope::User, r)));
        result
    }
}

#[derive(Debug, Default, Clone)]
pub struct MatchContext {
    pub project_root: String,
    pub home: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EffectivePolicy {
    pub path: String,
    pub tool: Tool,
    pub effective: Option<Policy>,
    pub source_scope: Option<Scope>,
    pub explicit: bool,
    pub conflict: bool,
    pub incomplete: bool,
}

fn norm(path: &str) -> String {
    path.strip_prefix("./").unwrap_or(path).replace('\\', "/")
}

fn absolute(path: &str) -> String {
    let p = path.replace('\\', "/");
    let bytes = p.as_bytes();
    if bytes.len() >= 2 && bytes[1] == b':' {
        format!(
            "/{}/{}",
            (bytes[0] as char).to_ascii_lowercase(),
            p[2..].trim_start_matches('/')
        )
    } else {
        p
    }
}

fn join(base: &str, path: &str) -> String {
    if base.is_empty() {
        path.to_string()
    } else {
        format!("{}/{}", absolute(base).trim_end_matches('/'), path)
    }
}

fn target_path(path: &str, context: &MatchContext) -> Option<String> {
    // Lexical glob comparison cannot safely resolve traversal or filesystem links.
    if path.replace('\\', "/").split('/').any(|part| part == "..") {
        return None;
    }
    if let Some(rest) = path.strip_prefix("~/") {
        return (!context.home.is_empty()).then(|| join(&context.home, rest));
    }
    if let Some(rest) = path.strip_prefix("//") {
        return Some(format!("/{rest}"));
    }
    if path.starts_with('/') || path.as_bytes().get(1) == Some(&b':') {
        return Some(absolute(path));
    }
    Some(join(&context.project_root, &norm(path)))
}

fn supports_tool(rule: &PolicyRule, tool: Tool) -> bool {
    rule.tools.as_ref().is_none_or(|tools| {
        tools.contains(&tool)
            || (tool == Tool::Edit && rule.policy == Policy::Deny && tools.contains(&Tool::Read))
    })
}

fn pattern_for(rule: &PolicyRule, scope: Scope, context: &MatchContext) -> Option<String> {
    let path = norm(&rule.path);
    // Preserve uncertainty rather than inventing gitignore negation/class semantics.
    if path.contains(['!', '[', ']', '{', '}', '?']) {
        return None;
    }
    let suffix = match rule.applies_to {
        AppliesTo::File | AppliesTo::Pattern => path.clone(),
        AppliesTo::Folder => format!("{path}/*"),
        AppliesTo::FolderAndChildren => format!("{path}/**"),
    };
    if let Some(rest) = suffix.strip_prefix("~/") {
        return (!context.home.is_empty()).then(|| join(&context.home, rest));
    }
    if let Some(rest) = suffix.strip_prefix("//") {
        return Some(format!("/{rest}"));
    }
    if let Some(rest) = suffix.strip_prefix('/') {
        return match scope {
            Scope::User if !context.home.is_empty() => {
                Some(join(&join(&context.home, ".claude"), rest))
            }
            Scope::Project | Scope::Local if !context.project_root.is_empty() => {
                Some(join(&context.project_root, rest))
            }
            _ => None,
        };
    }
    let single_file = !suffix.contains('/');
    let single_dir = path.split('/').count() == 1
        && matches!(
            rule.applies_to,
            AppliesTo::Folder | AppliesTo::FolderAndChildren
        );
    // gitignore bare filenames match at any depth. Single directory deny/ask
    // patterns also reach nested copies; allow patterns do not.
    let recursive = single_file || (single_dir && rule.policy != Policy::Allow);
    Some(join(
        &context.project_root,
        &if recursive {
            format!("**/{suffix}")
        } else {
            suffix
        },
    ))
}

fn rule_matches(
    rule: &PolicyRule,
    scope: Scope,
    target: &str,
    context: &MatchContext,
) -> Option<bool> {
    let pattern = pattern_for(rule, scope, context)?;
    let resolved = target_path(target, context)?;
    let windows = context.project_root.as_bytes().get(1) == Some(&b':')
        || context.home.as_bytes().get(1) == Some(&b':');
    let matcher = GlobBuilder::new(&pattern)
        .literal_separator(true)
        .case_insensitive(windows)
        .build()
        .ok()?
        .compile_matcher();
    let same_folder = matches!(
        rule.applies_to,
        AppliesTo::Folder | AppliesTo::FolderAndChildren
    ) && norm(&rule.path) == norm(target);
    Some(same_folder || matcher.is_match(resolved))
}

pub fn matching_rules_for(
    rules: &ScopedRules,
    target: &str,
    tool: Tool,
    context: &MatchContext,
) -> Vec<(Scope, PolicyRule)> {
    rules
        .tagged()
        .into_iter()
        .filter(|(scope, rule)| {
            supports_tool(rule, tool) && rule_matches(rule, *scope, target, context) == Some(true)
        })
        .map(|(scope, rule)| (scope, rule.clone()))
        .collect()
}

pub fn compute_for_tool(
    rules: &ScopedRules,
    target: &str,
    tool: Tool,
    context: &MatchContext,
) -> EffectivePolicy {
    let matched = matching_rules_for(rules, target, tool, context);
    let incomplete = rules.tagged().iter().any(|(scope, rule)| {
        supports_tool(rule, tool) && rule_matches(rule, *scope, target, context).is_none()
    });
    let has = |policy| matched.iter().any(|(_, rule)| rule.policy == policy);
    let effective = if incomplete {
        None
    } else if has(Policy::Deny) {
        Some(Policy::Deny)
    } else if has(Policy::Ask) {
        Some(Policy::Ask)
    } else if has(Policy::Allow) {
        Some(Policy::Allow)
    } else {
        None
    };
    EffectivePolicy {
        path: norm(target),
        tool,
        effective,
        incomplete,
        source_scope: matched
            .iter()
            .find(|(_, rule)| Some(rule.policy) == effective)
            .map(|(scope, _)| *scope),
        explicit: matched
            .iter()
            .any(|(_, rule)| Some(rule.policy) == effective && norm(&rule.path) == norm(target)),
        conflict: has(Policy::Allow) && (has(Policy::Ask) || has(Policy::Deny)),
    }
}

pub fn compute_for(rules: &ScopedRules, target: &str) -> EffectivePolicy {
    compute_for_tool(rules, target, Tool::Read, &MatchContext::default())
}

pub fn compute_all_in(rules: &ScopedRules, context: &MatchContext) -> Vec<EffectivePolicy> {
    let mut seen = std::collections::BTreeSet::new();
    let mut out = Vec::new();
    for (_, rule) in rules.tagged() {
        let key = norm(&rule.path);
        if seen.insert(key.clone()) {
            for tool in FILE_ACCESS_TOOLS {
                out.push(compute_for_tool(rules, &key, tool, context));
            }
        }
    }
    out
}

pub fn compute_all(rules: &ScopedRules) -> Vec<EffectivePolicy> {
    compute_all_in(rules, &MatchContext::default())
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
            ..Default::default()
        };
        let folder = compute_for(&rules, "src");
        assert_eq!(folder.effective, Some(Policy::Allow));
        assert!(folder.explicit);

        let child = compute_for(&rules, "src/app.ts");
        assert_eq!(child.effective, Some(Policy::Allow));
        assert!(!child.explicit); // inherited
    }

    #[test]
    fn unmatched_paths_have_no_explicit_decision() {
        let rules = ScopedRules {
            project: vec![rule("src", Policy::Allow, AppliesTo::FolderAndChildren)],
            ..Default::default()
        };
        let other = compute_for(&rules, "vendor/lib.js");
        assert_eq!(other.effective, None);
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
        assert_eq!(r.effective, Some(Policy::Deny));
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
        assert_eq!(
            compute_for(&rules, "src/app.ts").effective,
            Some(Policy::Allow)
        );
        let blocked = compute_for(&rules, "src/secret/key.pem");
        assert_eq!(blocked.effective, Some(Policy::Deny));
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
        assert_eq!(r.effective, Some(Policy::Ask));
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
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn managed_is_highest_source_authority_but_global_deny_still_wins() {
        let rules = ScopedRules {
            managed: vec![rule("src", Policy::Allow, AppliesTo::FolderAndChildren)],
            local: vec![
                rule("src", Policy::Allow, AppliesTo::FolderAndChildren),
                rule("src/private", Policy::Deny, AppliesTo::FolderAndChildren),
            ],
            ..Default::default()
        };

        let allowed = compute_for(&rules, "src/app.rs");
        assert_eq!(allowed.effective, Some(Policy::Allow));
        assert_eq!(allowed.source_scope, Some(Scope::Managed));

        let denied = compute_for(&rules, "src/private/key.txt");
        assert_eq!(denied.effective, Some(Policy::Deny));
        assert_eq!(denied.source_scope, Some(Scope::Local));
        assert!(denied.conflict);
    }

    #[test]
    fn tool_and_home_context_are_respected() {
        let mut edit = rule("src", Policy::Deny, AppliesTo::FolderAndChildren);
        edit.tools = Some(vec![Tool::Edit]);
        let rules = ScopedRules {
            project: vec![edit],
            user: vec![rule("~/repo/secrets/**", Policy::Deny, AppliesTo::Pattern)],
            ..Default::default()
        };
        let context = MatchContext {
            project_root: "C:/Users/u/repo".into(),
            home: "C:/Users/u".into(),
        };
        assert_eq!(
            compute_for_tool(&rules, "src/app.ts", Tool::Read, &context).effective,
            None
        );
        assert_eq!(
            compute_for_tool(&rules, "src/app.ts", Tool::Edit, &context).effective,
            Some(Policy::Deny)
        );
        assert_eq!(
            compute_for_tool(&rules, "secrets/key", Tool::Read, &context).effective,
            Some(Policy::Deny)
        );
    }

    #[test]
    fn direct_children_do_not_match_grandchildren() {
        let rules = ScopedRules {
            project: vec![rule("docs", Policy::Allow, AppliesTo::Folder)],
            ..Default::default()
        };
        assert_eq!(
            compute_for(&rules, "docs/a.md").effective,
            Some(Policy::Allow)
        );
        assert_eq!(compute_for(&rules, "docs/sub/a.md").effective, None);
    }

    #[test]
    fn unsupported_patterns_and_missing_context_remain_unknown() {
        let rules = ScopedRules {
            user: vec![rule("~/.ssh/**", Policy::Deny, AppliesTo::Pattern)],
            ..Default::default()
        };
        assert!(compute_for(&rules, "src/a").incomplete);
        assert_eq!(compute_for(&rules, "src/a").effective, None);
        let rules = ScopedRules {
            project: vec![rule("!sample.env", Policy::Deny, AppliesTo::Pattern)],
            ..Default::default()
        };
        assert!(compute_for(&rules, "sample.env").incomplete);
    }

    #[test]
    fn traversal_is_not_treated_as_an_allowed_child() {
        let rules = ScopedRules {
            project: vec![rule("src", Policy::Allow, AppliesTo::FolderAndChildren)],
            ..Default::default()
        };
        let result = compute_for(&rules, "src/../secrets/key");
        assert!(result.incomplete);
        assert_eq!(result.effective, None);
    }
}
