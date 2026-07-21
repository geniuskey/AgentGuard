//! Round-trip-safe Claude Code `settings.json` load/save (D5).
//!
//! Design (see `docs/policy-model.md` §6):
//! - Parse the whole file into `serde_json::Value`, touching only
//!   `permissions.{allow,ask,deny}`; every other key is preserved.
//! - Managed rules are re-emitted deterministically; unmanaged rules kept verbatim.
//! - The legacy `permissions.defaultMode` (deny-by-default) is no longer managed:
//!   it is stripped on write so policy is driven only by explicit path rules.
//! - On any parse/convert failure, DO NOT write — preserve the original (req §9.4).

use crate::model::{PolicyRule, Scope};
use crate::policy::{self, Permissions, UnmanagedRules};
use serde::Serialize;
use serde_json::{Map, Value};
use std::path::{Path, PathBuf};

/// Locations of the three settings files for a project (req §7).
#[derive(Debug, Clone)]
pub struct ScopePaths {
    pub user: PathBuf,
    pub project: PathBuf,
    pub local: PathBuf,
}

/// Claude Code's persisted trust decision for one project. Only these booleans
/// are surfaced; the rest of `~/.claude.json` may contain sensitive app state.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectTrustStatus {
    pub entry_found: bool,
    pub accepted: bool,
}

/// Resolve the three settings file paths for `project_root`.
pub fn scope_paths(project_root: &Path, home_dir: &Path) -> ScopePaths {
    ScopePaths {
        user: home_dir.join(".claude").join("settings.json"),
        project: project_root.join(".claude").join("settings.json"),
        local: project_root.join(".claude").join("settings.local.json"),
    }
}

fn normalized_project_key(path: &str) -> String {
    let normalized = path.replace('\\', "/");
    let trimmed = normalized.trim_end_matches('/');
    if trimmed.len() == 2 && trimmed.as_bytes()[1] == b':' {
        format!("{trimmed}/")
    } else {
        trimmed.to_string()
    }
}

fn project_keys_equal(left: &str, right: &str) -> bool {
    let left = normalized_project_key(left);
    let right = normalized_project_key(right);
    let windows_path = |s: &str| s.as_bytes().get(1) == Some(&b':');
    if cfg!(windows) || (windows_path(&left) && windows_path(&right)) {
        left.eq_ignore_ascii_case(&right)
    } else {
        left == right
    }
}

/// Extract the trust bit for `project_root` from Claude Code's application-state
/// JSON. Path separators and Windows drive-letter case are normalized.
pub fn project_trust_from_value(state: &Value, project_root: &Path) -> ProjectTrustStatus {
    let requested = project_root.to_string_lossy();
    let Some(projects) = state.get("projects").and_then(Value::as_object) else {
        return ProjectTrustStatus::default();
    };
    let Some((_, entry)) = projects
        .iter()
        .find(|(key, _)| project_keys_equal(key, &requested))
    else {
        return ProjectTrustStatus::default();
    };
    ProjectTrustStatus {
        entry_found: true,
        accepted: entry
            .get("hasTrustDialogAccepted")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    }
}

/// Read only the project trust decision from `~/.claude.json`.
pub fn project_trust_status(
    project_root: &Path,
    home_dir: &Path,
) -> crate::Result<ProjectTrustStatus> {
    let state_file = home_dir.join(".claude.json");
    if !state_file.exists() {
        return Ok(ProjectTrustStatus::default());
    }
    let state: Value = serde_json::from_str(&std::fs::read_to_string(state_file)?)?;
    Ok(project_trust_from_value(&state, project_root))
}

/// A parsed settings file: the full JSON tree plus extracted permissions.
#[derive(Debug, Clone)]
pub struct LoadedSettings {
    pub scope: Scope,
    /// Full original JSON tree (preserved for round-trip). Always an object.
    pub raw: Value,
    pub permissions: Permissions,
}

fn string_array(v: Option<&Value>) -> Vec<String> {
    match v {
        Some(Value::Array(a)) => a
            .iter()
            .filter_map(|x| x.as_str().map(|s| s.to_string()))
            .collect(),
        _ => Vec::new(),
    }
}

