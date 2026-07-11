//! Tauri command layer: thin wrappers that call into `agentguard_core`.
//!
//! Timestamps are passed in from the frontend so the core stays clock-independent.
//! See `docs/architecture.md` §3 for the command contract.

use agentguard_core::db::{self, ProjectRecord};
use agentguard_core::effective::{self, EffectivePolicy, ScopedRules};
use agentguard_core::fs_scan::{self, DirEntry, ScanResult};
use agentguard_core::model::{AppliesTo, Policy, PolicyRule, RiskLevel, Scope};
use agentguard_core::policy::{self, Permissions};
use agentguard_core::risk::{self, RiskScore};
use agentguard_core::{backup, paths, settings};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::State;

/// SQLite connection held in Tauri managed state.
pub struct Db(pub Mutex<Connection>);

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    pub data_dir: String,
    pub db_schema_version: i64,
}

#[tauri::command]
pub fn app_info() -> Result<AppInfo, String> {
    let data_dir = paths::app_data_dir().map_err(|e| e.to_string())?;
    Ok(AppInfo {
        name: "Agent Guard".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        data_dir: data_dir.to_string_lossy().to_string(),
        db_schema_version: db::SCHEMA_VERSION,
    })
}

/// Everything the Explorer needs right after opening a project.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectView {
    pub project: ProjectRecord,
    pub tree: Vec<DirEntry>,
    pub scan: ScanResult,
    pub risk: RiskScore,
    pub has_project_settings: bool,
    pub has_local_settings: bool,
}

