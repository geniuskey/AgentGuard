//! MCP server editing: import parsing (JSON / `claude mcp add ...`), validation,
//! warnings, and single-entry mutation of `.mcp.json` / `~/.claude.json`.
//! Mutations re-serialize the whole document but change only one entry; every
//! other key (and key order, via `preserve_order`) is kept.

use crate::settings_lint::secret_looking;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum McpTarget {
    Project,
    User,
    Local,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportedServer {
    pub name: String,
    pub config: Value,
    pub scope_hint: Option<McpTarget>,
    pub warnings: Vec<String>,
}

const UNRECOGNIZED: &str =
    "인식할 수 없는 형식입니다. JSON 또는 `claude mcp add ...` 명령을 붙여넣으세요.";

// --- Import ---------------------------------------------------------------------

pub fn parse_import(text: &str) -> Result<Vec<ImportedServer>, String> {
    let text = text.trim();
    if text.is_empty() {
        return Err("붙여넣은 내용이 비어 있습니다.".to_string());
    }
    let mut out = if text.starts_with('{') {
        let v: Value = serde_json::from_str(text).map_err(|e| format!("JSON 파싱 실패: {e}"))?;
        import_json(&v)?
    } else {
        let commands = tokenize(text)?;
        if commands.is_empty() {
            return Err(UNRECOGNIZED.to_string());
        }
        commands
            .iter()
            .map(|c| import_command(c))
            .collect::<Result<Vec<_>, _>>()?
    };
    for s in &mut out {
        validate_config(&s.name, &s.config)?;
        s.warnings = config_warnings(&s.name, &s.config);
    }
    Ok(out)
}

fn imported(name: String, config: Value, scope_hint: Option<McpTarget>) -> ImportedServer {
    ImportedServer {
        name,
        config,
        scope_hint,
        warnings: Vec::new(),
    }
}

fn is_server_cfg(v: &Value) -> bool {
    v.get("command").is_some_and(Value::is_string) || v.get("url").is_some_and(Value::is_string)
}

fn import_json(v: &Value) -> Result<Vec<ImportedServer>, String> {
    let Some(obj) = v.as_object() else {
        return Err(UNRECOGNIZED.to_string());
    };
    let map = if let Some(servers) = obj.get("mcpServers") {
        servers
            .as_object()
            .ok_or("`mcpServers`는 객체여야 합니다.")?
    } else if is_server_cfg(v) {
        return Ok(vec![imported(derive_name(v), v.clone(), None)]);
    } else if !obj.is_empty() && obj.values().all(is_server_cfg) {
        obj
    } else {
        return Err(UNRECOGNIZED.to_string());
    };
    if map.is_empty() {
        return Err("`mcpServers`에 서버가 없습니다.".to_string());
    }
    Ok(map
        .iter()
        .map(|(k, cfg)| imported(k.clone(), cfg.clone(), None))
        .collect())
}

/// Shell-like tokenizer. Returns one token list per command (newline-separated);
/// `\` / backtick line continuations are joined. Backslashes are literal except
/// `\"` inside double quotes, so Windows paths survive.
fn tokenize(text: &str) -> Result<Vec<Vec<String>>, String> {
    let chars: Vec<char> = text.chars().collect();
    let mut commands = Vec::new();
    let mut tokens: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut in_token = false;
    let mut i = 0;

    let is_continuation = |i: usize| -> bool {
        let mut j = i + 1;
        while j < chars.len() && (chars[j] == ' ' || chars[j] == '\t' || chars[j] == '\r') {
            j += 1;
        }
        j >= chars.len() || chars[j] == '\n'
    };

    while i < chars.len() {
        let c = chars[i];
        match c {
            '\'' => {
                in_token = true;
                i += 1;
                while i < chars.len() && chars[i] != '\'' {
                    cur.push(chars[i]);
                    i += 1;
                }
                if i >= chars.len() {
                    return Err("작은따옴표가 닫히지 않았습니다.".to_string());
                }
            }
            '"' => {
                in_token = true;
                i += 1;
                loop {
                    if i >= chars.len() {
                        return Err("큰따옴표가 닫히지 않았습니다.".to_string());
                    }
                    match chars[i] {
                        '"' => break,
                        '\\' if chars.get(i + 1) == Some(&'"') => {
                            cur.push('"');
                            i += 1;
                        }
                        ch => cur.push(ch),
                    }
                    i += 1;
                }
            }
            '\\' | '`' if is_continuation(i) => {
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
            }
            ' ' | '\t' | '\r' | '\n' => {
                if in_token {
                    tokens.push(std::mem::take(&mut cur));
                    in_token = false;
                }
                if c == '\n' && !tokens.is_empty() {
                    commands.push(std::mem::take(&mut tokens));
                }
            }
            _ => {
                in_token = true;
                cur.push(c);
            }
        }
        i += 1;
    }
    if in_token {
        tokens.push(cur);
    }
    if !tokens.is_empty() {
        commands.push(tokens);
    }
    // Strip shell prompts and drop comment lines.
    Ok(commands
        .into_iter()
        .map(|mut t| {
            if matches!(t.first().map(String::as_str), Some("$") | Some(">")) {
                t.remove(0);
            }
            t
        })
        .filter(|t| !t.is_empty() && !t[0].starts_with('#'))
        .collect())
}

fn parse_scope(v: &str) -> Result<McpTarget, String> {
    match v {
        "local" => Ok(McpTarget::Local),
        "user" => Ok(McpTarget::User),
        "project" => Ok(McpTarget::Project),
        other => Err(format!(
            "알 수 없는 scope입니다: `{other}` (local | user | project)"
        )),
    }
}

/// Split `--opt=value` into (`--opt`, Some(value)).
fn split_opt(tok: &str) -> (&str, Option<&str>) {
    if tok.starts_with("--") {
        if let Some((k, v)) = tok.split_once('=') {
            return (k, Some(v));
        }
    }
    (tok, None)
}

fn import_command(tokens: &[String]) -> Result<ImportedServer, String> {
    let claude = tokens.first().map(|t| t.to_ascii_lowercase());
    let is_claude = claude
        .as_deref()
        .is_some_and(|t| t == "claude" || t == "claude.exe" || t == "claude.cmd");
    if !is_claude || tokens.get(1).map(String::as_str) != Some("mcp") {
        return Err(UNRECOGNIZED.to_string());
    }
    match tokens.get(2).map(String::as_str) {
        Some("add") => import_add(&tokens[3..]),
        Some("add-json") => import_add_json(&tokens[3..]),
        _ => Err(UNRECOGNIZED.to_string()),
    }
}

fn import_add(rest: &[String]) -> Result<ImportedServer, String> {
    let mut transport: Option<String> = None;
    let mut scope = None;
    let mut env = Map::new();
    let mut headers = Map::new();
    let mut positionals: Vec<String> = Vec::new();
    let mut command_after_dashes: Option<Vec<String>> = None;
    let mut i = 0;

    while i < rest.len() {
        let tok = &rest[i];
        if tok == "--" {
            command_after_dashes = Some(rest[i + 1..].to_vec());
            break;
        }
        // Once a stdio target is known, everything else belongs to the command.
        if positionals.len() >= 2 && !is_remote(transport.as_deref(), &positionals[1]) {
            positionals.extend(rest[i..].iter().cloned());
            break;
        }
        if tok.starts_with('-') && tok.len() > 1 {
            let (key, inline) = split_opt(tok);
            let take_value = |i: &mut usize| -> Result<String, String> {
                if let Some(v) = inline {
                    return Ok(v.to_string());
                }
                *i += 1;
                rest.get(*i)
                    .cloned()
                    .ok_or_else(|| format!("`{key}` 옵션에 값이 없습니다."))
            };
            match key {
                "--transport" | "-t" => transport = Some(take_value(&mut i)?),
                "--scope" | "-s" => scope = Some(parse_scope(&take_value(&mut i)?)?),
                "--env" | "-e" => {
                    let first = take_value(&mut i)?;
                    let mut pairs = vec![first];
                    // Variadic: `-e A=1 B=2` (names never contain `=`).
                    while inline.is_none()
                        && rest
                            .get(i + 1)
                            .is_some_and(|t| !t.starts_with('-') && t.contains('='))
                    {
                        i += 1;
                        pairs.push(rest[i].clone());
                    }
                    for p in pairs {
                        let (k, v) = p.split_once('=').ok_or_else(|| {
                            format!("환경변수는 KEY=VALUE 형식이어야 합니다: `{p}`")
                        })?;
                        env.insert(k.to_string(), Value::String(v.to_string()));
                    }
                }
                "--header" | "-H" => {
                    let h = take_value(&mut i)?;
                    let (k, v) = h
                        .split_once(':')
                        .ok_or_else(|| format!("헤더는 `Name: Value` 형식이어야 합니다: `{h}`"))?;
                    headers.insert(k.trim().to_string(), Value::String(v.trim().to_string()));
                }
                other => return Err(format!("지원하지 않는 옵션입니다: `{other}`")),
            }
        } else {
            positionals.push(tok.clone());
        }
        i += 1;
    }

    let mut positionals = positionals.into_iter();
    let name = positionals
        .next()
        .ok_or("서버 이름이 없습니다: `claude mcp add <name> <command>`")?;
    let command_line: Vec<String> = match command_after_dashes {
        Some(cmd) => {
            if positionals.next().is_some() {
                return Err("`--` 앞에는 서버 이름만 올 수 있습니다.".to_string());
            }
            cmd
        }
        None => positionals.collect(),
    };
    let Some(target) = command_line.first().cloned() else {
        return Err("실행할 명령 또는 URL이 없습니다.".to_string());
    };

    let mut cfg = Map::new();
    if is_remote(transport.as_deref(), &target) {
        if command_line.len() > 1 {
            return Err("원격(http/sse) 서버에는 URL 하나만 지정해야 합니다.".to_string());
        }
        let t = match transport.as_deref() {
            None | Some("http") => "http",
            Some("sse") => "sse",
            Some(other) => return Err(format!("알 수 없는 transport입니다: `{other}`")),
        };
        cfg.insert("type".into(), json!(t));
        cfg.insert("url".into(), json!(target));
        if !headers.is_empty() {
            cfg.insert("headers".into(), Value::Object(headers));
        }
        if !env.is_empty() {
            cfg.insert("env".into(), Value::Object(env));
        }
    } else {
        match transport.as_deref() {
            None | Some("stdio") => {}
            Some(other) => return Err(format!("알 수 없는 transport입니다: `{other}`")),
        }
        cfg.insert("command".into(), json!(target));
        if command_line.len() > 1 {
            cfg.insert("args".into(), json!(command_line[1..]));
        }
        if !env.is_empty() {
            cfg.insert("env".into(), Value::Object(env));
        }
        if !headers.is_empty() {
            return Err("`--header`는 http/sse 서버에서만 사용할 수 있습니다.".to_string());
        }
    }
    Ok(imported(name, Value::Object(cfg), scope))
}

fn is_remote(transport: Option<&str>, target: &str) -> bool {
    match transport {
        Some("http") | Some("sse") => true,
        Some(_) => false,
        None => target.starts_with("http://") || target.starts_with("https://"),
    }
}

fn import_add_json(rest: &[String]) -> Result<ImportedServer, String> {
    let mut scope = None;
    let mut positionals = Vec::new();
    let mut i = 0;
    while i < rest.len() {
        let tok = &rest[i];
        let (key, inline) = split_opt(tok);
        match key {
            "--scope" | "-s" => {
                let v = match inline {
                    Some(v) => v.to_string(),
                    None => {
                        i += 1;
                        rest.get(i)
                            .cloned()
                            .ok_or("`--scope` 옵션에 값이 없습니다.")?
                    }
                };
                scope = Some(parse_scope(&v)?);
            }
            _ if tok.starts_with('-') && positionals.len() < 2 && !tok.starts_with("-{") => {
                return Err(format!("지원하지 않는 옵션입니다: `{tok}`"));
            }
            _ => positionals.push(tok.clone()),
        }
        i += 1;
    }
    let [name, raw] = positionals.as_slice() else {
        return Err("형식: `claude mcp add-json <name> '<json>'`".to_string());
    };
    let cfg: Value = serde_json::from_str(raw).map_err(|e| format!("JSON 파싱 실패: {e}"))?;
    if !cfg.is_object() {
        return Err("서버 설정은 JSON 객체여야 합니다.".to_string());
    }
    Ok(imported(name.clone(), cfg, scope))
}

// --- Name derivation --------------------------------------------------------------

const RUNNERS: &[&str] = &["npx", "uvx", "bunx", "pnpx"];

fn strip_exe(s: &str) -> String {
    let base = s
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(s)
        .to_ascii_lowercase();
    for ext in [".cmd", ".exe", ".bat"] {
        if let Some(b) = base.strip_suffix(ext) {
            return b.to_string();
        }
    }
    base
}

/// The full command line as tokens (`command` + `args`).
fn command_line(cfg: &Value) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(c) = cfg.get("command").and_then(Value::as_str) {
        out.push(c.to_string());
    }
    if let Some(args) = cfg.get("args").and_then(Value::as_array) {
        out.extend(args.iter().filter_map(Value::as_str).map(str::to_string));
    }
    out
}

