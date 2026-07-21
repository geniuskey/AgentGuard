//! Lightweight lint for Claude Code `settings.json` beyond the permissions block.
//!
//! Checks the *known* top-level keys against the official JSON Schema
//! (https://www.schemastore.org/claude-code-settings.json, fetched 2026-07) at the
//! type/enum level, and warns when `env` values look like plaintext secrets.
//! Unknown keys are reported as info only — new Claude Code versions add keys
//! faster than we ship, and unknown keys are harmless (Claude Code ignores them).
//! Full JSON-Schema validation is a backlog item (docs/claude-code-settings-plan.md).

use serde::Serialize;
use serde_json::Value;

/// Severity of one lint finding. `Warn` = probably a mistake, `Info` = FYI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LintLevel {
    Warn,
    Info,
}

/// One lint finding on a settings document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LintItem {
    pub level: LintLevel,
    /// Dotted key path (`"env.AWS_SECRET_ACCESS_KEY"`, `"cleanupPeriodDays"`).
    pub path: String,
    pub message: String,
}

fn warn(path: &str, message: impl Into<String>) -> LintItem {
    LintItem {
        level: LintLevel::Warn,
        path: path.to_string(),
        message: message.into(),
    }
}

fn info(path: &str, message: impl Into<String>) -> LintItem {
    LintItem {
        level: LintLevel::Info,
        path: path.to_string(),
        message: message.into(),
    }
}

/// Expected shape of a known top-level key.
enum Expect {
    Bool,
    /// Integer with an optional minimum.
    Int(Option<i64>),
    Str,
    StrArray,
    Object,
    /// String restricted to these values.
    Enum(&'static [&'static str]),
    /// Several shapes are valid across Claude Code versions — never warn.
    Any,
}

/// Known top-level keys (official schema). `permissions`/`env`/`hooks` get deeper
/// checks below; the rest are type-checked only.
const KNOWN_KEYS: &[(&str, Expect)] = &[
    ("$schema", Expect::Str),
    ("model", Expect::Str),
    ("availableModels", Expect::StrArray),
    (
        "effortLevel",
        Expect::Enum(&["low", "medium", "high", "xhigh"]),
    ),
    ("alwaysThinkingEnabled", Expect::Bool),
    ("outputStyle", Expect::Str),
    ("language", Expect::Str),
    ("includeCoAuthoredBy", Expect::Bool),
    ("includeGitInstructions", Expect::Bool),
    ("attribution", Expect::Object),
    ("cleanupPeriodDays", Expect::Int(Some(1))),
    ("autoUpdates", Expect::Bool),
    ("autoUpdatesChannel", Expect::Enum(&["stable", "latest"])),
    ("autoCompactEnabled", Expect::Bool),
    ("autoMemoryEnabled", Expect::Bool),
    ("fileCheckpointingEnabled", Expect::Bool),
    ("respectGitignore", Expect::Bool),
    ("spinnerTips", Expect::Any), // bool historically, array in current schema
    ("statusLine", Expect::Any),  // string in current schema, object in older versions
    ("apiKeyHelper", Expect::Str),
    ("forceLoginMethod", Expect::Str),
    ("enableAllProjectMcpServers", Expect::Bool),
    ("enabledMcpjsonServers", Expect::StrArray),
    ("disabledMcpjsonServers", Expect::StrArray),
    ("plansDirectory", Expect::Str),
    ("claudeMdExcludes", Expect::StrArray),
    ("env", Expect::Object),
    ("permissions", Expect::Object),
    ("hooks", Expect::Object),
    ("mcpServers", Expect::Object),
    ("modelOverrides", Expect::Object),
];

const DEFAULT_MODES: &[&str] = &[
    "default",
    "acceptEdits",
    "plan",
    "auto",
    "dontAsk",
    "delegate",
    "bypassPermissions",
];

/// Env-var name fragments whose values are credentials.
const SECRET_NAME_HINTS: &[&str] = &["KEY", "TOKEN", "SECRET", "PASSWORD", "CREDENTIAL"];

/// Value prefixes that are recognizable credential formats regardless of the name.
const SECRET_VALUE_PREFIXES: &[&str] = &["sk-", "ghp_", "github_pat_", "AKIA", "xoxb-", "xoxp-"];

fn type_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn is_string_array(v: &Value) -> bool {
    v.as_array()
        .map(|a| a.iter().all(|x| x.is_string()))
        .unwrap_or(false)
}