fn project_name(path: &Path) -> String {
    path.file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn open_project(
    state: State<Db>,
    path: String,
    timestamp: String,
) -> Result<ProjectView, String> {
    let root = PathBuf::from(&path);
    if !root.is_dir() {
        return Err(format!("not a directory: {path}"));
    }
    let scan = fs_scan::scan(&root).map_err(|e| e.to_string())?;
    let risk = risk::score(&scan.signals);
    let tree = fs_scan::list_dir(&root, "").map_err(|e| e.to_string())?;

    let record = ProjectRecord {
        id: uuid::Uuid::new_v4().to_string(),
        path: root.to_string_lossy().to_string(),
        name: project_name(&root),
        last_opened_at: timestamp,
        risk_profile: None,
        risk_score: Some(risk.score as i64),
        risk_level: Some(format!("{:?}", risk.level)),
        notes: None,
    };

    let mut conn = state.0.lock().map_err(|e| e.to_string())?;
    let id = db::upsert_project(&conn, &record).map_err(|e| e.to_string())?;
    db::save_sensitive_paths(&mut conn, &id, &scan.deny_candidates, "scanner")
        .map_err(|e| e.to_string())?;

    let sp = settings::scope_paths(&root, &paths::home_dir().map_err(|e| e.to_string())?);
    let mut record = record;
    record.id = id;

    Ok(ProjectView {
        project: record,
        tree,
        scan,
        risk,
        has_project_settings: sp.project.exists(),
        has_local_settings: sp.local.exists(),
    })
}

#[tauri::command]
pub fn list_dir(project_root: String, rel_dir: String) -> Result<Vec<DirEntry>, String> {
    fs_scan::list_dir(Path::new(&project_root), &rel_dir).map_err(|e| e.to_string())
}

/// Non-path tool denies toggled as a group to block the agent's web/network access,
/// so prompts and file contents can't leave the machine (intranet no-exfil). These are
/// tool-level rules, not path rules, so they ride alongside the neutral model as
/// `extra_deny` and are preserved as unmanaged rules in `settings.json`.
const WEB_DENY: &[&str] = &["WebSearch", "WebFetch", "Bash(curl:*)", "Bash(wget:*)"];

/// The web/network deny specifiers (single source of truth for the UI toggle).
#[tauri::command]
pub fn web_block_specifiers() -> Vec<String> {
    WEB_DENY.iter().map(|s| s.to_string()).collect()
}

/// Neutral rules for one scope plus its defaultMode and app-toggled capability denies.
#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ScopeRules {
    pub rules: Vec<PolicyRule>,
    pub default_mode: Option<String>,
    /// Non-path deny specifiers currently toggled on (subset of [`WEB_DENY`]).
    #[serde(default)]
    pub extra_deny: Vec<String>,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ScopedRulesDto {
    pub user: ScopeRules,
    pub project: ScopeRules,
    pub local: ScopeRules,
}

impl ScopedRulesDto {
    fn to_core(&self) -> ScopedRules {
        // Effective defaultMode: Local > Project > User (last one set wins).
        let default_mode = self
            .local
            .default_mode
            .clone()
            .or_else(|| self.project.default_mode.clone())
            .or_else(|| self.user.default_mode.clone());
        ScopedRules {
            user: self.user.rules.clone(),
            project: self.project.rules.clone(),
            local: self.local.rules.clone(),
            default_mode,
        }
    }
}

fn scope_file(root: &Path, scope: Scope) -> Result<PathBuf, String> {
    let sp = settings::scope_paths(root, &paths::home_dir().map_err(|e| e.to_string())?);
    Ok(match scope {
        Scope::User => sp.user,
        Scope::Project => sp.project,
        Scope::Local => sp.local,
    })
}

fn read_scope(root: &Path, scope: Scope) -> Result<ScopeRules, String> {
    let file = scope_file(root, scope)?;
    let text = std::fs::read_to_string(&file).unwrap_or_default();
    let loaded = settings::parse(scope, &text).map_err(|e| e.to_string())?;
    let (rules, unmanaged) = policy::from_permissions(&loaded.permissions);
    // Surface which capability denies are currently present so the UI toggle reflects state.
    let extra_deny = WEB_DENY
        .iter()
        .filter(|w| unmanaged.deny.iter().any(|d| d == **w))
        .map(|w| w.to_string())
        .collect();
    Ok(ScopeRules {
        rules,
        default_mode: loaded.default_mode,
        extra_deny,
    })
}

/// Load the managed (folded) rules for all three scopes.
#[tauri::command]
pub fn load_settings(project_root: String) -> Result<ScopedRulesDto, String> {
    let root = PathBuf::from(&project_root);
    Ok(ScopedRulesDto {
        user: read_scope(&root, Scope::User)?,
        project: read_scope(&root, Scope::Project)?,
        local: read_scope(&root, Scope::Local)?,
    })
}

/// Compute the effective (merged) policy for every distinct rule path.
#[tauri::command]
pub fn compute_effective(scoped: ScopedRulesDto) -> Result<Vec<EffectivePolicy>, String> {
    Ok(effective::compute_all(&scoped.to_core()))
}

/// Effective policy for a single path (used for file-tree badges).
#[tauri::command]
pub fn effective_for(
    scoped: ScopedRulesDto,
    target_path: String,
) -> Result<EffectivePolicy, String> {
    Ok(effective::compute_for(&scoped.to_core(), &target_path))
}

/// The raw `Tool(specifier)` strings a rule set would emit (Raw Rules preview tab).
#[tauri::command]
pub fn to_settings_preview(rules: Vec<PolicyRule>) -> Result<Permissions, String> {
    Ok(policy::to_permissions(&rules))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffView {
    pub path: String,
    pub before: String,
    pub after: String,
    pub changed: bool,
}

fn render_scope(
    root: &Path,
    scope: Scope,
    sr: &ScopeRules,
) -> Result<(PathBuf, String, String), String> {
    let file = scope_file(root, scope)?;
    let before = std::fs::read_to_string(&file).unwrap_or_default();
    let loaded = settings::parse(scope, &before).map_err(|e| e.to_string())?;
    let (_managed, mut unmanaged) = policy::from_permissions(&loaded.permissions);
    // Reconcile app-toggled capability denies: drop all known ones, then add those
    // currently enabled. This lets the toggle both add and remove them cleanly while
    // leaving every other unmanaged rule untouched.
    unmanaged.deny.retain(|d| !WEB_DENY.contains(&d.as_str()));
    for d in &sr.extra_deny {
        if !unmanaged.deny.contains(d) {
            unmanaged.deny.push(d.clone());
        }
    }
    let after = settings::render(
        &loaded.raw,
        &sr.rules,
        &unmanaged,
        sr.default_mode.as_deref(),
    )
    .map_err(|e| e.to_string())?;
    Ok((file, before, after))
}

/// Compute the before/after diff for one scope without writing anything.
#[tauri::command]
pub fn build_diff(
    project_root: String,
    scope: Scope,
    scope_rules: ScopeRules,
) -> Result<DiffView, String> {
    let root = PathBuf::from(&project_root);
    let (file, before, after) = render_scope(&root, scope, &scope_rules)?;
    Ok(DiffView {
        path: file.to_string_lossy().to_string(),
        changed: before != after,
        before,
        after,
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveResult {
    pub written: String,
    pub backup: Option<String>,
}

/// Save one scope: back up the existing file, then atomically write the rendered
/// settings, and persist the rule metadata to SQLite (D3). Never writes on failure.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn save_settings(
    state: State<Db>,
    project_root: String,
    project_id: String,
    scope: Scope,
    scope_rules: ScopeRules,
    timestamp: String,
    project_name: String,
) -> Result<SaveResult, String> {
    let root = PathBuf::from(&project_root);
    // Render first — if conversion fails we abort before touching the disk.
    let (file, _before, after) = render_scope(&root, scope, &scope_rules)?;

    // Back up existing file.
    let backups = paths::backups_dir().map_err(|e| e.to_string())?;
    let scope_label = match scope {
        Scope::User => "user-settings",
        Scope::Project => "project-settings",
        Scope::Local => "local-settings",
    };
    let proj = if scope == Scope::User {
        None
    } else {
        Some(project_name.as_str())
    };
    let name = backup::backup_filename(&timestamp, proj, scope_label);
    let backup_path = backup::backup(&file, &backups, &name).map_err(|e| e.to_string())?;

    // Write atomically.
    backup::atomic_write(&file, &after).map_err(|e| e.to_string())?;

    // Persist rule metadata + backup record to DB. An empty `project_id` means the
    // user scope is being edited without an open project (Home → User settings): there
    // is no project row to attach to, so skip `project_paths` and record a project-less
    // backup.
    let proj_id: Option<&str> = if project_id.is_empty() {
        None
    } else {
        Some(project_id.as_str())
    };
    {
        let mut conn = state.0.lock().map_err(|e| e.to_string())?;
        if let Some(pid) = proj_id {
            db::save_project_paths(&mut conn, pid, scope, &scope_rules.rules, &timestamp)
                .map_err(|e| e.to_string())?;
        }
        if let Some(bp) = &backup_path {
            db::record_backup(
                &conn,
                proj_id,
                scope,
                &file.to_string_lossy(),
                &bp.to_string_lossy(),
                &timestamp,
            )
            .map_err(|e| e.to_string())?;
        }
    }

    Ok(SaveResult {
        written: file.to_string_lossy().to_string(),
        backup: backup_path.map(|p| p.to_string_lossy().to_string()),
    })
}

#[tauri::command]
pub fn list_recent_projects(state: State<Db>) -> Result<Vec<ProjectRecord>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    db::list_recent_projects(&conn, 20).map_err(|e| e.to_string())
}

// --- Iteration 2+: Raw JSON, backups, scanner apply, env, gitignore, report ---

/// Read the raw settings text for a scope (empty string if the file doesn't exist).
#[tauri::command]
pub fn read_raw_settings(project_root: String, scope: Scope) -> Result<String, String> {
    let file = scope_file(Path::new(&project_root), scope)?;
    Ok(std::fs::read_to_string(&file).unwrap_or_default())
}

/// Save raw settings text after validating it as JSON (never writes invalid JSON).
/// Backs up the existing file first (req §8.8, §9.4).
#[tauri::command]
pub fn save_raw_settings(
    state: State<Db>,
    project_root: String,
    project_id: String,
    scope: Scope,
    text: String,
    timestamp: String,
    project_name: String,
) -> Result<SaveResult, String> {
    // Validate structure before touching disk.
    settings::parse(scope, &text).map_err(|e| e.to_string())?;
    let file = scope_file(Path::new(&project_root), scope)?;

    let backups = paths::backups_dir().map_err(|e| e.to_string())?;
    let scope_label = match scope {
        Scope::User => "user-settings",
        Scope::Project => "project-settings",
        Scope::Local => "local-settings",
    };
    let proj = if scope == Scope::User {
        None
    } else {
        Some(project_name.as_str())
    };
    let name = backup::backup_filename(&timestamp, proj, scope_label);
    let backup_path = backup::backup(&file, &backups, &name).map_err(|e| e.to_string())?;
    backup::atomic_write(&file, &text).map_err(|e| e.to_string())?;

    if let Some(bp) = &backup_path {
        // Empty `project_id` = user scope edited without an open project → project-less backup.
        let proj_id: Option<&str> = if project_id.is_empty() {
            None
        } else {
            Some(project_id.as_str())
        };
        let conn = state.0.lock().map_err(|e| e.to_string())?;
        db::record_backup(
            &conn,
            proj_id,
            scope,
            &file.to_string_lossy(),
            &bp.to_string_lossy(),
            &timestamp,
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(SaveResult {
        written: file.to_string_lossy().to_string(),
        backup: backup_path.map(|p| p.to_string_lossy().to_string()),
    })
}

/// Validate JSON text; returns an error message string, or null when valid.
#[tauri::command]
pub fn validate_json(text: String) -> Result<Option<String>, String> {
    if text.trim().is_empty() {
        return Ok(None);
    }
    match serde_json::from_str::<serde_json::Value>(&text) {
        Ok(_) => Ok(None),
        Err(e) => Ok(Some(e.to_string())),
    }
}

#[tauri::command]
pub fn list_backups(state: State<Db>, project_id: String) -> Result<Vec<db::BackupRecord>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    db::list_backups(&conn, &project_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn preview_backup(backup_path: String) -> Result<String, String> {
    std::fs::read_to_string(&backup_path).map_err(|e| e.to_string())
}

/// Restore a backup onto its original path (backs up current state first).
#[tauri::command]
pub fn restore_backup(
    backup_path: String,
    target_path: String,
    timestamp: String,
) -> Result<(), String> {
    let target = PathBuf::from(&target_path);
    let backups = paths::backups_dir().map_err(|e| e.to_string())?;
    let name = format!("{timestamp}_pre-restore.json");
    backup::backup(&target, &backups, &name).map_err(|e| e.to_string())?;
    backup::restore(Path::new(&backup_path), &target).map_err(|e| e.to_string())
}

/// Turn scanner recommendations into neutral rules (Deny for sensitive, Allow for source).
#[tauri::command]
pub fn scan_recommendation_rules(project_root: String) -> Result<Vec<PolicyRule>, String> {
    let scan = fs_scan::scan(Path::new(&project_root)).map_err(|e| e.to_string())?;
    Ok(agentguard_core::profiles::baseline_rules(
        agentguard_core::profiles::Profile::Conservative,
        &scan,
    ))
}

/// Baseline rules for a named profile (applied after user confirms in the Diff).
#[tauri::command]
pub fn apply_profile(project_root: String, profile: String) -> Result<ProfilePlan, String> {
    let scan = fs_scan::scan(Path::new(&project_root)).map_err(|e| e.to_string())?;
    let p = match profile.as_str() {
        "conservative" => agentguard_core::profiles::Profile::Conservative,
        "balanced" => agentguard_core::profiles::Profile::Balanced,
        "fast-dev" => agentguard_core::profiles::Profile::FastDev,
        _ => agentguard_core::profiles::Profile::Custom,
    };
    Ok(ProfilePlan {
        default_mode: p.default_mode().map(|s| s.to_string()),
        rules: agentguard_core::profiles::baseline_rules(p, &scan),
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfilePlan {
    pub default_mode: Option<String>,
    pub rules: Vec<PolicyRule>,
}

#[tauri::command]
pub fn get_env_status() -> Result<agentguard_core::env::EnvStatus, String> {
    Ok(agentguard_core::env::status())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitignoreStatus {
    pub exists: bool,
    pub ignored: bool,
}

#[tauri::command]
pub fn gitignore_status(project_root: String) -> Result<GitignoreStatus, String> {
    let (exists, ignored) = agentguard_core::gitignore::status(Path::new(&project_root));
    Ok(GitignoreStatus { exists, ignored })
}

#[tauri::command]
pub fn add_local_to_gitignore(project_root: String) -> Result<bool, String> {
    agentguard_core::gitignore::add_local_settings_entry(Path::new(&project_root))
        .map_err(|e| e.to_string())
}

/// Generate a Markdown policy report for the current effective policy.
#[tauri::command]
pub fn policy_report(
    project_name: String,
    profile: Option<String>,
    scoped: ScopedRulesDto,
    risk_score: u32,
    risk_level: String,
) -> Result<String, String> {
    let effective = effective::compute_all(&scoped.to_core());
    let level = match risk_level.to_lowercase().as_str() {
        "high" => agentguard_core::model::RiskLevel::High,
        "medium" => agentguard_core::model::RiskLevel::Medium,
        _ => agentguard_core::model::RiskLevel::Low,
    };
    let risk = RiskScore {
        score: risk_score,
        level,
    };
    Ok(agentguard_core::report::markdown(
        &project_name,
        profile.as_deref(),
        &risk,
        &effective,
    ))
}

/// A portable policy template (all scopes' rules), for team sharing (req §15.3).
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PolicyTemplate {
    pub version: u32,
    pub scoped: ScopedRulesDto,
}

#[tauri::command]
pub fn export_template(scoped: ScopedRulesDto) -> Result<String, String> {
    let tmpl = PolicyTemplate { version: 1, scoped };
    serde_json::to_string_pretty(&tmpl).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn import_template(text: String) -> Result<ScopedRulesDto, String> {
    let tmpl: PolicyTemplate =
        serde_json::from_str(&text).map_err(|e| format!("invalid template: {e}"))?;
    Ok(tmpl.scoped)
}

/// Write UTF-8 text to a user-chosen path (used by template/report export).
#[tauri::command]
pub fn write_text_file(path: String, contents: String) -> Result<(), String> {
    std::fs::write(&path, contents).map_err(|e| e.to_string())
}

/// Read UTF-8 text from a user-chosen path (used by template import).
#[tauri::command]
pub fn read_text_file(path: String) -> Result<String, String> {
    std::fs::read_to_string(&path).map_err(|e| e.to_string())
}

// --- Multi-agent global settings hub -----------------------------------------
//
// Beyond Claude Code, other local coding agents keep a global config under the home
// dir (Codex: `~/.codex/config.toml`, OpenCode: `~/.config/opencode/opencode.json`).
// The Home screen lists these so the user can jump straight into each one. Claude
// Code keeps its structured rule editor (`route: "/user"`); the others are edited as
// raw JSON/TOML with validation + backup (`route: "/agent?id=..."`).

/// One agent's global config descriptor for the Home list.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentGlobal {
    pub id: String,
    pub name: String,
    pub description: String,
    /// Absolute path to the resolved config file (first existing candidate, else the
    /// primary/default candidate so the user can create it).
    pub path: String,
    /// `"json"` or `"toml"` — drives validation and formatting.
    pub format: String,
    /// True when a structured (visual) editor exists for this agent (Claude Code).
    pub structured: bool,
    /// Frontend route to open when the entry is clicked.
    pub route: String,
    pub exists: bool,
}

/// Static registry of known agents and their candidate global config paths.
/// `candidates` are home-relative; the first that exists wins, else the first is used.
fn agent_specs() -> Vec<(&'static str, &'static str, &'static str, Vec<&'static str>, &'static str, bool, &'static str)> {
    vec![
        (
            "claude-code",
            "Claude Code",
            "Anthropic Claude Code — 도구 중심 permission (Allow/Ask/Deny)",
            vec![".claude/settings.json"],
            "json",
            true,
            "/user",
        ),
        (
            "codex",
            "Codex CLI",
            "OpenAI Codex CLI — approval_policy / sandbox_mode (TOML)",
            vec![".codex/config.toml"],
            "toml",
            false,
            "/agent?id=codex",
        ),
        (
            "opencode",
            "OpenCode",
            "OpenCode — permission / provider / mcp (JSON)",
            vec![".config/opencode/opencode.json", ".config/opencode/opencode.jsonc"],
            "json",
            false,
            "/agent?id=opencode",
        ),
    ]
}

fn resolve_agent(home: &Path, id: &str) -> Option<AgentGlobal> {
    let (aid, name, desc, candidates, format, structured, route) =
        agent_specs().into_iter().find(|s| s.0 == id)?;
    let chosen = candidates
        .iter()
        .map(|c| home.join(c))
        .find(|p| p.exists())
        .unwrap_or_else(|| home.join(candidates[0]));
    Some(AgentGlobal {
        id: aid.to_string(),
        name: name.to_string(),
        description: desc.to_string(),
        path: chosen.to_string_lossy().to_string(),
        format: format.to_string(),
        structured,
        route: route.to_string(),
        exists: chosen.exists(),
    })
}

/// List every known agent's global config (for the Home hub).
#[tauri::command]
pub fn list_agent_globals() -> Result<Vec<AgentGlobal>, String> {
    let home = paths::home_dir().map_err(|e| e.to_string())?;
    Ok(agent_specs()
        .into_iter()
        .filter_map(|s| resolve_agent(&home, s.0))
        .collect())
}

/// Resolve a single agent global config by id.
#[tauri::command]
pub fn get_agent_global(id: String) -> Result<AgentGlobal, String> {
    let home = paths::home_dir().map_err(|e| e.to_string())?;
    resolve_agent(&home, &id).ok_or_else(|| format!("unknown agent: {id}"))
}

/// Read an agent config file's raw text (empty string if it doesn't exist).
#[tauri::command]
pub fn read_agent_config(path: String) -> Result<String, String> {
    Ok(std::fs::read_to_string(&path).unwrap_or_default())
}

fn validate_config_text(text: &str, format: &str) -> Result<(), String> {
    if text.trim().is_empty() {
        return Ok(());
    }
    match format {
        "json" => serde_json::from_str::<serde_json::Value>(text)
            .map(|_| ())
            .map_err(|e| e.to_string()),
        "toml" => toml::from_str::<toml::Value>(text)
            .map(|_| ())
            .map_err(|e| e.to_string()),
        _ => Ok(()),
    }
}

/// Validate config text for a given format; returns an error message, or null when valid.
#[tauri::command]
pub fn validate_config(text: String, format: String) -> Result<Option<String>, String> {
    Ok(validate_config_text(&text, &format).err())
}

/// Save an agent config after validating it (never writes invalid JSON/TOML).
/// Backs up the existing file first (req §8.10, §9.4).
#[tauri::command]
pub fn save_agent_config(
    path: String,
    text: String,
    format: String,
    agent_id: String,
    timestamp: String,
) -> Result<SaveResult, String> {
    validate_config_text(&text, &format)?;
    let file = PathBuf::from(&path);
    let backups = paths::backups_dir().map_err(|e| e.to_string())?;
    let ext = if format == "toml" { "toml" } else { "json" };
    let name = format!("{timestamp}_{agent_id}-global.{ext}");
    let backup_path = backup::backup(&file, &backups, &name).map_err(|e| e.to_string())?;
    backup::atomic_write(&file, &text).map_err(|e| e.to_string())?;
    Ok(SaveResult {
        written: file.to_string_lossy().to_string(),
        backup: backup_path.map(|p| p.to_string_lossy().to_string()),
    })
}

// --- Global folder restrictions & intranet recommended security sets ----------
//
// User-scope (global) policy can lock down sensitive locations on the whole PC —
// credentials, keys, cloud config — which matters most on locked-down corporate
// intranets. Claude Code (tool-centric) gets structured Deny rules; Codex/OpenCode
// use their own models (sandbox / permission), so their "intranet set" is a config
// baseline merged into the existing file.

/// Convert an absolute folder path (e.g. from the native picker) into a Claude Code
/// global glob specifier. Windows drive paths become `//c/…/**`, home paths `~/…/**`.
/// (Delegates to the unit-tested core helper — see `policy::global_folder_pattern`.)
#[tauri::command]
pub fn home_relative_pattern(path: String) -> Result<String, String> {
    let home = paths::home_dir().map_err(|e| e.to_string())?;
    Ok(policy::global_folder_pattern(&path, &home))
}

// --- System explorer (global/user scope) -------------------------------------

/// One row in the all-drives explorer: an absolute OS path plus the Claude Code
/// pattern base it maps to (`~/x` under home, `//c/x` elsewhere, no `/**` suffix —
/// the frontend appends `/**` for folder rules).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemEntry {
    pub name: String,
    pub path: String,
    pub pattern: String,
    pub is_dir: bool,
}

/// Claude pattern base for an absolute path: the folder pattern without its
/// trailing `/**` (which also yields the correct exact-match pattern for files).
fn pattern_base(path: &Path, home: &Path) -> String {
    let pat = policy::global_folder_pattern(&path.to_string_lossy(), home);
    pat.trim_end_matches("/**").trim_end_matches('/').to_string()
}

/// Roots for the system explorer: the home folder first, then every mounted drive
/// (Windows) or the filesystem root (POSIX).
#[tauri::command]
pub fn list_drives() -> Result<Vec<SystemEntry>, String> {
    let home = paths::home_dir().map_err(|e| e.to_string())?;
    let mut out = vec![SystemEntry {
        name: format!("~ 홈 ({})", home.display()),
        path: home.to_string_lossy().to_string(),
        pattern: "~".to_string(),
        is_dir: true,
    }];
    if cfg!(windows) {
        for letter in b'A'..=b'Z' {
            let root = format!("{}:\\", letter as char);
            if Path::new(&root).exists() {
                out.push(SystemEntry {
                    name: format!("{}: 드라이브", letter as char),
                    path: root.clone(),
                    pattern: pattern_base(Path::new(&root), &home),
                    is_dir: true,
                });
            }
        }
    } else {
        out.push(SystemEntry {
            name: "/".to_string(),
            path: "/".to_string(),
            pattern: "/".to_string(),
            is_dir: true,
        });
    }
    Ok(out)
}

/// List one directory anywhere on the machine (read-only), folders first.
#[tauri::command]
pub fn list_system_dir(path: String) -> Result<Vec<SystemEntry>, String> {
    let home = paths::home_dir().map_err(|e| e.to_string())?;
    let rd = std::fs::read_dir(&path).map_err(|e| e.to_string())?;
    let mut out: Vec<SystemEntry> = rd
        .flatten()
        .map(|ent| {
            let p = ent.path();
            let is_dir = ent.file_type().map(|t| t.is_dir()).unwrap_or(false);
            SystemEntry {
                name: ent.file_name().to_string_lossy().to_string(),
                path: p.to_string_lossy().to_string(),
                pattern: pattern_base(&p, &home),
                is_dir,
            }
        })
        .collect();
    out.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(out)
}

/// Sensitive credential dirs (home) + secret-file globs (all drives) denied by the
/// intranet baseline. Windows normalizes paths to POSIX, so `//**/…` spans every drive
/// and `~/…` maps to the user profile. See <https://code.claude.com/docs/en/permissions>.
const CLAUDE_INTRANET_DENY: &[&str] = &[
    // Credential / cloud config directories under the home profile.
    "~/.ssh/**",
    "~/.aws/**",
    "~/.config/gcloud/**",
    "~/.azure/**",
    "~/.kube/**",
    "~/.gnupg/**",
    "~/.docker/config.json",
    "~/.config/gh/**",
    "~/.netrc",
    "~/.npmrc",
    "~/.pypirc",
    "~/.git-credentials",
    "~/.password-store/**",
    // Secret files anywhere on any drive (`//` = filesystem root, all drives on Windows).
    "//**/*.pem",
    "//**/*.key",
    "//**/*.pfx",
    "//**/*.p12",
    "//**/id_rsa",
    "//**/id_ed25519",
    "//**/.env",
    "//**/.env.*",
];

/// Lowercase system drive letter (Windows), derived from the home dir; `c` as fallback.
fn system_drive(home: &Path) -> char {
    home.to_string_lossy()
        .chars()
        .next()
        .filter(|c| c.is_ascii_alphabetic())
        .map(|c| c.to_ascii_lowercase())
        .unwrap_or('c')
}

/// The Claude Code intranet baseline as neutral Deny rules (merged into user scope):
/// credentials, secret files, and Windows system directories.
#[tauri::command]
pub fn intranet_recommendation_rules() -> Result<Vec<PolicyRule>, String> {
    let home = paths::home_dir().map_err(|e| e.to_string())?;
    let drive = system_drive(&home);
    let mut patterns: Vec<String> = CLAUDE_INTRANET_DENY.iter().map(|s| s.to_string()).collect();
    // Windows system locations (no-ops on non-Windows).
    patterns.push(format!("//{drive}/Windows/**"));
    patterns.push(format!("//{drive}/ProgramData/**"));

    Ok(patterns
        .into_iter()
        .map(|p| {
            let mut r = PolicyRule::new(p, Policy::Deny, AppliesTo::Pattern);
            r.reason = Some("사내 인트라넷 추천 보안 셋 — 민감 파일/자격증명/시스템 경로 차단".into());
            r.risk_level = Some(RiskLevel::High);
            r
        })
        .collect())
}

const CODEX_INTRANET_TOML: &str = r#"
approval_policy = "untrusted"
sandbox_mode = "workspace-write"

[tools]
web_search = false

[sandbox_workspace_write]
network_access = false
"#;

const OPENCODE_INTRANET_JSON: &str = r#"{
  "permission": { "edit": "ask", "bash": "ask", "webfetch": "deny" }
}"#;

fn merge_json(base: &mut serde_json::Value, patch: &serde_json::Value) {
    match (base, patch) {
        (serde_json::Value::Object(b), serde_json::Value::Object(p)) => {
            for (k, v) in p {
                merge_json(b.entry(k.clone()).or_insert(serde_json::Value::Null), v);
            }
        }
        (b, p) => *b = p.clone(),
    }
}

fn merge_toml(base: &mut toml::Value, patch: &toml::Value) {
    match (base, patch) {
        (toml::Value::Table(b), toml::Value::Table(p)) => {
            for (k, v) in p {
                match b.get_mut(k) {
                    Some(bv) => merge_toml(bv, v),
                    None => {
                        b.insert(k.clone(), v.clone());
                    }
                }
            }
        }
        (b, p) => *b = p.clone(),
    }
}

/// Merge the agent's intranet recommended security baseline into `current_text`
/// (preserving existing keys) and return the new full config text for the user to
/// review before saving. For Claude Code use [`intranet_recommendation_rules`] instead.
#[tauri::command]
pub fn intranet_recommendation(agent_id: String, current_text: String) -> Result<String, String> {
    match agent_id.as_str() {
        "codex" => {
            let mut base: toml::Value = if current_text.trim().is_empty() {
                toml::Value::Table(Default::default())
            } else {
                toml::from_str(&current_text).map_err(|e| e.to_string())?
            };
            let patch: toml::Value = toml::from_str(CODEX_INTRANET_TOML).map_err(|e| e.to_string())?;
            merge_toml(&mut base, &patch);
            toml::to_string(&base).map_err(|e| e.to_string())
        }
        "opencode" => {
            let mut base: serde_json::Value = if current_text.trim().is_empty() {
                serde_json::json!({})
            } else {
                serde_json::from_str(&current_text).map_err(|e| e.to_string())?
            };
            let patch: serde_json::Value =
                serde_json::from_str(OPENCODE_INTRANET_JSON).map_err(|e| e.to_string())?;
            merge_json(&mut base, &patch);
            let mut s = serde_json::to_string_pretty(&base).map_err(|e| e.to_string())?;
            s.push('\n');
            Ok(s)
        }
        other => Err(format!("no intranet recommendation for agent: {other}")),
    }
}