/// (runner, package, has `-y`) for `npx`/`uvx`/`bunx` launches (also via `cmd /c`).
fn runner_package(cfg: &Value) -> Option<(String, String, bool)> {
    let line = command_line(cfg);
    let idx = line
        .iter()
        .position(|t| RUNNERS.contains(&strip_exe(t).as_str()))?;
    let runner = strip_exe(&line[idx]);
    let rest = &line[idx + 1..];
    let yes = rest.iter().any(|t| t == "-y" || t == "--yes");
    let pkg = rest.iter().find(|t| !t.starts_with('-'))?.clone();
    Some((runner, pkg, yes))
}

fn package_base(pkg: &str) -> String {
    let unscoped = match pkg.strip_prefix('@') {
        Some(s) => s.split_once('/').map(|(_, p)| p).unwrap_or(s),
        None => pkg,
    };
    let no_version = unscoped
        .split(['@', '='])
        .next()
        .unwrap_or(unscoped)
        .to_string();
    let mut n = no_version.as_str();
    for p in ["mcp-server-", "server-"] {
        n = n.strip_prefix(p).unwrap_or(n);
    }
    for s in ["-mcp-server", "-mcp", "-server"] {
        n = n.strip_suffix(s).unwrap_or(n);
    }
    n.to_string()
}