fn check_known(key: &str, expect: &Expect, v: &Value, out: &mut Vec<LintItem>) {
    match expect {
        Expect::Bool if !v.is_boolean() => {
            out.push(warn(
                key,
                format!("true/false여야 합니다 (현재: {})", type_name(v)),
            ));
        }
        Expect::Int(min) => match v.as_i64() {
            Some(n) => {
                if let Some(m) = min {
                    if n < *m {
                        out.push(warn(key, format!("{m} 이상의 정수여야 합니다 (현재: {n})")));
                    }
                }
            }
            None => out.push(warn(
                key,
                format!("정수여야 합니다 (현재: {})", type_name(v)),
            )),
        },
        Expect::Str if !v.is_string() => {
            out.push(warn(
                key,
                format!("문자열이어야 합니다 (현재: {})", type_name(v)),
            ));
        }
        Expect::StrArray if !is_string_array(v) => {
            out.push(warn(key, "문자열 배열이어야 합니다".to_string()));
        }
        Expect::Object if !v.is_object() => {
            out.push(warn(
                key,
                format!("객체여야 합니다 (현재: {})", type_name(v)),
            ));
        }
        Expect::Enum(allowed) => match v.as_str() {
            Some(s) if allowed.contains(&s) => {}
            Some(s) => out.push(warn(
                key,
                format!("허용값: {} (현재: \"{s}\")", allowed.join(" | ")),
            )),
            None => out.push(warn(key, "문자열이어야 합니다".to_string())),
        },
        _ => {}
    }
}

fn secret_looking(name: &str, value: &str) -> bool {
    if value.is_empty() {
        return false;
    }
    // `$VAR` / `${VAR}` references another variable — not an inline secret.
    if value.starts_with('$') {
        return false;
    }
    let upper = name.to_ascii_uppercase();
    SECRET_VALUE_PREFIXES.iter().any(|p| value.starts_with(p))
        || SECRET_NAME_HINTS.iter().any(|h| upper.contains(h))
}

fn lint_env(env: &Value, out: &mut Vec<LintItem>) {
    let Some(map) = env.as_object() else { return };
    for (name, v) in map {
        let path = format!("env.{name}");
        match v.as_str() {
            None => out.push(warn(&path, "환경변수 값은 문자열이어야 합니다".to_string())),
            Some(s) if secret_looking(name, s) => out.push(warn(
                &path,
                "비밀값이 설정 파일에 평문으로 저장된 것 같습니다 — apiKeyHelper나 OS 환경변수 사용을 권장합니다"
                    .to_string(),
            )),
            _ => {}
        }
    }
}

fn lint_permissions(perms: &Value, out: &mut Vec<LintItem>) {
    let Some(map) = perms.as_object() else { return };
    for list in ["allow", "ask", "deny"] {
        if let Some(v) = map.get(list) {
            if !is_string_array(v) {
                out.push(warn(
                    &format!("permissions.{list}"),
                    "문자열 배열이어야 합니다".to_string(),
                ));
            }
        }
    }
    if let Some(v) = map.get("defaultMode") {
        match v.as_str() {
            Some(s) if DEFAULT_MODES.contains(&s) => {}
            Some(s) => out.push(warn(
                "permissions.defaultMode",
                format!("허용값: {} (현재: \"{s}\")", DEFAULT_MODES.join(" | ")),
            )),
            None => out.push(warn(
                "permissions.defaultMode",
                "문자열이어야 합니다".to_string(),
            )),
        }
    }
    if let Some(v) = map.get("additionalDirectories") {
        if !is_string_array(v) {
            out.push(warn(
                "permissions.additionalDirectories",
                "문자열(폴더 경로) 배열이어야 합니다".to_string(),
            ));
        }
    }
    if let Some(v) = map.get("disableBypassPermissionsMode") {
        // Official schema: string enum ["disable"] (a boolean is a common mistake).
        if v.as_str() != Some("disable") {
            out.push(warn(
                "permissions.disableBypassPermissionsMode",
                "값은 문자열 \"disable\"만 허용됩니다".to_string(),
            ));
        }
    }
}

