//! Read-only visibility into the agent's non-path risk surface: hooks defined in
//! settings files (they run arbitrary shell commands) and MCP servers
//! (`.mcp.json`, `~/.claude.json`), which operate outside the file-permission
//! model. Parsing is defensive: malformed shapes are skipped, never an error.

use crate::model::Scope;
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeSet;

/// One configured hook command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HookEntry {
    pub scope: Scope,
    /// Hook event name (`PreToolUse`, `PostToolUse`, `SessionStart`, ...).
    pub event: String,
    /// Tool matcher, when the event carries one (e.g. `"Bash"`, `"Edit|Write"`).
    pub matcher: Option<String>,
    /// `command` | `prompt` | `agent` | `http` | `mcp`.
    pub handler_type: String,
    /// Human-readable executable command, prompt, URL, or MCP tool name.
    pub command: String,
    /// `high` for automatic code/tool/network execution, `medium` for prompt-only hooks.
    pub risk_level: String,
    pub uses_web: bool,
}

/// One configured MCP server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServer {
    pub name: String,
    /// Where it was configured (`project (.mcp.json)` / `user (~/.claude.json)`).
    pub source: String,
    /// `stdio` | `http` | `sse`.
    pub transport: String,
    /// The launched command line, or the remote URL.
    pub target: String,
    /// Likely talks to the internet: remote transports always do; stdio servers
    /// are flagged by a name/command heuristic (context7, fetch, search, ...).
    pub uses_web: bool,
    /// Whether Claude Code will currently load this server. Project `.mcp.json`
    /// entries require explicit approval in settings.
    pub active: bool,
    pub status_reason: String,
}

/// Name/command fragments of stdio MCP servers known to reach the internet.
const WEB_HINT_FRAGMENTS: &[&str] = &[
    "context7",
    "fetch",
    "search",
    "brave",
    "tavily",
    "exa",
    "firecrawl",
    "perplexity",
    "browser",
    "puppeteer",
    "playwright",
    "web",
    "http",
];

fn stdio_uses_web(name: &str, target: &str) -> bool {
    let hay = format!(
        "{} {}",
        name.to_ascii_lowercase(),
        target.to_ascii_lowercase()
    );
    WEB_HINT_FRAGMENTS.iter().any(|h| hay.contains(h))
}

/// Extract hook entries from a parsed settings.json tree.
pub fn hooks_from_settings(scope: Scope, raw: &Value) -> Vec<HookEntry> {
    let mut out = Vec::new();
    let Some(events) = raw.get("hooks").and_then(|h| h.as_object()) else {
        return out;
    };
    for (event, groups) in events {
        let Some(groups) = groups.as_array() else {
            continue;
        };
        for group in groups {
            let matcher = group
                .get("matcher")
                .and_then(|m| m.as_str())
                .map(|s| s.to_string());
            let Some(hooks) = group.get("hooks").and_then(|h| h.as_array()) else {
                continue;
            };
            for hook in hooks {
                let handler_type = hook
                    .get("type")
                    .and_then(Value::as_str)
                    .unwrap_or("command");
                let target = match handler_type {
                    "command" => hook.get("command").and_then(Value::as_str),
                    "prompt" | "agent" => hook.get("prompt").and_then(Value::as_str),
                    "http" => hook.get("url").and_then(Value::as_str),
                    "mcp" => hook
                        .get("tool")
                        .or_else(|| hook.get("toolName"))
                        .and_then(Value::as_str),
                    _ => None,
                };
                if let Some(command) = target {
                    let uses_web = handler_type == "http"
                        || (handler_type == "command" && stdio_uses_web("hook", command));
                    let risk_level = if handler_type == "prompt" {
                        "medium"
                    } else {
                        "high"
                    };
                    out.push(HookEntry {
                        scope,
                        event: event.clone(),
                        matcher: matcher.clone(),
                        handler_type: handler_type.to_string(),
                        command: command.to_string(),
                        risk_level: risk_level.to_string(),
                        uses_web,
                    });
                }
            }
        }
    }
    out
}

