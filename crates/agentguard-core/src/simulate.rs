//! Policy simulator: answers "would Claude Code allow this?" for a concrete
//! path or Bash command, with the matching rules as evidence.
//!
//! Path queries reuse the effective merge (`effective.rs`) over the neutral
//! rules. Command queries evaluate raw `Bash(...)` specifiers with Claude
//! Code's prefix semantics (`Bash(npm run test:*)` = literal string prefix,
//! bare `Bash` = everything). Both fall back to `defaultMode` when nothing
//! matches (deny > ask > allow, deny wins across scopes).

use crate::effective::{self, ScopedRules};
use crate::model::{AppliesTo, Policy, Scope};
use crate::policy::{self, Permissions};
use serde::Serialize;

/// One rule that matched the query.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SimMatch {
    pub scope: Scope,
    /// Which permission list the match came from.
    pub list: Policy,
    /// Display form: the emitted specifier (paths) or the raw rule string (commands).
    pub rule: String,
    /// This rule carries the winning decision.
    pub decisive: bool,
}

/// Simulation outcome for a single query.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SimResult {
    pub query: String,
    /// `"path"` or `"command"`.
    pub kind: String,
    pub decision: Policy,
    pub matches: Vec<SimMatch>,
    /// No rule matched — `defaultMode` decided.
    pub fallback: bool,
    pub default_mode: Option<String>,
}

fn mark_decisive(matches: &mut [SimMatch], decision: Policy) {
    for m in matches.iter_mut() {
        m.decisive = m.list == decision;
    }
}

/// Simulate file access for a project-relative path against the neutral rules.
pub fn simulate_path(rules: &ScopedRules, target: &str) -> SimResult {
    let eff = effective::compute_for(rules, target);
    let mut matches: Vec<SimMatch> = effective::matching_rules(rules, target)
        .into_iter()
        .map(|(scope, r)| SimMatch {
            scope,
            list: r.policy,
            rule: display_rule(&r.path, r.applies_to),
            decisive: false,
        })
        .collect();
    let fallback = matches.is_empty();
    mark_decisive(&mut matches, eff.effective);
    SimResult {
        query: eff.path,
        kind: "path".into(),
        decision: eff.effective,
        matches,
        fallback,
        default_mode: rules.default_mode.clone(),
    }
}

fn display_rule(path: &str, applies_to: AppliesTo) -> String {
    policy::specifier_for(path, applies_to)
}

/// Does a raw permission rule string (e.g. `Bash(npm run test:*)`) match `cmd`?
fn bash_rule_matches(raw: &str, cmd: &str) -> bool {
    let s = raw.trim();
    if s == "Bash" {
        return true; // bare tool rule covers every command
    }
    let Some(spec) = s.strip_prefix("Bash(").and_then(|r| r.strip_suffix(')')) else {
        return false;
    };
    bash_specifier_matches(spec, cmd)
}

/// Claude Code Bash specifier semantics (approximation): `prefix:*` is a literal
/// string prefix, `*` matches everything, anything else is an exact match.
pub fn bash_specifier_matches(spec: &str, cmd: &str) -> bool {
    let spec = spec.trim();
    if spec == "*" {
        return true;
    }
    if let Some(prefix) = spec.strip_suffix(":*") {
        return cmd.starts_with(prefix);
    }
    cmd == spec
}

