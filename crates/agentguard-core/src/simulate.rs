//! Compare explicit rules for supported paths and simple shell commands.
//! Unsupported syntax and unmatched inputs have no decision. These comparisons
//! never predict runtime approval (modes, trust, CLI flags, hooks, sandbox).

use crate::effective::{self, MatchContext, ScopedRules};
use crate::model::{AppliesTo, Policy, Scope, Tool};
use crate::policy::{self, Permissions};
use serde::Serialize;

/// Shell tool whose permission rules should be evaluated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellTool {
    Bash,
    PowerShell,
}

impl ShellTool {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Bash => "Bash",
            Self::PowerShell => "PowerShell",
        }
    }

    fn case_insensitive(self) -> bool {
        matches!(self, Self::PowerShell)
    }
}

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
    pub decision: Option<Policy>,
    pub notes: Vec<String>,
    pub matches: Vec<SimMatch>,
    /// No explicit rule matched; runtime approval has not been determined.
    pub fallback: bool,
}

fn mark_decisive(matches: &mut [SimMatch], decision: Policy) {
    for m in matches.iter_mut() {
        m.decisive = m.list == decision;
    }
}

/// Simulate file access for a project-relative path against the neutral rules.
pub fn simulate_path(rules: &ScopedRules, target: &str) -> SimResult {
    simulate_path_for(rules, target, Tool::Read, &MatchContext::default())
}

pub fn simulate_path_for(
    rules: &ScopedRules,
    target: &str,
    tool: Tool,
    context: &MatchContext,
) -> SimResult {
    let eff = effective::compute_for_tool(rules, target, tool, context);
    let mut matches: Vec<SimMatch> = effective::matching_rules_for(rules, target, tool, context)
        .into_iter()
        .map(|(scope, r)| SimMatch {
            scope,
            list: r.policy,
            rule: display_rule(&r.path, r.applies_to),
            decisive: false,
        })
        .collect();
    let fallback = matches.is_empty();
    if let Some(decision) = eff.effective {
        mark_decisive(&mut matches, decision);
    }
    SimResult {
        query: eff.path,
        kind: "path".into(),
        decision: eff.effective,
        notes: vec![if eff.incomplete {
            "컨텍스트가 없거나 지원하지 않는 경로 패턴이 있어 판정할 수 없습니다."
        } else {
            "명시된 경로 규칙만 비교했습니다. 실행 모드·신뢰·CLI 설정·심볼릭 링크·훅에 따른 실제 승인 결과는 다를 수 있습니다."
        }.into()],
        matches,
        fallback,
    }
}

fn display_rule(path: &str, applies_to: AppliesTo) -> String {
    policy::specifier_for(path, applies_to)
}

/// Does a raw permission rule string match `cmd` for the selected shell tool?
fn shell_rule_matches(raw: &str, cmd: &str, tool: ShellTool) -> bool {
    let s = raw.trim();
    let name = tool.as_str();
    if s == name {
        return true; // bare tool rule covers every command
    }
    let prefix = format!("{name}(");
    let Some(spec) = s.strip_prefix(&prefix).and_then(|r| r.strip_suffix(')')) else {
        return false;
    };
    shell_specifier_matches(spec, cmd, tool.case_insensitive())
}

/// Match Claude Code shell-rule wildcards. `*` may occur anywhere and spans spaces.
/// A trailing ` *` (or its `:*` alias) also matches the prefix with no arguments.
fn shell_specifier_matches(spec: &str, cmd: &str, case_insensitive: bool) -> bool {
    let mut pattern = spec.trim().to_string();
    let mut command = cmd.trim().to_string();
    if case_insensitive {
        pattern = pattern.to_lowercase();
        command = command.to_lowercase();
    }
    if pattern == "*" {
        return true;
    }

    if let Some(prefix) = pattern.strip_suffix(":*") {
        return command == prefix
            || command
                .strip_prefix(prefix)
                .is_some_and(|rest| rest.chars().next().is_some_and(char::is_whitespace));
    }

    // Claude treats `cmd *` as `cmd` or `cmd <anything>`; a normal glob matcher
    // would miss the zero-argument form because of the literal space.
    if let Some(prefix_pattern) = pattern.strip_suffix(" *") {
        if wildcard_match(prefix_pattern, &command) {
            return true;
        }
    }

    wildcard_match(&pattern, &command)
}

