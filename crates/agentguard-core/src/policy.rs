//! Neutral model <-> Claude Code permission-rule conversion (D1).
//!
//! Claude Code keys permissions on `Tool(specifier)` strings held in three arrays
//! (`allow` / `ask` / `deny`). A path-centric [`PolicyRule`] fans out into one
//! specifier per file-access tool; parsing folds them back. See `docs/policy-model.md`
//! sections 4 (fan-out) and 5 (fold).

use crate::model::{AppliesTo, Policy, PolicyRule, Tool, FILE_ACCESS_TOOLS};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The three permission arrays of a Claude Code `settings.json`.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Permissions {
    pub allow: Vec<String>,
    pub ask: Vec<String>,
    pub deny: Vec<String>,
}

/// Rules that could not be interpreted as path rules (e.g. `Bash(...)`, `WebFetch(...)`,
/// or unparseable strings). Preserved verbatim on round-trip (D5).
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnmanagedRules {
    pub allow: Vec<String>,
    pub ask: Vec<String>,
    pub deny: Vec<String>,
}

/// Convert a relative path + `applies_to` into a Claude Code specifier.
///
/// Project/Local scope is anchored to the project root with `./`. `Pattern` is
/// used verbatim (the caller is expected to pass an already-anchored pattern).
pub fn specifier_for(path: &str, applies_to: AppliesTo) -> String {
    let p = path.trim_start_matches("./");
    match applies_to {
        AppliesTo::File => format!("./{p}"),
        AppliesTo::Folder => format!("./{p}/*"),
        AppliesTo::FolderAndChildren => format!("./{p}/**"),
        AppliesTo::Pattern => {
            if path.starts_with("./") || path.starts_with('/') || path.starts_with('~') {
                path.to_string()
            } else {
                format!("./{path}")
            }
        }
    }
}

/// Best-effort inverse of [`specifier_for`]: specifier -> (relative path, applies_to).
fn parse_specifier(spec: &str) -> (String, AppliesTo) {
    let s = spec.trim();
    if let Some(base) = s.strip_suffix("/**") {
        (strip_anchor(base), AppliesTo::FolderAndChildren)
    } else if let Some(base) = s.strip_suffix("/*") {
        (strip_anchor(base), AppliesTo::Folder)
    } else if !s.contains('*') {
        (strip_anchor(s), AppliesTo::File)
    } else {
        // Contains a glob but not a simple trailing form -> keep as raw pattern.
        (s.to_string(), AppliesTo::Pattern)
    }
}

fn strip_anchor(s: &str) -> String {
    s.trim_start_matches("./").to_string()
}

/// Which tools a rule expands to.
fn rule_tools(rule: &PolicyRule) -> Vec<Tool> {
    match &rule.tools {
        Some(t) => t.clone(),
        None => FILE_ACCESS_TOOLS.to_vec(),
    }
}

fn array_for(perms: &mut Permissions, policy: Policy) -> &mut Vec<String> {
    match policy {
        Policy::Allow => &mut perms.allow,
        Policy::Ask => &mut perms.ask,
        Policy::Deny => &mut perms.deny,
    }
}

/// Fan out neutral rules into Claude Code permission arrays (D1, §4).
///
/// Deterministic order: rules are grouped by policy, and within each array the
/// strings follow (rule input order, then [`FILE_ACCESS_TOOLS`] order) so diffs stay stable.
pub fn to_permissions(rules: &[PolicyRule]) -> Permissions {
    let mut perms = Permissions::default();
    for rule in rules {
        let spec = specifier_for(&rule.path, rule.applies_to);
        let tools = rule_tools(rule);
        // Emit in FILE_ACCESS_TOOLS order regardless of the subset's own order.
        for tool in FILE_ACCESS_TOOLS.iter() {
            if tools.contains(tool) {
                array_for(&mut perms, rule.policy).push(format!("{}({})", tool.as_str(), spec));
            }
        }
    }
    perms
}

/// Fold permission arrays back into neutral rules (D1, §5).
///
/// For each (policy, specifier) the set of matched file-access tools is collected.
/// A full [`FILE_ACCESS_TOOLS`] set folds to a single rule with `tools: None`; a
/// partial set keeps `tools: Some(subset)`. Non-file-access or unparseable rules
/// are returned in [`UnmanagedRules`] for verbatim preservation.
pub fn from_permissions(perms: &Permissions) -> (Vec<PolicyRule>, UnmanagedRules) {
    let mut rules = Vec::new();
    let mut unmanaged = UnmanagedRules::default();

    for (policy, arr, un) in [
        (Policy::Allow, &perms.allow, &mut unmanaged.allow),
        (Policy::Ask, &perms.ask, &mut unmanaged.ask),
        (Policy::Deny, &perms.deny, &mut unmanaged.deny),
    ] {
        // Group tools by specifier, preserving first-seen order of specifiers.
        let mut order: Vec<String> = Vec::new();
        let mut by_spec: BTreeMap<String, Vec<Tool>> = BTreeMap::new();

        for raw in arr {
            match parse_rule_string(raw) {
                Some((tool, spec)) => {
                    by_spec
                        .entry(spec.clone())
                        .or_insert_with(|| {
                            order.push(spec.clone());
                            Vec::new()
                        })
                        .push(tool);
                }
                None => un.push(raw.clone()),
            }
        }

        for spec in order {
            let tools = &by_spec[&spec];
            let has_all = FILE_ACCESS_TOOLS.iter().all(|t| tools.contains(t))
                && tools.len() == FILE_ACCESS_TOOLS.len();
            let (path, applies_to) = parse_specifier(&spec);
            let tool_field = if has_all {
                None
            } else {
                // Normalize subset order to FILE_ACCESS_TOOLS order.
                Some(
                    FILE_ACCESS_TOOLS
                        .iter()
                        .copied()
                        .filter(|t| tools.contains(t))
                        .collect(),
                )
            };
            rules.push(PolicyRule {
                path,
                policy,
                applies_to,
                tools: tool_field,
                reason: None,
                risk_level: None,
                notes: None,
            });
        }
    }

    (rules, unmanaged)
}