/// Parse settings text into [`LoadedSettings`] without losing unknown fields.
///
/// An empty/whitespace-only file is treated as `{}` (a fresh settings file).
pub fn parse(scope: Scope, text: &str) -> crate::Result<LoadedSettings> {
    let raw: Value = if text.trim().is_empty() {
        Value::Object(Map::new())
    } else {
        serde_json::from_str(text)?
    };
    if !raw.is_object() {
        return Err(crate::Error::InvalidRule(
            "settings.json root must be a JSON object".into(),
        ));
    }

    let perms_obj = raw.get("permissions").and_then(|p| p.as_object());
    let permissions = Permissions {
        allow: string_array(perms_obj.and_then(|p| p.get("allow"))),
        ask: string_array(perms_obj.and_then(|p| p.get("ask"))),
        deny: string_array(perms_obj.and_then(|p| p.get("deny"))),
    };
    Ok(LoadedSettings {
        scope,
        raw,
        permissions,
    })
}

/// Split neutral `rules` into managed permission arrays and fold the loaded file's
/// permissions to recover its unmanaged (non-file-access) rules.
///
/// Merge managed `rules` (+ preserved `unmanaged`) back into `base` JSON and return
/// the serialized text to write. Never mutates keys outside `permissions`.
///
/// Ordering within each array: managed rules (deterministic, from
/// [`policy::to_permissions`]) first, then preserved unmanaged rules verbatim.
pub fn render(
    base: &Value,
    rules: &[PolicyRule],
    unmanaged: &UnmanagedRules,
) -> crate::Result<String> {
    if !base.is_object() {
        return Err(crate::Error::InvalidRule(
            "base settings must be a JSON object".into(),
        ));
    }
    let managed = policy::to_permissions(rules);

    let mut out = base.clone();
    let obj = out.as_object_mut().expect("checked object above");

    // Build (or replace) the permissions object, preserving any sibling keys it had
    // (e.g. additionalDirectories).
    let mut perms = obj
        .get("permissions")
        .and_then(|p| p.as_object())
        .cloned()
        .unwrap_or_default();

    set_or_clear_array(&mut perms, "allow", managed.allow, &unmanaged.allow);
    set_or_clear_array(&mut perms, "ask", managed.ask, &unmanaged.ask);
    set_or_clear_array(&mut perms, "deny", managed.deny, &unmanaged.deny);

    // Deny-by-default is no longer a managed concept — strip any legacy value so
    // policy is driven only by explicit allow/ask/deny path rules.
    perms.remove("defaultMode");

    if perms.is_empty() {
        obj.remove("permissions");
    } else {
        obj.insert("permissions".into(), Value::Object(perms));
    }

    // Pretty-print with a trailing newline (matches typical editor output).
    let mut s = serde_json::to_string_pretty(&out)?;
    s.push('\n');
    Ok(s)
}