/// Small `*`-only glob matcher. Claude command rules do not give `?` or character
/// classes special meaning, so keeping this local is less surprising than a
/// filesystem glob implementation.
fn wildcard_match(pattern: &str, text: &str) -> bool {
    let pattern: Vec<char> = pattern.chars().collect();
    let text: Vec<char> = text.chars().collect();
    let (mut p, mut t) = (0, 0);
    let (mut star, mut retry_t) = (None, 0);

    while t < text.len() {
        if p < pattern.len() && pattern[p] == text[t] {
            p += 1;
            t += 1;
        } else if p < pattern.len() && pattern[p] == '*' {
            star = Some(p);
            p += 1;
            retry_t = t;
        } else if let Some(star_pos) = star {
            p = star_pos + 1;
            retry_t += 1;
            t = retry_t;
        } else {
            return false;
        }
    }

    while p < pattern.len() && pattern[p] == '*' {
        p += 1;
    }
    p == pattern.len()
}

/// Public helpers retained for callers/tests that need one shell's semantics.
pub fn bash_specifier_matches(spec: &str, cmd: &str) -> bool {
    shell_specifier_matches(spec, cmd, false)
}

pub fn powershell_specifier_matches(spec: &str, cmd: &str) -> bool {
    shell_specifier_matches(spec, cmd, true)
}

