//! Round-trip-safe Claude Code `settings.json` load/save (D5).
//!
//! Design (see `docs/policy-model.md` §6):
//! - Parse the whole file into `serde_json::Value`, touching only
//!   `permissions.{allow,ask,deny,defaultMode}`; every other key is preserved.
//! - Managed rules are re-emitted deterministically; unmanaged rules kept verbatim.
//! - On any parse/convert failure, DO NOT write — preserve the original (req §9.4).

use crate::model::{PolicyRule, Scope};
use crate::policy::{self, Permissions, UnmanagedRules};
use serde_json::{Map, Value};
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
    /// Full original JSON tree (preserved for round-trip). Always an object.
    pub raw: Value,
    pub permissions: Permissions,
    pub default_mode: Option<String>,
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
    let default_mode = perms_obj
        .and_then(|p| p.get("defaultMode"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    Ok(LoadedSettings {
        scope,
        raw,
        permissions,
        default_mode,
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
    default_mode: Option<&str>,
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

    match default_mode {
        Some(m) => {
            perms.insert("defaultMode".into(), Value::String(m.to_string()));
        }
        None => {
            perms.remove("defaultMode");
        }
    }

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
        assert_eq!(s.default_mode, None);
    }

    #[test]
    fn parse_extracts_permissions_and_default_mode() {
        let text = r#"{
            "permissions": {
                "defaultMode": "dontAsk",
                "allow": ["Read(./src/**)"],
                "deny": ["Read(./secrets/**)"]
            }
        }"#;
        let s = parse(Scope::Project, text).unwrap();
        assert_eq!(s.default_mode.as_deref(), Some("dontAsk"));
        assert_eq!(s.permissions.allow, vec!["Read(./src/**)"]);
        assert_eq!(s.permissions.deny, vec!["Read(./secrets/**)"]);
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

        let rendered = render(&loaded.raw, &rules, &unmanaged, Some("dontAsk")).unwrap();
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

        // (d) defaultMode set.
        assert_eq!(reparsed.default_mode.as_deref(), Some("dontAsk"));
    }

    #[test]
    fn render_is_idempotent() {
        let base = serde_json::json!({});
        let rules = vec![PolicyRule::new(
            "src",
            Policy::Allow,
            AppliesTo::FolderAndChildren,
        )];
        let a = render(&base, &rules, &UnmanagedRules::default(), None).unwrap();
        let reparsed = parse(Scope::Project, &a).unwrap();
        let (folded, un) = policy::from_permissions(&reparsed.permissions);
        let b = render(&reparsed.raw, &folded, &un, None).unwrap();
        assert_eq!(a, b);
    }
}