/// Simulate a Bash command against the raw permission arrays of each scope.
pub fn simulate_command(
    perms: &[(Scope, Permissions)],
    default_mode: Option<&str>,
    command: &str,
) -> SimResult {
    let cmd = command.trim();
    let mut matches = Vec::new();
    for (scope, p) in perms {
        for (list, arr) in [
            (Policy::Deny, &p.deny),
            (Policy::Ask, &p.ask),
            (Policy::Allow, &p.allow),
        ] {
            for raw in arr {
                if bash_rule_matches(raw, cmd) {
                    matches.push(SimMatch {
                        scope: *scope,
                        list,
                        rule: raw.clone(),
                        decisive: false,
                    });
                }
            }
        }
    }

    let has = |p: Policy| matches.iter().any(|m| m.list == p);
    let (decision, fallback) = if has(Policy::Deny) {
        (Policy::Deny, false)
    } else if has(Policy::Ask) {
        (Policy::Ask, false)
    } else if has(Policy::Allow) {
        (Policy::Allow, false)
    } else {
        (effective::fallback(default_mode), true)
    };
    mark_decisive(&mut matches, decision);

    SimResult {
        query: cmd.to_string(),
        kind: "command".into(),
        decision,
        matches,
        fallback,
        default_mode: default_mode.map(|s| s.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::PolicyRule;

    fn perms(allow: &[&str], ask: &[&str], deny: &[&str]) -> Permissions {
        Permissions {
            allow: allow.iter().map(|s| s.to_string()).collect(),
            ask: ask.iter().map(|s| s.to_string()).collect(),
            deny: deny.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn command_exact_and_prefix_matching() {
        let scopes = vec![(
            Scope::Project,
            perms(&["Bash(npm run test:*)", "Bash(git status)"], &[], &[]),
        )];

        let r = simulate_command(&scopes, None, "npm run test -- --watch");
        assert_eq!(r.decision, Policy::Allow);
        assert_eq!(r.matches.len(), 1);
        assert!(r.matches[0].decisive);

        let r = simulate_command(&scopes, None, "git status");
        assert_eq!(r.decision, Policy::Allow);

        // Exact rule does not cover arguments.
        let r = simulate_command(&scopes, None, "git status --short");
        assert!(r.fallback);
        assert_eq!(r.decision, Policy::Ask); // no defaultMode -> prompt
    }

    #[test]
    fn command_deny_wins_across_scopes() {
        let scopes = vec![
            (Scope::User, perms(&[], &[], &["Bash(curl:*)"])),
            (Scope::Local, perms(&["Bash(curl:*)"], &[], &[])),
        ];
        let r = simulate_command(&scopes, None, "curl https://example.com");
        assert_eq!(r.decision, Policy::Deny);
        assert_eq!(r.matches.len(), 2);
        let deny = r.matches.iter().find(|m| m.list == Policy::Deny).unwrap();
        assert!(deny.decisive);
        let allow = r.matches.iter().find(|m| m.list == Policy::Allow).unwrap();
        assert!(!allow.decisive);
    }

    #[test]
    fn bare_bash_rule_matches_everything() {
        let scopes = vec![(Scope::Project, perms(&[], &["Bash"], &[]))];
        let r = simulate_command(&scopes, None, "anything at all");
        assert_eq!(r.decision, Policy::Ask);
        assert!(!r.fallback);
    }

    #[test]
    fn command_fallback_follows_default_mode() {
        let scopes = vec![(Scope::Project, perms(&[], &[], &[]))];
        let r = simulate_command(&scopes, Some("dontAsk"), "rm -rf /");
        assert!(r.fallback);
        assert_eq!(r.decision, Policy::Deny);

        let r = simulate_command(&scopes, None, "ls");
        assert!(r.fallback);
        assert_eq!(r.decision, Policy::Ask);
    }

    #[test]
    fn non_bash_rules_never_match_commands() {
        let scopes = vec![(
            Scope::Project,
            perms(&["Read(./src/**)"], &[], &["WebFetch(domain:evil.example)"]),
        )];
        let r = simulate_command(&scopes, None, "cat src/app.ts");
        assert!(r.fallback);
        assert!(r.matches.is_empty());
    }

    #[test]
    fn path_simulation_reports_matches_and_decision() {
        let rules = ScopedRules {
            project: vec![PolicyRule::new(
                "src",
                Policy::Allow,
                AppliesTo::FolderAndChildren,
            )],
            local: vec![PolicyRule::new(
                "src/secret",
                Policy::Deny,
                AppliesTo::FolderAndChildren,
            )],
            default_mode: Some("dontAsk".into()),
            ..Default::default()
        };

        let r = simulate_path(&rules, "src/secret/key.pem");
        assert_eq!(r.decision, Policy::Deny);
        assert_eq!(r.matches.len(), 2);
        let deny = r.matches.iter().find(|m| m.list == Policy::Deny).unwrap();
        assert_eq!(deny.scope, Scope::Local);
        assert_eq!(deny.rule, "./src/secret/**");
        assert!(deny.decisive);

        // Unmatched path falls back to Default Deny.
        let r = simulate_path(&rules, "vendor/lib.js");
        assert!(r.fallback);
        assert_eq!(r.decision, Policy::Deny);
        assert_eq!(r.default_mode.as_deref(), Some("dontAsk"));
    }
}