fn sanitize_name(s: &str) -> String {
    let n: String = s
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
        .collect();
    if n.is_empty() {
        "server".to_string()
    } else {
        n
    }
}

fn url_host(url: &str) -> Option<&str> {
    let rest = url.split_once("://")?.1;
    let authority = rest.split(['/', '?', '#']).next()?;
    let host = authority.rsplit('@').next()?;
    if host.starts_with('[') {
        return host.split(']').next().map(|h| h.trim_start_matches('['));
    }
    host.split(':').next()
}

fn derive_name(cfg: &Value) -> String {
    if let Some((_, pkg, _)) = runner_package(cfg) {
        return sanitize_name(&package_base(&pkg));
    }
    if let Some(host) = cfg.get("url").and_then(Value::as_str).and_then(url_host) {
        const GENERIC: &[&str] = &["mcp", "api", "www", "server"];
        let labels: Vec<&str> = host.split('.').collect();
        let pick = labels
            .iter()
            .take(labels.len().saturating_sub(1).max(1))
            .find(|l| !GENERIC.contains(l) && !l.chars().all(|c| c.is_ascii_digit()));
        if let Some(l) = pick {
            return sanitize_name(l);
        }
    }
    "server".to_string()
}

// --- Validation & warnings -------------------------------------------------------

fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
}

fn string_map(cfg: &Value, key: &str) -> Result<(), String> {
    match cfg.get(key) {
        None => Ok(()),
        Some(Value::Object(m)) if m.values().all(Value::is_string) => Ok(()),
        Some(_) => Err(format!("`{key}`는 문자열 값을 가진 객체여야 합니다.")),
    }
}

pub fn validate_config(name: &str, config: &Value) -> Result<(), String> {
    if !valid_name(name) {
        return Err(format!(
            "서버 이름 `{name}`이(가) 올바르지 않습니다. 영문, 숫자, `_` `.` `-`만 사용할 수 있습니다."
        ));
    }
    let Some(obj) = config.as_object() else {
        return Err(format!("`{name}`: 서버 설정은 JSON 객체여야 합니다."));
    };
    let ty = match obj.get("type") {
        None => None,
        Some(Value::String(t)) if matches!(t.as_str(), "stdio" | "http" | "sse") => {
            Some(t.as_str())
        }
        Some(_) => {
            return Err(format!(
                "`{name}`: `type`은 stdio | http | sse 중 하나여야 합니다."
            ))
        }
    };
    let command = obj.get("command");
    let url = obj.get("url");
    let remote = match (command, url) {
        (Some(_), Some(_)) => {
            return Err(format!(
                "`{name}`: `command`와 `url`을 동시에 지정할 수 없습니다."
            ))
        }
        (Some(Value::String(c)), None) if !c.trim().is_empty() => false,
        (None, Some(Value::String(u))) if u.starts_with("http://") || u.starts_with("https://") => {
            true
        }
        (None, Some(_)) => {
            return Err(format!(
                "`{name}`: `url`은 http:// 또는 https://로 시작해야 합니다."
            ))
        }
        _ => return Err(format!("`{name}`: `command` 또는 `url`이 필요합니다.")),
    };
    if remote && ty == Some("stdio") {
        return Err(format!(
            "`{name}`: `url` 서버의 type은 http 또는 sse여야 합니다."
        ));
    }
    if !remote && matches!(ty, Some("http") | Some("sse")) {
        return Err(format!("`{name}`: http/sse 서버에는 `url`이 필요합니다."));
    }
    if let Some(args) = obj.get("args") {
        if !args
            .as_array()
            .is_some_and(|a| a.iter().all(Value::is_string))
        {
            return Err(format!("`{name}`: `args`는 문자열 배열이어야 합니다."));
        }
    }
    string_map(config, "env").map_err(|e| format!("`{name}`: {e}"))?;
    string_map(config, "headers").map_err(|e| format!("`{name}`: {e}"))?;
    Ok(())
}