/// Parse a `Tool(specifier)` string. Returns `None` for bare tools, non-file-access
/// tools, or malformed strings (all treated as unmanaged).
fn parse_rule_string(raw: &str) -> Option<(Tool, String)> {
    let s = raw.trim();
    let open = s.find('(')?;
    if !s.ends_with(')') {
        return None;
    }
    let tool = Tool::from_str(&s[..open])?;
    let spec = &s[open + 1..s.len() - 1];
    if spec.is_empty() {
        return None;
    }
    Some((tool, spec.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::PolicyRule;

    fn fanned(path: &str, applies: AppliesTo) -> Vec<String> {
        let spec = specifier_for(path, applies);
        FILE_ACCESS_TOOLS
            .iter()
            .map(|t| format!("{}({})", t.as_str(), spec))
            .collect()
    }

    #[test]
    fn fan_out_matches_policy_model_example() {
        // docs/policy-model.md §4.3
        let rules = vec![
            PolicyRule::new("src", Policy::Allow, AppliesTo::FolderAndChildren),
            PolicyRule::new("tests", Policy::Allow, AppliesTo::FolderAndChildren),
            PolicyRule::new("docs", Policy::Ask, AppliesTo::FolderAndChildren),
            PolicyRule::new("secrets", Policy::Deny, AppliesTo::FolderAndChildren),
            PolicyRule::new(".env", Policy::Deny, AppliesTo::Pattern),
            PolicyRule::new("README.md", Policy::Allow, AppliesTo::File),
        ];
        let perms = to_permissions(&rules);

        let mut expected_allow = fanned("src", AppliesTo::FolderAndChildren);
        expected_allow.extend(fanned("tests", AppliesTo::FolderAndChildren));
        expected_allow.extend(fanned("README.md", AppliesTo::File));
        assert_eq!(perms.allow, expected_allow);

        assert_eq!(perms.ask, fanned("docs", AppliesTo::FolderAndChildren));

        let mut expected_deny = fanned("secrets", AppliesTo::FolderAndChildren);
        expected_deny.extend(fanned(".env", AppliesTo::Pattern));
        assert_eq!(perms.deny, expected_deny);

        // Spot-check exact canonical strings.
        assert!(perms.allow.contains(&"Read(./src/**)".to_string()));
        assert!(perms
            .allow
            .contains(&"NotebookEdit(./README.md)".to_string()));
        assert!(perms.deny.contains(&"Grep(./.env)".to_string()));
    }

    #[test]
    fn specifier_anchoring() {
        assert_eq!(
            specifier_for("src", AppliesTo::FolderAndChildren),
            "./src/**"
        );
        assert_eq!(specifier_for("src", AppliesTo::Folder), "./src/*");
        assert_eq!(specifier_for("README.md", AppliesTo::File), "./README.md");
        // Already-anchored input is not doubled.
        assert_eq!(
            specifier_for("./src", AppliesTo::FolderAndChildren),
            "./src/**"
        );
    }

    #[test]
    fn round_trip_full_toolset_folds_to_single_rule() {
        let rules = vec![
            PolicyRule::new("src", Policy::Allow, AppliesTo::FolderAndChildren),
            PolicyRule::new("secrets", Policy::Deny, AppliesTo::FolderAndChildren),
        ];
        let perms = to_permissions(&rules);
        let (back, unmanaged) = from_permissions(&perms);

        assert_eq!(unmanaged, UnmanagedRules::default());
        assert_eq!(back.len(), 2);
        let src = back.iter().find(|r| r.path == "src").unwrap();
        assert_eq!(src.policy, Policy::Allow);
        assert_eq!(src.applies_to, AppliesTo::FolderAndChildren);
        assert_eq!(src.tools, None); // full set folded
    }

    #[test]
    fn partial_toolset_keeps_tool_specific_rule() {
        let perms = Permissions {
            allow: vec![
                "Read(./notes/**)".to_string(),
                "Grep(./notes/**)".to_string(),
            ],
            ..Default::default()
        };
        let (rules, _) = from_permissions(&perms);
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].path, "notes");
        assert_eq!(rules[0].tools, Some(vec![Tool::Read, Tool::Grep]));
    }

    #[test]
    fn non_file_access_rules_are_preserved_as_unmanaged() {
        let perms = Permissions {
            allow: vec![
                "Bash(npm run test:*)".to_string(),
                "Read(./src/**)".to_string(),
            ],
            deny: vec!["WebFetch(domain:example.com)".to_string()],
            ..Default::default()
        };
        let (rules, unmanaged) = from_permissions(&perms);
        // Only the Read rule is managed; but as a partial set it stays tool-specific.
        assert_eq!(rules.iter().filter(|r| r.path == "src").count(), 1);
        assert_eq!(unmanaged.allow, vec!["Bash(npm run test:*)".to_string()]);
        assert_eq!(
            unmanaged.deny,
            vec!["WebFetch(domain:example.com)".to_string()]
        );
    }
}