fn set_or_clear_array(
    perms: &mut Map<String, Value>,
    key: &str,
    managed: Vec<String>,
    unmanaged: &[String],
) {
    let mut combined: Vec<String> = managed;
    combined.extend(unmanaged.iter().cloned());
    if combined.is_empty() {
        perms.remove(key);
    } else {
        perms.insert(
            key.into(),
            Value::Array(combined.into_iter().map(Value::String).collect()),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{AppliesTo, Policy};

    #[test]
    fn parse_empty_is_ok() {
        let s = parse(Scope::Project, "   ").unwrap();
        assert!(s.raw.is_object());
        assert!(s.permissions.allow.is_empty());
    }

    #[test]
    fn project_trust_matches_windows_separator_and_case_variants() {
        let state = serde_json::json!({
            "projects": {
                "D:/git/GeniusKey/AgentGuard": {
                    "hasTrustDialogAccepted": true,
                    "otherSensitiveState": "not surfaced"
                }
            }
        });
        let status = project_trust_from_value(&state, Path::new(r"d:\git\geniuskey\AgentGuard\"));
        assert_eq!(
            status,
            ProjectTrustStatus {
                entry_found: true,
                accepted: true
            }
        );
    }

    #[test]
    fn project_trust_missing_or_unaccepted_is_explicit() {
        let missing = project_trust_from_value(&serde_json::json!({}), Path::new("/work/app"));
        assert_eq!(missing, ProjectTrustStatus::default());

        let state = serde_json::json!({
            "projects": { "/work/app": { "hasTrustDialogAccepted": false } }
        });
        let untrusted = project_trust_from_value(&state, Path::new("/work/app"));
        assert!(untrusted.entry_found);
        assert!(!untrusted.accepted);
    }

    #[test]
    fn parse_extracts_permissions() {
        let text = r#"{
            "permissions": {
                "defaultMode": "dontAsk",
                "allow": ["Read(./src/**)"],
                "deny": ["Read(./secrets/**)"]
            }
        }"#;
        let s = parse(Scope::Project, text).unwrap();
        assert_eq!(s.permissions.allow, vec!["Read(./src/**)"]);
        assert_eq!(s.permissions.deny, vec!["Read(./secrets/**)"]);
    }

    /// A legacy `defaultMode` present in a file is stripped on the next managed write.
    #[test]
    fn render_strips_legacy_default_mode() {
        let original = r#"{
            "permissions": {
                "defaultMode": "dontAsk",
                "allow": ["Read(./src/**)"]
            }
        }"#;
        let loaded = parse(Scope::Project, original).unwrap();
        let (rules, unmanaged) = policy::from_permissions(&loaded.permissions);
        let rendered = render(&loaded.raw, &rules, &unmanaged).unwrap();
        assert!(!rendered.contains("defaultMode"), "rendered: {rendered}");
        let reparsed = parse(Scope::Project, &rendered).unwrap();
        assert!(reparsed.raw["permissions"].get("defaultMode").is_none());
    }

    /// The DoD invariant: unknown top-level keys and non-file-access rules survive a
    /// full load -> add managed rules -> render -> re-parse cycle. (D5)
    #[test]
    fn round_trip_preserves_unknown_fields_and_unmanaged_rules() {
        let original = r#"{
            "$schema": "https://json.schemastore.org/claude-code-settings.json",
            "model": "claude-sonnet-5",
            "env": { "FOO": "bar" },
            "hooks": { "PreToolUse": [] },
            "permissions": {
                "additionalDirectories": ["~/shared"],
                "allow": ["Bash(npm run test:*)"],
                "deny": ["WebFetch(domain:evil.example)"]
            }
        }"#;

        let loaded = parse(Scope::Project, original).unwrap();
        // Recover unmanaged rules by folding what we parsed.
        let (_existing_rules, unmanaged) = policy::from_permissions(&loaded.permissions);

        // User adds two managed rules in the GUI.
        let rules = vec![
            PolicyRule::new("src", Policy::Allow, AppliesTo::FolderAndChildren),
            PolicyRule::new("secrets", Policy::Deny, AppliesTo::FolderAndChildren),
        ];

        let rendered = render(&loaded.raw, &rules, &unmanaged).unwrap();
        let reparsed = parse(Scope::Project, &rendered).unwrap();

        // (a) Unknown top-level keys preserved verbatim.
        assert_eq!(reparsed.raw.get("model").unwrap(), "claude-sonnet-5");
        assert_eq!(reparsed.raw.get("env").unwrap()["FOO"], "bar");
        assert!(reparsed.raw.get("hooks").is_some());
        assert!(reparsed.raw.get("$schema").is_some());
        // permissions.additionalDirectories preserved.
        assert_eq!(
            reparsed.raw["permissions"]["additionalDirectories"][0],
            "~/shared"
        );

        // (b) Unmanaged rules preserved.
        assert!(reparsed
            .permissions
            .allow
            .contains(&"Bash(npm run test:*)".to_string()));
        assert!(reparsed
            .permissions
            .deny
            .contains(&"WebFetch(domain:evil.example)".to_string()));

        // (c) Managed rules round-trip: folding gives back src(allow) + secrets(deny).
        let (folded, _) = policy::from_permissions(&reparsed.permissions);
        let src = folded
            .iter()
            .find(|r| r.path == "src" && r.policy == Policy::Allow)
            .expect("src allow rule present");
        assert_eq!(src.applies_to, AppliesTo::FolderAndChildren);
        assert_eq!(src.tools, None); // full toolset folded
        assert!(folded
            .iter()
            .any(|r| r.path == "secrets" && r.policy == Policy::Deny));
    }

    #[test]
    fn render_is_idempotent() {
        let base = serde_json::json!({});
        let rules = vec![PolicyRule::new(
            "src",
            Policy::Allow,
            AppliesTo::FolderAndChildren,
        )];
        let a = render(&base, &rules, &UnmanagedRules::default()).unwrap();
        let reparsed = parse(Scope::Project, &a).unwrap();
        let (folded, un) = policy::from_permissions(&reparsed.permissions);
        let b = render(&reparsed.raw, &folded, &un).unwrap();
        assert_eq!(a, b);
    }
}