pub fn config_warnings(_name: &str, config: &Value) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(env) = config.get("env").and_then(Value::as_object) {
        for (k, v) in env {
            if let Some(v) = v.as_str() {
                if !v.contains("${") && secret_looking(k, v) {
                    out.push(format!(
                        "env `{k}`에 비밀값이 평문으로 저장된 것 같습니다 — `${{{k}}}` 같은 환경변수 참조 사용을 권장합니다."
                    ));
                }
            }
        }
    }
    if let Some(headers) = config.get("headers").and_then(Value::as_object) {
        for (k, v) in headers {
            let Some(v) = v.as_str() else { continue };
            let lower = k.to_ascii_lowercase();
            let sensitive = lower == "authorization"
                || lower.contains("token")
                || lower.contains("key")
                || secret_looking(k, v);
            if sensitive && !v.is_empty() && !v.contains("${") {
                out.push(format!(
                    "헤더 `{k}`에 비밀값이 평문으로 저장된 것 같습니다 — `${{VAR}}` 환경변수 참조 사용을 권장합니다."
                ));
            }
        }
    }
    if let Some(url) = config.get("url").and_then(Value::as_str) {
        if url.starts_with("http://") {
            let host = url_host(url).unwrap_or("");
            if !matches!(host, "localhost" | "127.0.0.1" | "::1") {
                out.push(
                    "암호화되지 않은 http:// 원격 서버입니다 — https 사용을 권장합니다."
                        .to_string(),
                );
            }
        }
    }
    if let Some((runner, pkg, yes)) = runner_package(config) {
        let unpinned = !pkg.trim_start_matches('@').contains('@');
        if (runner == "npx" || runner == "bunx") && yes && unpinned {
            out.push(format!(
                "`{pkg}`의 버전이 고정되지 않아 실행할 때마다 최신 버전을 설치합니다."
            ));
        }
    }
    if cfg!(windows) && config.get("command").and_then(Value::as_str) == Some("npx") {
        out.push("Windows에서는 `cmd /c npx ...` 형태가 필요할 수 있습니다.".to_string());
    }
    out
}

// --- File mutation ---------------------------------------------------------------

fn normalize_key(k: &str) -> String {
    let s = k.replace('\\', "/");
    let s = s.trim_end_matches('/');
    let b = s.as_bytes();
    if b.len() >= 2 && b[1] == b':' && b[0].is_ascii_alphabetic() {
        s.to_ascii_lowercase()
    } else {
        s.to_string()
    }
}

fn forward_slash_key(k: &str) -> String {
    let s = k.replace('\\', "/");
    let t = s.trim_end_matches('/');
    if t.is_empty() {
        s
    } else {
        t.to_string()
    }
}

fn parse_root(text: &str) -> Result<Map<String, Value>, String> {
    if text.trim().is_empty() {
        return Ok(Map::new());
    }
    let v: Value = serde_json::from_str(text)
        .map_err(|e| format!("파일이 올바른 JSON이 아니어서 수정하지 않았습니다: {e}"))?;
    match v {
        Value::Object(m) => Ok(m),
        _ => Err("파일 최상위가 JSON 객체가 아니어서 수정하지 않았습니다.".to_string()),
    }
}

fn find_project_key(projects: &Map<String, Value>, project_key: &str) -> Option<String> {
    let want = normalize_key(project_key);
    projects.keys().find(|k| normalize_key(k) == want).cloned()
}

fn servers_mut(
    obj: &mut Map<String, Value>,
    create: bool,
) -> Result<Option<&mut Map<String, Value>>, String> {
    if !obj.contains_key("mcpServers") {
        if !create {
            return Ok(None);
        }
        obj.insert("mcpServers".into(), Value::Object(Map::new()));
    }
    obj.get_mut("mcpServers")
        .and_then(Value::as_object_mut)
        .map(Some)
        .ok_or_else(|| "`mcpServers`가 객체가 아니어서 수정하지 않았습니다.".to_string())
}

