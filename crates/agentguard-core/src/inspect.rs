//! Read-only visibility into the agent's non-path risk surface: hooks defined in
//! settings files (they run arbitrary shell commands) and MCP servers
//! (`.mcp.json`, `~/.claude.json`), which operate outside the file-permission
//! model. Parsing is defensive: malformed shapes are skipped, never an error.

use crate::model::Scope;
use serde::Serialize;
use serde_json::Value;

/// One configured hook command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HookEntry {
    pub scope: Scope,
    /// Hook event name (`PreToolUse`, `PostToolUse`, `SessionStart`, ...).
    pub event: String,
    /// Tool matcher, when the event carries one (e.g. `"Bash"`, `"Edit|Write"`).
    pub matcher: Option<String>,
    pub command: String,
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
                if let Some(command) = hook.get("command").and_then(|c| c.as_str()) {
                    out.push(HookEntry {
                        scope,
                        event: event.clone(),
                        matcher: matcher.clone(),
                        command: command.to_string(),
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
        });
    }
    out
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
}