/// Extract MCP servers from an `mcpServers` object (shared shape between
/// `.mcp.json` and `~/.claude.json`).
pub fn mcp_servers_from_value(source: &str, servers: &Value) -> Vec<McpServer> {
    let mut out = Vec::new();
    let Some(map) = servers.as_object() else {
        return out;
    };
    for (name, cfg) in map {
        let Some(cfg) = cfg.as_object() else {
            continue;
        };
        let (transport, target, uses_web) =
            if let Some(url) = cfg.get("url").and_then(|u| u.as_str()) {
                let t = cfg
                    .get("type")
                    .and_then(|t| t.as_str())
                    .unwrap_or("http")
                    .to_string();
                (t, url.to_string(), true)
            } else if let Some(cmd) = cfg.get("command").and_then(|c| c.as_str()) {
                let args: Vec<String> = cfg
                    .get("args")
                    .and_then(|a| a.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|x| x.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default();
                let line = if args.is_empty() {
                    cmd.to_string()
                } else {
                    format!("{cmd} {}", args.join(" "))
                };
                let web = stdio_uses_web(name, &line);
                ("stdio".to_string(), line, web)
            } else {
                continue; // neither url nor command — not a server entry
            };
        out.push(McpServer {
            name: name.clone(),
            source: source.to_string(),
            transport,
            target,
            uses_web,
            active: !source.contains("project"),
            status_reason: if source.contains("project") {
                "프로젝트 MCP 승인 대기".to_string()
            } else {
                "사용자 설정에서 활성".to_string()
            },
        });
    }
    out
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProjectMcpApproval {
    pub enable_all: bool,
    pub enabled: BTreeSet<String>,
    pub disabled: BTreeSet<String>,
}

/// Resolve the three settings keys Claude Code uses to approve project
/// `.mcp.json` entries. Values are supplied from low to high precedence.
pub fn project_mcp_approval(settings: &[&Value]) -> ProjectMcpApproval {
    let mut approval = ProjectMcpApproval::default();
    for raw in settings {
        if let Some(enabled) = raw
            .get("enableAllProjectMcpServers")
            .and_then(Value::as_bool)
        {
            approval.enable_all = enabled;
        }
        for (key, target) in [
            ("enabledMcpjsonServers", &mut approval.enabled),
            ("disabledMcpjsonServers", &mut approval.disabled),
        ] {
            if let Some(names) = raw.get(key).and_then(Value::as_array) {
                target.extend(names.iter().filter_map(Value::as_str).map(str::to_string));
            }
        }
    }
    approval
}

pub fn apply_project_mcp_approval(servers: &mut [McpServer], approval: &ProjectMcpApproval) {
    for server in servers
        .iter_mut()
        .filter(|server| server.source.contains("project"))
    {
        if approval.disabled.contains(&server.name) {
            server.active = false;
            server.status_reason = "disabledMcpjsonServers에서 비활성".to_string();
        } else if approval.enable_all || approval.enabled.contains(&server.name) {
            server.active = true;
            server.status_reason = if approval.enable_all {
                "모든 프로젝트 MCP 서버 승인".to_string()
            } else {
                "enabledMcpjsonServers에서 승인".to_string()
            };
        } else {
            server.active = false;
            server.status_reason = "프로젝트 MCP 승인 대기".to_string();
        }
    }
}

/// Parse `.mcp.json` text (root `{ "mcpServers": { ... } }`).
pub fn parse_mcp_json(source: &str, text: &str) -> Vec<McpServer> {
    let Ok(v) = serde_json::from_str::<Value>(text) else {
        return Vec::new();
    };
    v.get("mcpServers")
        .map(|s| mcp_servers_from_value(source, s))
        .unwrap_or_default()
}

/// Parse `~/.claude.json` text and extract its top-level `mcpServers`.
pub fn parse_claude_json_mcp(source: &str, text: &str) -> Vec<McpServer> {
    let Ok(v) = serde_json::from_str::<Value>(text) else {
        return Vec::new();
    };
    v.get("mcpServers")
        .map(|s| mcp_servers_from_value(source, s))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn hooks_parsed_with_and_without_matcher() {
        let raw = json!({
            "hooks": {
                "PreToolUse": [
                    {
                        "matcher": "Bash",
                        "hooks": [{ "type": "command", "command": "echo pre" }]
                    }
                ],
                "SessionStart": [
                    { "hooks": [{ "type": "command", "command": "setup.sh" }] }
                ]
            }
        });
        let mut hooks = hooks_from_settings(Scope::Project, &raw);
        hooks.sort_by(|a, b| a.event.cmp(&b.event));
        assert_eq!(hooks.len(), 2);
        assert_eq!(hooks[0].event, "PreToolUse");
        assert_eq!(hooks[0].matcher.as_deref(), Some("Bash"));
        assert_eq!(hooks[0].command, "echo pre");
        assert_eq!(hooks[0].handler_type, "command");
        assert_eq!(hooks[0].risk_level, "high");
        assert_eq!(hooks[1].event, "SessionStart");
        assert_eq!(hooks[1].matcher, None);
    }

    #[test]
    fn no_hooks_key_yields_empty() {
        assert!(hooks_from_settings(Scope::User, &json!({})).is_empty());
        // Malformed shapes are skipped, not errors.
        let bad = json!({ "hooks": { "PreToolUse": "not-an-array" } });
        assert!(hooks_from_settings(Scope::User, &bad).is_empty());
    }

    #[test]
    fn mcp_servers_stdio_and_remote() {
        let text = r#"{
            "mcpServers": {
                "filesystem": {
                    "command": "npx",
                    "args": ["-y", "@modelcontextprotocol/server-filesystem", "."]
                },
                "linear": { "type": "sse", "url": "https://mcp.linear.app/sse" }
            }
        }"#;
        let mut servers = parse_mcp_json("project (.mcp.json)", text);
        servers.sort_by(|a, b| a.name.cmp(&b.name));
        assert_eq!(servers.len(), 2);
        assert_eq!(servers[0].name, "filesystem");
        assert_eq!(servers[0].transport, "stdio");
        assert_eq!(
            servers[0].target,
            "npx -y @modelcontextprotocol/server-filesystem ."
        );
        assert!(!servers[0].uses_web); // local stdio, no web hints
        assert_eq!(servers[1].transport, "sse");
        assert_eq!(servers[1].target, "https://mcp.linear.app/sse");
        assert!(servers[1].uses_web); // remote transports always reach out
    }

    #[test]
    fn web_reaching_stdio_servers_are_flagged() {
        let text = r#"{
            "mcpServers": {
                "context7": { "command": "npx", "args": ["-y", "@upstash/context7-mcp"] },
                "local-db": { "command": "sqlite-mcp", "args": ["./app.db"] }
            }
        }"#;
        let mut servers = parse_mcp_json("x", text);
        servers.sort_by(|a, b| a.name.cmp(&b.name));
        assert!(servers[0].uses_web); // context7 fetches docs from the web
        assert!(!servers[1].uses_web);
    }

    #[test]
    fn invalid_or_empty_mcp_json_yields_empty() {
        assert!(parse_mcp_json("x", "not json").is_empty());
        assert!(parse_mcp_json("x", "{}").is_empty());
        // Entries without command/url are skipped.
        let text = r#"{ "mcpServers": { "weird": { "note": "nothing runnable" } } }"#;
        assert!(parse_mcp_json("x", text).is_empty());
    }

    #[test]
    fn prompt_http_agent_and_mcp_hook_handlers_are_visible() {
        let raw = json!({
            "hooks": {
                "Stop": [{ "hooks": [
                    { "type": "prompt", "prompt": "Check $ARGUMENTS" },
                    { "type": "agent", "prompt": "Verify tests" },
                    { "type": "http", "url": "https://hooks.example.test/audit" },
                    { "type": "mcp", "tool": "mcp__audit__record" }
                ] }]
            }
        });
        let hooks = hooks_from_settings(Scope::Project, &raw);
        assert_eq!(hooks.len(), 4);
        assert_eq!(hooks[0].risk_level, "medium");
        assert!(hooks[2].uses_web);
        assert_eq!(hooks[3].handler_type, "mcp");
    }

    #[test]
    fn project_mcp_approval_respects_disabled_precedence() {
        let user = json!({ "enableAllProjectMcpServers": true });
        let local = json!({
            "enabledMcpjsonServers": ["safe", "blocked"],
            "disabledMcpjsonServers": ["blocked"]
        });
        let approval = project_mcp_approval(&[&user, &local]);
        let mut servers = parse_mcp_json(
            "project (.mcp.json)",
            r#"{ "mcpServers": {
                "safe": { "command": "safe-mcp" },
                "blocked": { "command": "blocked-mcp" }
            } }"#,
        );
        apply_project_mcp_approval(&mut servers, &approval);
        let safe = servers.iter().find(|server| server.name == "safe").unwrap();
        let blocked = servers
            .iter()
            .find(|server| server.name == "blocked")
            .unwrap();
        assert!(safe.active);
        assert!(!blocked.active);
        assert!(blocked.status_reason.contains("비활성"));
    }
}