pub fn apply(
    text: &str,
    target: McpTarget,
    project_key: &str,
    name: &str,
    config: Option<&Value>,
) -> Result<String, String> {
    if let Some(cfg) = config {
        validate_config(name, cfg)?;
    }
    let mut root = parse_root(text)?;
    let create = config.is_some();
    let servers = match target {
        McpTarget::Project | McpTarget::User => servers_mut(&mut root, create)?,
        McpTarget::Local => {
            if project_key.trim().is_empty() {
                return Err("local 설정에는 프로젝트 경로가 필요합니다.".to_string());
            }
            if !root.contains_key("projects") && create {
                root.insert("projects".into(), Value::Object(Map::new()));
            }
            match root.get_mut("projects") {
                None => None,
                Some(Value::Object(projects)) => {
                    let key = match find_project_key(projects, project_key) {
                        Some(k) => Some(k),
                        None if create => {
                            let k = forward_slash_key(project_key);
                            projects.insert(k.clone(), json!({}));
                            Some(k)
                        }
                        None => None,
                    };
                    match key {
                        None => None,
                        Some(k) => match projects.get_mut(&k) {
                            Some(Value::Object(entry)) => servers_mut(entry, create)?,
                            _ => {
                                return Err("프로젝트 항목이 객체가 아니어서 수정하지 않았습니다."
                                    .to_string())
                            }
                        },
                    }
                }
                Some(_) => {
                    return Err("`projects`가 객체가 아니어서 수정하지 않았습니다.".to_string())
                }
            }
        }
    };
    if let Some(servers) = servers {
        match config {
            Some(cfg) => {
                servers.insert(name.to_string(), cfg.clone());
            }
            None => {
                servers.shift_remove(name);
            }
        }
    }
    let mut out = serde_json::to_string_pretty(&Value::Object(root)).map_err(|e| e.to_string())?;
    out.push('\n');
    Ok(out)
}

fn local_section(root: &Map<String, Value>, project_key: &str) -> Value {
    root.get("projects")
        .and_then(Value::as_object)
        .and_then(|p| find_project_key(p, project_key).and_then(|k| p.get(&k)))
        .and_then(|e| e.get("mcpServers"))
        .cloned()
        .unwrap_or_else(|| json!({}))
}

pub fn section(text: &str, target: McpTarget, project_key: &str) -> Result<Value, String> {
    let root = parse_root(text)?;
    Ok(match target {
        McpTarget::Project | McpTarget::User => {
            root.get("mcpServers").cloned().unwrap_or_else(|| json!({}))
        }
        McpTarget::Local => local_section(&root, project_key),
    })
}