/// Lint a Claude Code settings document (already-parsed JSON root).
pub fn lint_value(root: &Value) -> Vec<LintItem> {
    let mut out = Vec::new();
    let Some(obj) = root.as_object() else {
        return vec![warn("$", "settings.json 최상위는 JSON 객체여야 합니다")];
    };

    for (key, v) in obj {
        match KNOWN_KEYS.iter().find(|(k, _)| k == key) {
            Some((_, expect)) => check_known(key, expect, v, &mut out),
            None => out.push(info(
                key,
                "Agent Guard가 모르는 키입니다 — 최신 Claude Code 기능이라면 정상이며, 그대로 보존됩니다".to_string(),
            )),
        }
    }

    if let Some(env) = obj.get("env") {
        lint_env(env, &mut out);
    }
    if let Some(perms) = obj.get("permissions") {
        lint_permissions(perms, &mut out);
    }
    if obj.get("includeCoAuthoredBy").is_some() {
        out.push(info(
            "includeCoAuthoredBy",
            "최신 버전에서는 attribution 키로 대체되었습니다 (여전히 동작함)".to_string(),
        ));
    }
    out
}

/// Lint settings text. Parse errors yield a single warning (the Raw editor already
/// surfaces syntax errors separately).
pub fn lint_text(text: &str) -> Vec<LintItem> {
    if text.trim().is_empty() {
        return Vec::new();
    }
    match serde_json::from_str::<Value>(text) {
        Ok(v) => lint_value(&v),
        Err(e) => vec![warn("$", format!("JSON 구문 오류: {e}"))],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn paths_of(items: &[LintItem], level: LintLevel) -> Vec<String> {
        items
            .iter()
            .filter(|i| i.level == level)
            .map(|i| i.path.clone())
            .collect()
    }

    #[test]
    fn empty_and_valid_settings_are_clean() {
        assert!(lint_text("").is_empty());
        let v = json!({
            "model": "claude-sonnet-5",
            "effortLevel": "high",
            "includeCoAuthoredBy": true,
            "cleanupPeriodDays": 30,
            "autoUpdatesChannel": "stable",
            "permissions": {
                "defaultMode": "dontAsk",
                "allow": ["Read(./src/**)"],
                "additionalDirectories": ["~/shared"],
                "disableBypassPermissionsMode": "disable"
            },
            "env": { "ANTHROPIC_MODEL": "claude-sonnet-5" }
        });
        let items = lint_value(&v);
        // includeCoAuthoredBy deprecation notice is the only (info) finding.
        assert!(paths_of(&items, LintLevel::Warn).is_empty(), "{items:?}");
    }

    #[test]
    fn wrong_types_and_enums_warn() {
        let v = json!({
            "model": 3,
            "effortLevel": "extreme",
            "cleanupPeriodDays": 0,
            "permissions": {
                "defaultMode": "yolo",
                "disableBypassPermissionsMode": true
            }
        });
        let warns = paths_of(&lint_value(&v), LintLevel::Warn);
        assert!(warns.contains(&"model".to_string()));
        assert!(warns.contains(&"effortLevel".to_string()));
        assert!(warns.contains(&"cleanupPeriodDays".to_string()));
        assert!(warns.contains(&"permissions.defaultMode".to_string()));
        assert!(warns.contains(&"permissions.disableBypassPermissionsMode".to_string()));
    }

    #[test]
    fn env_secret_values_warn_but_references_do_not() {
        let v = json!({
            "env": {
                "ANTHROPIC_API_KEY": "sk-ant-abc123",
                "MY_TOKEN": "some-value",
                "GITHUB_TOKEN": "$GH_TOKEN",
                "AWS_REGION": "us-east-1"
            }
        });
        let warns = paths_of(&lint_value(&v), LintLevel::Warn);
        assert!(warns.contains(&"env.ANTHROPIC_API_KEY".to_string()));
        assert!(warns.contains(&"env.MY_TOKEN".to_string())); // name hint
        assert!(!warns.contains(&"env.GITHUB_TOKEN".to_string())); // $ reference
        assert!(!warns.contains(&"env.AWS_REGION".to_string()));
    }

    #[test]
    fn unknown_keys_are_info_not_warn() {
        let v = json!({ "someFutureKey": { "x": 1 } });
        let items = lint_value(&v);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].level, LintLevel::Info);
        assert_eq!(items[0].path, "someFutureKey");
    }

    #[test]
    fn version_drift_keys_never_warn() {
        // statusLine was an object in older versions, a string now; spinnerTips
        // flipped bool -> array. Both shapes must pass.
        let v = json!({
            "statusLine": { "type": "command", "command": "st.sh" },
            "spinnerTips": false
        });
        assert!(paths_of(&lint_value(&v), LintLevel::Warn).is_empty());
        let v2 = json!({ "statusLine": "st.sh", "spinnerTips": ["tip"] });
        assert!(paths_of(&lint_value(&v2), LintLevel::Warn).is_empty());
    }

    #[test]
    fn syntax_error_is_single_warning() {
        let items = lint_text("{ not json");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].level, LintLevel::Warn);
    }
}