/// Simulate a shell command against the raw permission arrays of each scope.
pub fn simulate_command(
    perms: &[(Scope, Permissions)],
    command: &str,
    tool: ShellTool,
) -> SimResult {
    let cmd = command.trim();
    // A wildcard is not a shell parser. Never approve compound commands,
    // substitutions, quoted invocations, redirections or wrapper/alias forms.
    let first = cmd
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    let unsupported = cmd.contains([
        ';', '|', '&', '\n', '\r', '`', '$', '<', '>', '(', ')', '\'', '"',
    ]) || matches!(
        first.as_str(),
        "timeout"
            | "time"
            | "nice"
            | "nohup"
            | "stdbuf"
            | "command"
            | "builtin"
            | "noglob"
            | "xargs"
            | "watch"
            | "setsid"
            | "ionice"
            | "flock"
            | "find"
    ) || (tool == ShellTool::PowerShell
        && matches!(
            first.as_str(),
            "gci"
                | "ls"
                | "dir"
                | "cat"
                | "gc"
                | "type"
                | "rm"
                | "del"
                | "erase"
                | "ri"
                | "curl"
                | "wget"
                | "iwr"
                | "irm"
                | "saps"
                | "start"
        ))
        || first.contains('=');
    if unsupported {
        return SimResult { query: cmd.into(), kind: "command".into(), decision: None,
            matches: Vec::new(), fallback: false,
            notes: vec!["복합 명령·인용·치환·리다이렉션·래퍼·별칭은 이 비교기에서 판정하지 않습니다. 실제 Claude Code에서 확인하세요.".into()] };
    }
    let mut matches = Vec::new();
    for (scope, p) in perms {
        for (list, arr) in [
            (Policy::Deny, &p.deny),
            (Policy::Ask, &p.ask),
            (Policy::Allow, &p.allow),
        ] {
            for raw in arr {
                if shell_rule_matches(raw, cmd, tool) {
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
        (Some(Policy::Deny), false)
    } else if has(Policy::Ask) {
        (Some(Policy::Ask), false)
    } else if has(Policy::Allow) {
        (Some(Policy::Allow), false)
    } else {
        (None, true)
    };
    if let Some(decision) = decision {
        mark_decisive(&mut matches, decision);
    }

    SimResult {
        query: cmd.to_string(),
        kind: "command".into(),
        decision,
        notes: vec!["저장된 명시 규칙의 단순 문자열 비교입니다. 실행 모드·신뢰·샌드박스·훅·CLI 설정에 따른 실제 승인은 예측하지 않습니다.".into()],
        matches,
        fallback,
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

        let r = simulate_command(&scopes, "npm run test -- --watch", ShellTool::Bash);
        assert_eq!(r.decision, Some(Policy::Allow));
        assert_eq!(r.matches.len(), 1);
        assert!(r.matches[0].decisive);

        let r = simulate_command(&scopes, "git status", ShellTool::Bash);
        assert_eq!(r.decision, Some(Policy::Allow));

        // Exact rule does not cover arguments.
        let r = simulate_command(&scopes, "git status --short", ShellTool::Bash);
        assert!(r.fallback);
        assert_eq!(r.decision, None); // no explicit rule
    }

    #[test]
    fn command_deny_wins_across_scopes() {
        let scopes = vec![
            (Scope::User, perms(&[], &[], &["Bash(curl:*)"])),
            (Scope::Local, perms(&["Bash(curl:*)"], &[], &[])),
        ];
        let r = simulate_command(&scopes, "curl https://example.com", ShellTool::Bash);
        assert_eq!(r.decision, Some(Policy::Deny));
        assert_eq!(r.matches.len(), 2);
        let deny = r.matches.iter().find(|m| m.list == Policy::Deny).unwrap();
        assert!(deny.decisive);
        let allow = r.matches.iter().find(|m| m.list == Policy::Allow).unwrap();
        assert!(!allow.decisive);
    }

    #[test]
    fn bare_bash_rule_matches_everything() {
        let scopes = vec![(Scope::Project, perms(&[], &["Bash"], &[]))];
        let r = simulate_command(&scopes, "anything at all", ShellTool::Bash);
        assert_eq!(r.decision, Some(Policy::Ask));
        assert!(!r.fallback);
    }

    #[test]
    fn unmatched_commands_have_no_explicit_decision() {
        let scopes = vec![(Scope::Project, perms(&[], &[], &[]))];
        let r = simulate_command(&scopes, "rm -rf /", ShellTool::Bash);
        assert!(r.fallback);
        assert_eq!(r.decision, None);

        let r = simulate_command(&scopes, "ls", ShellTool::Bash);
        assert!(r.fallback);
        assert_eq!(r.decision, None);
    }

    #[test]
    fn non_bash_rules_never_match_commands() {
        let scopes = vec![(
            Scope::Project,
            perms(&["Read(./src/**)"], &[], &["WebFetch(domain:evil.example)"]),
        )];
        let r = simulate_command(&scopes, "cat src/app.ts", ShellTool::Bash);
        assert!(r.fallback);
        assert!(r.matches.is_empty());
    }

    #[test]
    fn wildcard_matches_actual_claude_examples() {
        assert!(bash_specifier_matches("npm run *", "npm run"));
        assert!(bash_specifier_matches("npm run *", "npm run check"));
        assert!(!bash_specifier_matches("npm run *", "npm runner"));
        assert!(bash_specifier_matches("git * main", "git push origin main"));
        assert!(bash_specifier_matches("* --version", "cargo --version"));
        assert!(bash_specifier_matches("npm test:*", "npm test -- --watch"));
        assert!(!bash_specifier_matches("npm test:*", "npm testing"));
    }

    #[test]
    fn powershell_rules_are_case_insensitive_and_tool_specific() {
        let scopes = vec![(
            Scope::Local,
            perms(
                &["PowerShell(npm run *)"],
                &[],
                &["Bash(npm run dangerous)"],
            ),
        )];

        let ps = simulate_command(&scopes, "NPM RUN CHECK", ShellTool::PowerShell);
        assert_eq!(ps.decision, Some(Policy::Allow));
        assert_eq!(ps.matches[0].rule, "PowerShell(npm run *)");

        let bash = simulate_command(&scopes, "npm run check", ShellTool::Bash);
        assert!(bash.fallback);
        assert_eq!(bash.decision, None);
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
            ..Default::default()
        };

        let r = simulate_path(&rules, "src/secret/key.pem");
        assert_eq!(r.decision, Some(Policy::Deny));
        assert_eq!(r.matches.len(), 2);
        let deny = r.matches.iter().find(|m| m.list == Policy::Deny).unwrap();
        assert_eq!(deny.scope, Scope::Local);
        assert_eq!(deny.rule, "./src/secret/**");
        assert!(deny.decisive);

        // Unmatched path falls back to Claude Code's prompt behavior.
        let r = simulate_path(&rules, "vendor/lib.js");
        assert!(r.fallback);
        assert_eq!(r.decision, None);
    }

    #[test]
    fn complex_commands_never_get_a_false_allow() {
        let scopes = vec![(
            Scope::Project,
            perms(
                &["Bash(npm *)", "PowerShell(npm *)"],
                &[],
                &["Bash(curl *)"],
            ),
        )];
        for cmd in [
            "npm test && curl https://example.com",
            "npm test; curl x",
            "npm test | curl x",
            "npm test $(curl x)",
            "npm test > result.txt",
            "timeout 30 npm test",
        ] {
            let result = simulate_command(&scopes, cmd, ShellTool::Bash);
            assert_eq!(result.decision, None, "{cmd}");
            assert!(!result.notes.is_empty());
        }
        assert_eq!(
            simulate_command(&scopes, "iwr https://example.com", ShellTool::PowerShell).decision,
            None
        );
    }
}