pub fn local_servers(text: &str, project_key: &str) -> Value {
    parse_root(text)
        .map(|root| local_section(&root, project_key))
        .unwrap_or_else(|_| json!({}))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one(text: &str) -> ImportedServer {
        let mut v = parse_import(text).unwrap();
        assert_eq!(v.len(), 1, "{v:?}");
        v.remove(0)
    }

    #[test]
    fn import_mcp_servers_wrapper_and_map() {
        let v = parse_import(
            r#"{ "mcpServers": { "a": { "command": "node", "args": ["x.js"] }, "b": { "type": "http", "url": "https://x.dev/mcp" } } }"#,
        )
        .unwrap();
        assert_eq!(v.len(), 2);
        assert_eq!(v[0].name, "a");
        assert_eq!(v[1].config["url"], "https://x.dev/mcp");

        let v = parse_import(r#"{ "fs": { "command": "npx", "args": ["-y", "@x/fs@1.0.0"] } }"#)
            .unwrap();
        assert_eq!(v[0].name, "fs");
    }

    #[test]
    fn import_single_cfg_derives_name() {
        let s = one(r#"{ "command": "npx", "args": ["-y", "@upstash/context7-mcp@latest"] }"#);
        assert_eq!(s.name, "context7");
        let s = one(r#"{ "command": "uvx", "args": ["mcp-server-fetch"] }"#);
        assert_eq!(s.name, "fetch");
        let s = one(
            r#"{ "command": "cmd", "args": ["/c", "npx", "-y", "@modelcontextprotocol/server-filesystem", "C:\\x"] }"#,
        );
        assert_eq!(s.name, "filesystem");
        let s = one(r#"{ "type": "sse", "url": "https://mcp.linear.app/sse" }"#);
        assert_eq!(s.name, "linear");
        let s = one(r#"{ "command": "node", "args": ["server.js"] }"#);
        assert_eq!(s.name, "server");
    }

    #[test]
    fn import_claude_mcp_add_stdio_with_quotes_and_env() {
        let s = one(
            r#"$ claude mcp add -e API_KEY=abc -e "OTHER=a b" -s user my-db node "C:\tools\db server.js" --port 5"#,
        );
        assert_eq!(s.name, "my-db");
        assert_eq!(s.scope_hint, Some(McpTarget::User));
        assert_eq!(
            s.config,
            json!({
                "command": "node",
                "args": ["C:\\tools\\db server.js", "--port", "5"],
                "env": { "API_KEY": "abc", "OTHER": "a b" }
            })
        );
    }

    #[test]
    fn import_claude_mcp_add_double_dash() {
        let s = one("claude mcp add --env A=1 B=2 airtable -- npx -y airtable-mcp-server -e x");
        assert_eq!(s.name, "airtable");
        assert_eq!(s.scope_hint, None);
        assert_eq!(
            s.config,
            json!({
                "command": "npx",
                "args": ["-y", "airtable-mcp-server", "-e", "x"],
                "env": { "A": "1", "B": "2" }
            })
        );
        assert!(s.config.get("type").is_none());
    }

    #[test]
    fn import_claude_mcp_add_remote_with_headers() {
        let s = one(
            "claude mcp add --transport http api https://api.example.com/mcp --header \"Authorization: Bearer x\" -H 'X-Team: t'",
        );
        assert_eq!(
            s.config,
            json!({
                "type": "http",
                "url": "https://api.example.com/mcp",
                "headers": { "Authorization": "Bearer x", "X-Team": "t" }
            })
        );
        assert!(!s.warnings.is_empty());

        let s = one("claude mcp add -t sse -s project asana https://mcp.asana.com/sse");
        assert_eq!(
            s.config,
            json!({ "type": "sse", "url": "https://mcp.asana.com/sse" })
        );
        assert_eq!(s.scope_hint, Some(McpTarget::Project));

        let s = one("claude mcp add notion https://mcp.notion.com/mcp");
        assert_eq!(s.config["type"], "http");
    }

    #[test]
    fn import_add_json_and_multiline() {
        let s = one(
            r#"claude mcp add-json -s local weather '{"type":"stdio","command":"/bin/w","args":["--k"]}'"#,
        );
        assert_eq!(s.name, "weather");
        assert_eq!(s.scope_hint, Some(McpTarget::Local));
        assert_eq!(s.config["command"], "/bin/w");

        let v = parse_import(
            "claude mcp add a -- node a.js\n\n> claude mcp add b \\\n  --scope user -- node b.js\nclaude mcp add c `\n  -- node c.js",
        )
        .unwrap();
        let names: Vec<_> = v.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["a", "b", "c"]);
        assert_eq!(v[1].scope_hint, Some(McpTarget::User));
        assert_eq!(v[2].config["args"], json!(["c.js"]));
    }

    #[test]
    fn import_errors() {
        assert!(parse_import("hello world")
            .unwrap_err()
            .contains("인식할 수 없는"));
        assert!(parse_import("claude mcp add 'bad name' -- node x").is_err());
        assert!(parse_import(r#"{ "bad name": { "command": "x" } }"#).is_err());
        assert!(parse_import("claude mcp add x \"unterminated").is_err());
        assert!(parse_import("claude mcp add --scope global x -- node").is_err());
    }

    #[test]
    fn validate_rules() {
        assert!(validate_config("ok_1.a-b", &json!({ "command": "x" })).is_ok());
        assert!(validate_config("no/slash", &json!({ "command": "x" })).is_err());
        assert!(validate_config("a", &json!({})).is_err());
        assert!(validate_config("a", &json!({ "url": "ftp://x" })).is_err());
        assert!(validate_config("a", &json!({ "type": "stdio", "url": "https://x" })).is_err());
        assert!(validate_config("a", &json!({ "command": "x", "args": [1] })).is_err());
        assert!(validate_config("a", &json!({ "command": "x", "env": { "A": 1 } })).is_err());
    }

    #[test]
    fn warnings() {
        let w = config_warnings(
            "a",
            &json!({ "command": "node", "env": { "API_KEY": "abc123" } }),
        );
        assert_eq!(w.len(), 1);
        assert!(w[0].contains("API_KEY"));
        let w = config_warnings(
            "a",
            &json!({ "command": "node", "env": { "API_KEY": "${API_KEY}", "B": "$X" } }),
        );
        assert!(w.is_empty(), "{w:?}");
        let w = config_warnings(
            "a",
            &json!({ "type": "http", "url": "https://x", "headers": { "Authorization": "Bearer ${T}" } }),
        );
        assert!(w.is_empty());
        let w = config_warnings(
            "a",
            &json!({ "type": "http", "url": "http://example.com/mcp" }),
        );
        assert_eq!(w.len(), 1);
        assert!(config_warnings("a", &json!({ "url": "http://localhost:3000/mcp" })).is_empty());
        let w = config_warnings(
            "a",
            &json!({ "command": "cmd", "args": ["/c", "npx", "-y", "some-pkg"] }),
        );
        assert_eq!(w.len(), 1);
        assert!(w[0].contains("최신"));
        assert!(config_warnings(
            "a",
            &json!({ "command": "cmd", "args": ["/c", "npx", "-y", "@s/pkg@1.2.3"] })
        )
        .is_empty());
        let w = config_warnings("a", &json!({ "command": "npx", "args": ["@s/pkg@1.2.3"] }));
        assert_eq!(w.len(), usize::from(cfg!(windows)));
    }

    #[test]
    fn apply_project_upsert_replace_remove() {
        let text =
            r#"{ "other": 1, "mcpServers": { "a": { "command": "x" }, "b": { "command": "y" } } }"#;
        let out = apply(
            text,
            McpTarget::Project,
            "",
            "a",
            Some(&json!({ "command": "z" })),
        )
        .unwrap();
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["other"], 1);
        let keys: Vec<_> = v["mcpServers"]
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect();
        assert_eq!(keys, ["a", "b"]);
        assert_eq!(v["mcpServers"]["a"]["command"], "z");
        assert!(out.ends_with('\n'));

        let out = apply(
            &out,
            McpTarget::Project,
            "",
            "c",
            Some(&json!({ "command": "c" })),
        )
        .unwrap();
        let v: Value = serde_json::from_str(&out).unwrap();
        let keys: Vec<_> = v["mcpServers"]
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect();
        assert_eq!(keys, ["a", "b", "c"]);

        let out = apply(&out, McpTarget::Project, "", "a", None).unwrap();
        let out = apply(&out, McpTarget::Project, "", "b", None).unwrap();
        let out = apply(&out, McpTarget::Project, "", "c", None).unwrap();
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v, json!({ "other": 1, "mcpServers": {} }));

        let out = apply(
            "",
            McpTarget::Project,
            "",
            "a",
            Some(&json!({ "command": "x" })),
        )
        .unwrap();
        assert_eq!(
            serde_json::from_str::<Value>(&out).unwrap(),
            json!({ "mcpServers": { "a": { "command": "x" } } })
        );
    }

    #[test]
    fn apply_user_preserves_other_keys_and_order() {
        let text = r#"{ "numStartups": 5, "mcpServers": { "a": { "command": "x" } }, "projects": { "D:/p": {} }, "zzz": true }"#;
        let out = apply(
            text,
            McpTarget::User,
            "",
            "b",
            Some(&json!({ "url": "https://x" })),
        )
        .unwrap();
        let v: Value = serde_json::from_str(&out).unwrap();
        let keys: Vec<_> = v.as_object().unwrap().keys().cloned().collect();
        assert_eq!(keys, ["numStartups", "mcpServers", "projects", "zzz"]);
        assert_eq!(v["mcpServers"]["b"]["url"], "https://x");
        assert_eq!(v["projects"], json!({ "D:/p": {} }));
        assert_eq!(
            section(&out, McpTarget::User, "").unwrap()["a"]["command"],
            "x"
        );
    }

    #[test]
    fn apply_local_matches_windows_key_and_creates_entry() {
        let text = r#"{ "projects": { "D:/git/x": { "allowedTools": [], "mcpServers": { "old": { "command": "o" } } } } }"#;
        let out = apply(
            text,
            McpTarget::Local,
            "D:\\git\\x\\",
            "n",
            Some(&json!({ "command": "n" })),
        )
        .unwrap();
        let v: Value = serde_json::from_str(&out).unwrap();
        let projects = v["projects"].as_object().unwrap();
        assert_eq!(projects.len(), 1);
        assert_eq!(v["projects"]["D:/git/x"]["allowedTools"], json!([]));
        assert_eq!(v["projects"]["D:/git/x"]["mcpServers"]["n"]["command"], "n");
        assert_eq!(
            local_servers(&out, "d:\\git\\x").as_object().unwrap().len(),
            2
        );

        let out = apply(&out, McpTarget::Local, "D:\\git\\x", "old", None).unwrap();
        let s = section(&out, McpTarget::Local, "D:/git/x").unwrap();
        assert_eq!(s, json!({ "n": { "command": "n" } }));

        let out = apply(
            text,
            McpTarget::Local,
            "C:\\other\\proj",
            "a",
            Some(&json!({ "command": "a" })),
        )
        .unwrap();
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(
            v["projects"]["C:/other/proj"],
            json!({ "mcpServers": { "a": { "command": "a" } } })
        );
        assert!(v["projects"]["D:/git/x"].is_object());

        assert_eq!(local_servers("{}", "D:/nope"), json!({}));
        assert_eq!(local_servers("not json", "D:/nope"), json!({}));
    }

    #[test]
    fn apply_refuses_invalid_input() {
        let err = apply(
            "{ broken",
            McpTarget::User,
            "",
            "a",
            Some(&json!({ "command": "x" })),
        )
        .unwrap_err();
        assert!(err.contains("올바른 JSON이 아니어서"));
        assert!(apply("[]", McpTarget::User, "", "a", None).is_err());
        assert!(apply(
            "{}",
            McpTarget::User,
            "",
            "bad name",
            Some(&json!({ "command": "x" }))
        )
        .is_err());
        assert!(apply(
            r#"{"mcpServers": 1}"#,
            McpTarget::User,
            "",
            "a",
            Some(&json!({ "command": "x" }))
        )
        .is_err());
        assert!(section("{ broken", McpTarget::Project, "").is_err());
    }
}
