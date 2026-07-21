//! Tauri command layer: thin wrappers that call into `agentguard_core`.
//!
//! Timestamps are passed in from the frontend so the core stays clock-independent.
//! See `docs/architecture.md` §3 for the command contract.

use agentguard_core::db::{self, ProjectRecord};
use agentguard_core::effective::{self, EffectivePolicy, ScopedRules};
use agentguard_core::fs_scan::{self, DirEntry, ScanResult};
use agentguard_core::inspect::{self, HookEntry, McpServer};
use agentguard_core::model::{AppliesTo, Policy, PolicyRule, RiskLevel, Scope};
use agentguard_core::policy::{self, Permissions};
use agentguard_core::risk::{self, RiskScore};
use agentguard_core::simulate::{self, SimResult};
use agentguard_core::{backup, paths, settings, settings_lint};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::State;
use tauri_plugin_dialog::DialogExt;

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

/// Privacy-preserving diagnostics: versions and file presence only. No setting
/// values, environment values, project filenames, or credentials are included.
#[tauri::command]
pub fn diagnostic_report(project_root: Option<String>) -> Result<String, String> {
    let data_dir = paths::app_data_dir().map_err(|e| e.to_string())?;
    let home = paths::home_dir().map_err(|e| e.to_string())?;
    let mut lines = vec![
        "# Agent Guard diagnostics".to_string(),
        format!("- appVersion: {}", env!("CARGO_PKG_VERSION")),
        format!("- dbSchemaVersion: {}", db::SCHEMA_VERSION),
        format!("- os: {}", std::env::consts::OS),
        format!("- arch: {}", std::env::consts::ARCH),
        format!("- dataDirectoryExists: {}", data_dir.is_dir()),
        format!(
            "- managedFileTierPresent: {}",
            settings::load_file_managed_settings(&program_files_dir()).is_some()
        ),
        format!(
            "- userSettingsPresent: {}",
            home.join(".claude/settings.json").is_file()
        ),
    ];
    if let Some(root) = project_root.filter(|root| !root.is_empty()) {
        let root = PathBuf::from(root);
        let scope_paths = settings::scope_paths(&root, &home);
        lines.extend([
            format!("- projectDirectoryExists: {}", root.is_dir()),
            format!(
                "- projectSettingsPresent: {}",
                scope_paths.project.is_file()
            ),
            format!("- localSettingsPresent: {}", scope_paths.local.is_file()),
            format!("- projectMcpPresent: {}", root.join(".mcp.json").is_file()),
        ]);
    }
    lines.push("- contentsIncluded: false".to_string());
    Ok(format!("{}\n", lines.join("\n")))
}

/// Everything the Explorer needs right after opening a project.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectView {
    pub project: ProjectRecord,
    pub tree: Vec<DirEntry>,
    pub scan: ScanResult,
    pub sensitive_paths: Vec<db::SensitivePathRecord>,
    pub risk: RiskScore,
    pub has_project_settings: bool,
    pub has_local_settings: bool,
}

fn project_name(path: &Path) -> String {
    path.file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string_lossy().to_string())
}

/// Missing optional configuration is an empty document. Other read failures
/// (ACL denial, sharing violation, invalid encoding) must remain visible so a
/// subsequent save never looks like it is replacing an empty file.
fn read_optional_text(path: &Path) -> Result<String, String> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(text),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(error) => Err(format!("failed to read {}: {error}", path.display())),
    }
}

fn ensure_path_within(path: &Path, root: &Path, label: &str) -> Result<(), String> {
    let resolved_root = root
        .canonicalize()
        .map_err(|error| format!("failed to resolve {}: {error}", root.display()))?;
    let resolved_path = path
        .canonicalize()
        .map_err(|error| format!("failed to resolve {}: {error}", path.display()))?;
    if resolved_path.starts_with(&resolved_root) {
        Ok(())
    } else {
        Err(format!(
            "{label} path is outside Agent Guard storage: {}",
            path.display()
        ))
    }
}

fn sync_sensitive_paths(
    conn: &mut Connection,
    project_id: &str,
    scan: &mut ScanResult,
) -> Result<Vec<db::SensitivePathRecord>, String> {
    db::save_sensitive_paths(conn, project_id, &scan.deny_candidates, "scanner")
        .map_err(|e| e.to_string())?;
    let records = db::list_sensitive_paths(conn, project_id).map_err(|e| e.to_string())?;
    let dismissed: std::collections::HashSet<&str> = records
        .iter()
        .filter(|item| item.dismissed)
        .map(|item| item.path.as_str())
        .collect();
    scan.deny_candidates
        .retain(|candidate| !dismissed.contains(candidate.as_str()));
    Ok(records)
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
    let home = paths::home_dir().map_err(|e| e.to_string())?;
    let mut scan = fs_scan::scan(&root).map_err(|e| e.to_string())?;
    // Hooks / MCP servers act outside the path-permission model — fold them
    // into the risk signals before scoring.
    let surface = agent_surface(&root, &home);
    scan.signals.has_hooks = !surface.hooks.is_empty();
    scan.signals.has_mcp_servers = surface.mcp_servers.iter().any(|server| server.active);
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
    let sensitive_paths = sync_sensitive_paths(&mut conn, &id, &mut scan)?;

    let sp = settings::scope_paths(&root, &home);
    let record = db::get_project_by_path(&conn, &root.to_string_lossy())
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("project disappeared after upsert: {id}"))?;

    Ok(ProjectView {
        project: record,
        tree,
        scan,
        sensitive_paths,
        risk,
        has_project_settings: sp.project.exists(),
        has_local_settings: sp.local.exists(),
    })
}

#[tauri::command]
pub fn list_dir(project_root: String, rel_dir: String) -> Result<Vec<DirEntry>, String> {
    fs_scan::list_dir(Path::new(&project_root), &rel_dir).map_err(|e| e.to_string())
}

/// Non-path tool denies toggled as a group to block Claude's web tools and common
/// command-line HTTP clients. This is defense in depth, not an OS firewall: an
/// allowed arbitrary interpreter can still open sockets.
const WEB_DENY: &[&str] = &[
    "WebSearch",
    "WebFetch",
    "Bash(curl:*)",
    "Bash(wget:*)",
    // Cover cmdlet names, short aliases, and native executable names because alias
    // resolution differs between Windows PowerShell and PowerShell 7.
    "PowerShell(Invoke-WebRequest *)",
    "PowerShell(Invoke-RestMethod *)",
    "PowerShell(Start-BitsTransfer *)",
    "PowerShell(iwr *)",
    "PowerShell(irm *)",
    "PowerShell(curl *)",
    "PowerShell(wget *)",
    "PowerShell(curl.exe *)",
    "PowerShell(wget.exe *)",
];

/// The web/network deny specifiers (single source of truth for the UI toggle).
#[tauri::command]
pub fn web_block_specifiers() -> Vec<String> {
    WEB_DENY.iter().map(|s| s.to_string()).collect()
}

/// Neutral rules for one scope plus app-toggled capability denies.
#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ScopeRules {
    pub rules: Vec<PolicyRule>,
    /// Non-path deny specifiers currently toggled on (subset of [`WEB_DENY`]).
    #[serde(default)]
    pub extra_deny: Vec<String>,
    /// Claude Code administrator switch that makes managed permission rules the
    /// only rules considered for allow/ask/deny decisions.
    #[serde(default)]
    pub enforce_managed_only: bool,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ScopedRulesDto {
    #[serde(default)]
    pub managed: ScopeRules,
    pub user: ScopeRules,
    pub project: ScopeRules,
    pub local: ScopeRules,
}

impl ScopedRulesDto {
    fn to_core(&self) -> ScopedRules {
        let managed_only = self.managed.enforce_managed_only;
        ScopedRules {
            managed: self.managed.rules.clone(),
            user: if managed_only {
                Vec::new()
            } else {
                self.user.rules.clone()
            },
            project: if managed_only {
                Vec::new()
            } else {
                self.project.rules.clone()
            },
            local: if managed_only {
                Vec::new()
            } else {
                self.local.rules.clone()
            },
        }
    }
}

fn scope_file(root: &Path, scope: Scope) -> Result<PathBuf, String> {
    if scope == Scope::Managed {
        return Err("managed settings are read-only".to_string());
    }
    let sp = settings::scope_paths(root, &paths::home_dir().map_err(|e| e.to_string())?);
    Ok(match scope {
        Scope::Managed => unreachable!("handled above"),
        Scope::User => sp.user,
        Scope::Project => sp.project,
        Scope::Local => sp.local,
    })
}

fn program_files_dir() -> PathBuf {
    std::env::var_os("ProgramFiles")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Program Files"))
}

fn managed_scope() -> ScopeRules {
    let program_files = program_files_dir();
    let Some(loaded) = settings::load_file_managed_settings(&program_files) else {
        return ScopeRules::default();
    };
    let (rules, unmanaged) = policy::from_permissions(&loaded.permissions);
    let extra_deny = WEB_DENY
        .iter()
        .filter(|w| unmanaged.deny.iter().any(|d| d == **w))
        .map(|w| w.to_string())
        .collect();
    let enforce_managed_only = loaded
        .raw
        .get("allowManagedPermissionRulesOnly")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    ScopeRules {
        rules,
        extra_deny,
        enforce_managed_only,
    }
}

fn rules_for_scope_mut(dto: &mut ScopedRulesDto, scope: Scope) -> &mut Vec<PolicyRule> {
    match scope {
        Scope::Managed => &mut dto.managed.rules,
        Scope::User => &mut dto.user.rules,
        Scope::Project => &mut dto.project.rules,
        Scope::Local => &mut dto.local.rules,
    }
}

/// Restore SQLite-only annotations after folding the authoritative settings
/// files. Path + policy identifies a row; policy is included so an external
/// allow-to-deny edit never inherits stale annotations from the old decision.
fn merge_stored_rule_metadata(dto: &mut ScopedRulesDto, stored: Vec<db::ScopedRuleRow>) {
    for saved in stored {
        let Some(rule) = rules_for_scope_mut(dto, saved.scope)
            .iter_mut()
            .find(|rule| rule.path == saved.rule.path && rule.policy == saved.rule.policy)
        else {
            continue;
        };
        if saved.rule.tools.is_some() {
            rule.tools = saved.rule.tools;
        }
        rule.reason = saved.rule.reason;
        rule.risk_level = saved.rule.risk_level;
        rule.notes = saved.rule.notes;
    }
}

fn read_scope(root: &Path, scope: Scope) -> Result<ScopeRules, String> {
    let file = scope_file(root, scope)?;
    let text = read_optional_text(&file)?;
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
        extra_deny,
        enforce_managed_only: false,
    })
}

/// Load folded rules for the read-only managed tier and three writable scopes.
#[tauri::command]
pub fn load_settings(state: State<Db>, project_root: String) -> Result<ScopedRulesDto, String> {
    let root = PathBuf::from(&project_root);
    let mut dto = ScopedRulesDto {
        managed: managed_scope(),
        user: read_scope(&root, Scope::User)?,
        project: read_scope(&root, Scope::Project)?,
        local: read_scope(&root, Scope::Local)?,
    };
    if !project_root.is_empty() {
        let conn = state.0.lock().map_err(|e| e.to_string())?;
        if let Some(project) =
            db::get_project_by_path(&conn, &project_root).map_err(|e| e.to_string())?
        {
            let stored = db::load_project_paths(&conn, &project.id).map_err(|e| e.to_string())?;
            merge_stored_rule_metadata(&mut dto, stored);
        }
    }
    Ok(dto)
}

/// Read Claude Code's trust bit plus the number of shared-project allow rules.
/// Never returns any other content from the potentially sensitive ~/.claude.json.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeProjectTrustView {
    pub entry_found: bool,
    pub accepted: bool,
    pub shared_allow_rules: usize,
}

#[tauri::command]
pub fn claude_project_trust_status(project_root: String) -> Result<ClaudeProjectTrustView, String> {
    let root = PathBuf::from(&project_root);
    let home = paths::home_dir().map_err(|e| e.to_string())?;
    let trust = settings::project_trust_status(&root, &home).map_err(|e| e.to_string())?;
    let project_file = settings::scope_paths(&root, &home).project;
    let text = std::fs::read_to_string(project_file).unwrap_or_default();
    let loaded = settings::parse(Scope::Project, &text).map_err(|e| e.to_string())?;
    Ok(ClaudeProjectTrustView {
        entry_found: trust.entry_found,
        accepted: trust.accepted,
        shared_allow_rules: loaded.permissions.allow.len(),
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
    let before = read_optional_text(&file)?;
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
    let after = settings::render(&loaded.raw, &sr.rules, &unmanaged).map_err(|e| e.to_string())?;
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

fn rollback_written_file(file: &Path, previous: Option<&PathBuf>) -> Result<(), String> {
    match previous {
        Some(backup_path) => backup::restore(backup_path, file).map_err(|error| error.to_string()),
        None => match std::fs::remove_file(file) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.to_string()),
        },
    }
}

fn metadata_failure_after_write(file: &Path, previous: Option<&PathBuf>, error: String) -> String {
    match rollback_written_file(file, previous) {
        Ok(()) => format!("metadata update failed; settings file was rolled back: {error}"),
        Err(rollback_error) => format!(
            "metadata update failed and settings rollback also failed: {error}; rollback: {rollback_error}"
        ),
    }
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
        Scope::Managed => return Err("managed settings are read-only".to_string()),
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
    let db_result: Result<(), String> = (|| {
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
        Ok(())
    })();
    if let Err(error) = db_result {
        return Err(metadata_failure_after_write(
            &file,
            backup_path.as_ref(),
            error,
        ));
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

/// Persist (or clear) the profile selected for an existing project.
#[tauri::command]
pub fn set_project_profile(
    state: State<Db>,
    project_id: String,
    profile: Option<String>,
) -> Result<(), String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    let updated = db::set_project_profile(&conn, &project_id, profile.as_deref())
        .map_err(|e| e.to_string())?;
    if updated {
        Ok(())
    } else {
        Err(format!("unknown project: {project_id}"))
    }
}

#[tauri::command]
pub fn list_sensitive_paths(
    state: State<Db>,
    project_id: String,
) -> Result<Vec<db::SensitivePathRecord>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    db::list_sensitive_paths(&conn, &project_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_sensitive_path_dismissed(
    state: State<Db>,
    project_id: String,
    id: String,
    dismissed: bool,
) -> Result<(), String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    let updated = db::set_sensitive_path_dismissed(&conn, &project_id, &id, dismissed)
        .map_err(|e| e.to_string())?;
    if updated {
        Ok(())
    } else {
        Err(format!("unknown sensitive path: {id}"))
    }
}

// --- Iteration 2+: Raw JSON, backups, scanner apply, env, gitignore, report ---

/// Read the raw settings text for a scope (empty string if the file doesn't exist).
#[tauri::command]
pub fn read_raw_settings(project_root: String, scope: Scope) -> Result<String, String> {
    let file = scope_file(Path::new(&project_root), scope)?;
    read_optional_text(&file)
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
        Scope::Managed => return Err("managed settings are read-only".to_string()),
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
        let db_result = (|| {
            let conn = state.0.lock().map_err(|e| e.to_string())?;
            db::record_backup(
                &conn,
                proj_id,
                scope,
                &file.to_string_lossy(),
                &bp.to_string_lossy(),
                &timestamp,
            )
            .map_err(|e| e.to_string())
        })();
        if let Err(error) = db_result {
            return Err(metadata_failure_after_write(
                &file,
                backup_path.as_ref(),
                error,
            ));
        }
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
pub fn preview_backup(state: State<Db>, backup_id: String) -> Result<String, String> {
    let record = {
        let conn = state.0.lock().map_err(|e| e.to_string())?;
        db::get_backup(&conn, &backup_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("unknown backup: {backup_id}"))?
    };
    let backups = paths::backups_dir().map_err(|e| e.to_string())?;
    let backup_path = PathBuf::from(&record.backup_path);
    ensure_path_within(&backup_path, &backups, "backup")?;
    std::fs::read_to_string(&backup_path)
        .map_err(|error| format!("failed to read backup {}: {error}", backup_path.display()))
}

/// Restore a backup onto its original path (backs up current state first).
#[tauri::command]
pub fn restore_backup(
    state: State<Db>,
    backup_id: String,
    timestamp: String,
) -> Result<(), String> {
    let record = {
        let conn = state.0.lock().map_err(|e| e.to_string())?;
        db::get_backup(&conn, &backup_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("unknown backup: {backup_id}"))?
    };
    let target = PathBuf::from(&record.original_path);
    let backups = paths::backups_dir().map_err(|e| e.to_string())?;
    let backup_path = PathBuf::from(&record.backup_path);
    ensure_path_within(&backup_path, &backups, "backup")?;
    let name = format!("{timestamp}_pre-restore.json");
    let pre_restore = backup::backup(&target, &backups, &name).map_err(|e| e.to_string())?;
    if let Some(pre_restore) = &pre_restore {
        let scope: Scope = serde_json::from_value(serde_json::Value::String(record.scope.clone()))
            .map_err(|error| format!("invalid backup scope in database: {error}"))?;
        let conn = state.0.lock().map_err(|e| e.to_string())?;
        db::record_backup(
            &conn,
            record.project_id.as_deref(),
            scope,
            &record.original_path,
            &pre_restore.to_string_lossy(),
            &timestamp,
        )
        .map_err(|error| error.to_string())?;
    }
    backup::restore(&backup_path, &target).map_err(|e| e.to_string())
}

/// Turn scanner recommendations into neutral rules (Deny for sensitive, Allow for source).
#[tauri::command]
pub fn scan_recommendation_rules(
    state: State<Db>,
    project_root: String,
) -> Result<Vec<PolicyRule>, String> {
    let mut scan = fs_scan::scan(Path::new(&project_root)).map_err(|e| e.to_string())?;
    let mut conn = state.0.lock().map_err(|e| e.to_string())?;
    if let Some(project) =
        db::get_project_by_path(&conn, &project_root).map_err(|e| e.to_string())?
    {
        sync_sensitive_paths(&mut conn, &project.id, &mut scan)?;
    }
    Ok(agentguard_core::profiles::baseline_rules(
        agentguard_core::profiles::Profile::Conservative,
        &scan,
    ))
}

/// Baseline rules for a named profile (applied after user confirms in the Diff).
#[tauri::command]
pub fn apply_profile(
    state: State<Db>,
    project_root: String,
    profile: String,
) -> Result<ProfilePlan, String> {
    let mut scan = fs_scan::scan(Path::new(&project_root)).map_err(|e| e.to_string())?;
    let p = match profile.as_str() {
        "conservative" => agentguard_core::profiles::Profile::Conservative,
        "balanced" => agentguard_core::profiles::Profile::Balanced,
        "fast-dev" => agentguard_core::profiles::Profile::FastDev,
        _ => agentguard_core::profiles::Profile::Custom,
    };
    let mut conn = state.0.lock().map_err(|e| e.to_string())?;
    if let Some(project) =
        db::get_project_by_path(&conn, &project_root).map_err(|e| e.to_string())?
    {
        sync_sensitive_paths(&mut conn, &project.id, &mut scan)?;
        db::set_project_profile(&conn, &project.id, Some(&profile)).map_err(|e| e.to_string())?;
    }
    Ok(ProfilePlan {
        rules: agentguard_core::profiles::baseline_rules(p, &scan),
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfilePlan {
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

/// Is this path matched by the project's top-level `.gitignore`?
#[tauri::command]
pub fn path_ignored(project_root: String, rel_path: String) -> Result<bool, String> {
    Ok(agentguard_core::gitignore::is_ignored(
        Path::new(&project_root),
        &rel_path,
    ))
}

/// Note an intentionally-accessible git-ignored path in CLAUDE.md so the agent
/// knows to read it directly / search with `rg --no-ignore`. Returns false when
/// the note already exists.
#[tauri::command]
pub fn note_ignored_path(project_root: String, rel_path: String) -> Result<bool, String> {
    agentguard_core::gitignore::note_allowed_ignored_path(Path::new(&project_root), &rel_path)
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
    let mut scoped = scoped;
    // Managed policy belongs to the administrator and is never portable.
    scoped.managed = ScopeRules::default();
    let tmpl = PolicyTemplate { version: 1, scoped };
    serde_json::to_string_pretty(&tmpl).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn import_template(text: String) -> Result<ScopedRulesDto, String> {
    let tmpl: PolicyTemplate =
        serde_json::from_str(&text).map_err(|e| format!("invalid template: {e}"))?;
    if tmpl.version != 1 {
        return Err(format!(
            "unsupported policy template version: {}",
            tmpl.version
        ));
    }
    let mut scoped = tmpl.scoped;
    scoped.managed = ScopeRules::default();
    Ok(scoped)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectedTextFile {
    pub path: String,
    pub text: String,
}

/// Pick and read a policy template in the trusted Rust layer. The webview never
/// receives an arbitrary-read primitive.
#[tauri::command]
pub async fn pick_policy_template(
    app: tauri::AppHandle,
) -> Result<Option<SelectedTextFile>, String> {
    let Some(selected) = app
        .dialog()
        .file()
        .add_filter("JSON", &["json"])
        .set_title("Agent Guard 정책 템플릿 가져오기")
        .blocking_pick_file()
    else {
        return Ok(None);
    };
    let path = selected.into_path().map_err(|error| error.to_string())?;
    let metadata = std::fs::metadata(&path).map_err(|error| error.to_string())?;
    if metadata.len() > 2 * 1024 * 1024 {
        return Err("policy template is larger than 2 MiB".to_string());
    }
    let text = read_optional_text(&path)?;
    Ok(Some(SelectedTextFile {
        path: path.to_string_lossy().to_string(),
        text,
    }))
}

fn safe_default_name(name: &str, fallback: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|character| {
            if character.is_alphanumeric() || matches!(character, '-' | '_' | '.') {
                character
            } else {
                '_'
            }
        })
        .collect();
    let cleaned = cleaned.trim_matches('.');
    if cleaned.is_empty() {
        fallback.to_string()
    } else {
        cleaned.to_string()
    }
}

/// Save an export only to the path returned by the native dialog owned by this
/// command. Supported kinds deliberately limit extensions and default names.
#[tauri::command]
pub async fn save_export_file(
    app: tauri::AppHandle,
    kind: String,
    default_name: String,
    contents: String,
) -> Result<Option<String>, String> {
    let (label, extension, suffix) = match kind.as_str() {
        "policy-template" => ("JSON", "json", "agentguard-template.json"),
        "policy-report" => ("Markdown", "md", "policy-report.md"),
        _ => return Err(format!("unsupported export kind: {kind}")),
    };
    let base = safe_default_name(&default_name, "agentguard");
    let Some(selected) = app
        .dialog()
        .file()
        .add_filter(label, &[extension])
        .set_file_name(format!("{base}-{suffix}"))
        .set_title("Agent Guard 파일 저장")
        .blocking_save_file()
    else {
        return Ok(None);
    };
    let path = selected.into_path().map_err(|error| error.to_string())?;
    backup::atomic_write(&path, &contents).map_err(|error| error.to_string())?;
    Ok(Some(path.to_string_lossy().to_string()))
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
    /// True when a structured (visual) editor exists for this agent.
    pub structured: bool,
    /// Frontend route to open when the entry is clicked.
    pub route: String,
    pub exists: bool,
}

/// Static registry of known agents and their candidate global config paths.
/// `candidates` are home-relative; the first that exists wins, else the first is used.
type AgentSpec = (
    &'static str,
    &'static str,
    &'static str,
    Vec<&'static str>,
    &'static str,
    bool,
    &'static str,
);

fn agent_specs() -> Vec<AgentSpec> {
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
            "OpenAI Codex CLI — 모델/승인 정책/샌드박스 (TOML)",
            vec![".codex/config.toml"],
            "toml",
            true,
            "/agent?id=codex",
        ),
        (
            "opencode",
            "OpenCode",
            "OpenCode — 모델/공유/permission (JSON)",
            vec![
                ".config/opencode/opencode.json",
                ".config/opencode/opencode.jsonc",
            ],
            "json",
            true,
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
pub fn read_agent_config(agent_id: String) -> Result<String, String> {
    let home = paths::home_dir().map_err(|e| e.to_string())?;
    let agent =
        resolve_agent(&home, &agent_id).ok_or_else(|| format!("unknown agent: {agent_id}"))?;
    read_optional_text(Path::new(&agent.path))
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
    text: String,
    agent_id: String,
    timestamp: String,
) -> Result<SaveResult, String> {
    let home = paths::home_dir().map_err(|e| e.to_string())?;
    let agent =
        resolve_agent(&home, &agent_id).ok_or_else(|| format!("unknown agent: {agent_id}"))?;
    validate_config_text(&text, &agent.format)?;
    let file = PathBuf::from(&agent.path);
    let backups = paths::backups_dir().map_err(|e| e.to_string())?;
    let ext = if agent.format == "toml" {
        "toml"
    } else {
        "json"
    };
    let name = format!("{timestamp}_{agent_id}-global.{ext}");
    let backup_path = backup::backup(&file, &backups, &name).map_err(|e| e.to_string())?;
    backup::atomic_write(&file, &text).map_err(|e| e.to_string())?;
    Ok(SaveResult {
        written: file.to_string_lossy().to_string(),
        backup: backup_path.map(|p| p.to_string_lossy().to_string()),
    })
}

// --- Global folder restrictions & security baseline sets -----------------------
//
// User-scope (global) policy can lock down sensitive locations on the whole PC —
// credentials, keys, cloud config — which matters most on locked-down corporate
// machines. Claude Code (tool-centric) gets structured Deny rules; Codex/OpenCode
// use their own models (sandbox / permission), so their baseline is a config
// patch merged into the existing file.

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
    pat.trim_end_matches("/**")
        .trim_end_matches('/')
        .to_string()
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
/// security baseline. Windows normalizes paths to POSIX, so `//**/…` spans every drive
/// and `~/…` maps to the user profile. See <https://code.claude.com/docs/en/permissions>.
const CLAUDE_BASELINE_DENY: &[&str] = &[
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

/// The Claude Code security baseline as neutral Deny rules (merged into user scope):
/// credentials, secret files, and Windows system directories.
#[tauri::command]
pub fn security_baseline_rules() -> Result<Vec<PolicyRule>, String> {
    let home = paths::home_dir().map_err(|e| e.to_string())?;
    let drive = system_drive(&home);
    let mut patterns: Vec<String> = CLAUDE_BASELINE_DENY.iter().map(|s| s.to_string()).collect();
    // Windows system locations (no-ops on non-Windows).
    patterns.push(format!("//{drive}/Windows/**"));
    patterns.push(format!("//{drive}/ProgramData/**"));

    Ok(patterns
        .into_iter()
        .map(|p| {
            let mut r = PolicyRule::new(p, Policy::Deny, AppliesTo::Pattern);
            r.reason = Some("보안 베이스라인 — 민감 파일/자격증명/시스템 경로 차단".into());
            r.risk_level = Some(RiskLevel::High);
            r
        })
        .collect())
}

const CODEX_BASELINE_TOML: &str = r#"
approval_policy = "untrusted"
sandbox_mode = "workspace-write"

[tools]
web_search = false

[sandbox_workspace_write]
network_access = false
"#;

/// OpenCode security baseline (<https://opencode.ai/docs/permissions>):
/// no conversation sharing/auto-update (network egress), no webfetch/websearch,
/// bash gated with curl/wget denied, credential/secret paths unreadable.
const OPENCODE_BASELINE_JSON: &str = r#"{
  "share": "disabled",
  "autoupdate": false,
  "permission": {
    "edit": "ask",
    "bash": { "*": "ask", "curl *": "deny", "wget *": "deny" },
    "webfetch": "deny",
    "websearch": "deny",
    "external_directory": "ask",
    "read": {
      "*": "allow",
      "~/.ssh/**": "deny",
      "~/.aws/**": "deny",
      "~/.kube/**": "deny",
      "~/.gnupg/**": "deny",
      "**/.env": "deny",
      "**/.env.*": "deny",
      "**/*.pem": "deny",
      "**/*.key": "deny",
      "**/id_rsa": "deny",
      "**/id_ed25519": "deny"
    }
  }
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

/// Merge the agent's security baseline into `current_text` (preserving existing
/// keys) and return the new full config text for the user to review before
/// saving. For Claude Code use [`security_baseline_rules`] instead.
#[tauri::command]
pub fn security_baseline(agent_id: String, current_text: String) -> Result<String, String> {
    match agent_id.as_str() {
        "codex" => {
            let mut base: toml::Value = if current_text.trim().is_empty() {
                toml::Value::Table(Default::default())
            } else {
                toml::from_str(&current_text).map_err(|e| e.to_string())?
            };
            let patch: toml::Value =
                toml::from_str(CODEX_BASELINE_TOML).map_err(|e| e.to_string())?;
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
                serde_json::from_str(OPENCODE_BASELINE_JSON).map_err(|e| e.to_string())?;
            merge_json(&mut base, &patch);
            let mut s = serde_json::to_string_pretty(&base).map_err(|e| e.to_string())?;
            s.push('\n');
            Ok(s)
        }
        other => Err(format!("no security baseline for agent: {other}")),
    }
}

// --- Structured config editing (GUI over raw JSON/TOML) --------------------------
//
// The settings form reads the whole config as a JSON tree and writes one key at a
// time by dotted path, preserving every other key. TOML is converted to/from JSON
// for a uniform frontend contract (comments are not preserved on TOML writes —
// the pre-save diff makes that visible).

fn toml_to_json(v: &toml::Value) -> serde_json::Value {
    match v {
        toml::Value::String(s) => serde_json::Value::String(s.clone()),
        toml::Value::Integer(i) => serde_json::json!(i),
        toml::Value::Float(f) => serde_json::json!(f),
        toml::Value::Boolean(b) => serde_json::Value::Bool(*b),
        toml::Value::Datetime(d) => serde_json::Value::String(d.to_string()),
        toml::Value::Array(a) => serde_json::Value::Array(a.iter().map(toml_to_json).collect()),
        toml::Value::Table(t) => serde_json::Value::Object(
            t.iter()
                .map(|(k, v)| (k.clone(), toml_to_json(v)))
                .collect(),
        ),
    }
}

fn json_to_toml(v: &serde_json::Value) -> Result<toml::Value, String> {
    Ok(match v {
        serde_json::Value::Null => return Err("TOML은 null을 지원하지 않습니다".into()),
        serde_json::Value::Bool(b) => toml::Value::Boolean(*b),
        serde_json::Value::Number(n) => match n.as_i64() {
            Some(i) => toml::Value::Integer(i),
            None => toml::Value::Float(n.as_f64().unwrap_or(0.0)),
        },
        serde_json::Value::String(s) => toml::Value::String(s.clone()),
        serde_json::Value::Array(a) => {
            toml::Value::Array(a.iter().map(json_to_toml).collect::<Result<_, _>>()?)
        }
        serde_json::Value::Object(o) => toml::Value::Table(
            o.iter()
                .map(|(k, v)| Ok((k.clone(), json_to_toml(v)?)))
                .collect::<Result<_, String>>()?,
        ),
    })
}

fn json_set(
    obj: &mut serde_json::Map<String, serde_json::Value>,
    segs: &[&str],
    value: &serde_json::Value,
) {
    if segs.len() == 1 {
        obj.insert(segs[0].to_string(), value.clone());
        return;
    }
    let child = obj
        .entry(segs[0].to_string())
        .or_insert_with(|| serde_json::json!({}));
    if !child.is_object() {
        *child = serde_json::json!({});
    }
    json_set(
        child.as_object_mut().expect("object ensured"),
        &segs[1..],
        value,
    );
}

/// Remove `segs`, pruning parent objects that become empty.
fn json_remove(obj: &mut serde_json::Map<String, serde_json::Value>, segs: &[&str]) {
    if segs.len() == 1 {
        obj.remove(segs[0]);
        return;
    }
    let mut prune = false;
    if let Some(serde_json::Value::Object(child)) = obj.get_mut(segs[0]) {
        json_remove(child, &segs[1..]);
        prune = child.is_empty();
    }
    if prune {
        obj.remove(segs[0]);
    }
}

fn toml_set(table: &mut toml::value::Table, segs: &[&str], value: &toml::Value) {
    if segs.len() == 1 {
        table.insert(segs[0].to_string(), value.clone());
        return;
    }
    let child = table
        .entry(segs[0].to_string())
        .or_insert_with(|| toml::Value::Table(Default::default()));
    if !child.is_table() {
        *child = toml::Value::Table(Default::default());
    }
    toml_set(
        child.as_table_mut().expect("table ensured"),
        &segs[1..],
        value,
    );
}

fn toml_remove(table: &mut toml::value::Table, segs: &[&str]) {
    if segs.len() == 1 {
        table.remove(segs[0]);
        return;
    }
    let mut prune = false;
    if let Some(toml::Value::Table(child)) = table.get_mut(segs[0]) {
        toml_remove(child, &segs[1..]);
        prune = child.is_empty();
    }
    if prune {
        table.remove(segs[0]);
    }
}

/// Parse an agent config into a JSON tree for the structured settings form.
#[tauri::command]
pub fn config_get(text: String, format: String) -> Result<serde_json::Value, String> {
    if text.trim().is_empty() {
        return Ok(serde_json::json!({}));
    }
    match format.as_str() {
        "toml" => {
            let v: toml::Value = toml::from_str(&text).map_err(|e| e.to_string())?;
            Ok(toml_to_json(&v))
        }
        _ => serde_json::from_str(&text).map_err(|e| e.to_string()),
    }
}

/// Set (or remove, when `value` is null/absent) one dotted-path key in a config
/// text, preserving every other key. Returns the new full text.
#[tauri::command]
pub fn config_set_value(
    text: String,
    format: String,
    path: String,
    value: Option<serde_json::Value>,
) -> Result<String, String> {
    let segs: Vec<&str> = path.split('.').collect();
    if segs.iter().any(|s| s.is_empty()) {
        return Err(format!("invalid path: {path}"));
    }
    let set = matches!(&value, Some(v) if !v.is_null());

    match format.as_str() {
        "toml" => {
            let mut root: toml::Value = if text.trim().is_empty() {
                toml::Value::Table(Default::default())
            } else {
                toml::from_str(&text).map_err(|e| e.to_string())?
            };
            {
                let table = root
                    .as_table_mut()
                    .ok_or_else(|| "config root must be a TOML table".to_string())?;
                if set {
                    toml_set(table, &segs, &json_to_toml(value.as_ref().expect("set"))?);
                } else {
                    toml_remove(table, &segs);
                }
            }
            toml::to_string(&root).map_err(|e| e.to_string())
        }
        _ => {
            let mut root: serde_json::Value = if text.trim().is_empty() {
                serde_json::json!({})
            } else {
                serde_json::from_str(&text).map_err(|e| e.to_string())?
            };
            {
                let obj = root
                    .as_object_mut()
                    .ok_or_else(|| "config root must be a JSON object".to_string())?;
                if set {
                    json_set(obj, &segs, value.as_ref().expect("set"));
                } else {
                    json_remove(obj, &segs);
                }
            }
            let mut s = serde_json::to_string_pretty(&root).map_err(|e| e.to_string())?;
            s.push('\n');
            Ok(s)
        }
    }
}

/// Lint Claude Code settings text: known-key type/enum checks + plaintext-secret
/// detection in `env`. Delegates to the unit-tested core module.
#[tauri::command]
pub fn lint_claude_settings(text: String) -> Result<Vec<settings_lint::LintItem>, String> {
    Ok(settings_lint::lint_text(&text))
}

// --- Agent security status summary ----------------------------------------------

/// One security-relevant setting extracted from an agent config, for the
/// structured summary above the raw editor.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSecItem {
    pub label: String,
    pub value: String,
    /// `Some(true)` = safe, `Some(false)` = needs attention, `None` = informational.
    pub ok: Option<bool>,
}

fn sec(label: &str, value: impl Into<String>, ok: Option<bool>) -> AgentSecItem {
    AgentSecItem {
        label: label.to_string(),
        value: value.into(),
        ok,
    }
}

/// Render an OpenCode permission value (string or `{"*": ..., pattern: ...}` map).
fn opencode_perm(v: Option<&serde_json::Value>, default: &str) -> (String, Option<bool>) {
    let (base, extra) = match v {
        Some(serde_json::Value::String(s)) => (s.clone(), 0),
        Some(serde_json::Value::Object(m)) => {
            let base = m
                .get("*")
                .and_then(|x| x.as_str())
                .unwrap_or(default)
                .to_string();
            (
                base,
                m.len().saturating_sub(usize::from(m.contains_key("*"))),
            )
        }
        _ => (format!("{default} (기본값)"), 0),
    };
    let ok = if base.starts_with("allow") {
        Some(false)
    } else {
        Some(true)
    };
    let value = if extra > 0 {
        format!("{base} + 패턴 {extra}개")
    } else {
        base
    };
    (value, ok)
}

fn opencode_security_status(text: &str) -> Result<Vec<AgentSecItem>, String> {
    let v: serde_json::Value = if text.trim().is_empty() {
        serde_json::json!({})
    } else {
        serde_json::from_str(text).map_err(|e| e.to_string())?
    };
    let perm = v.get("permission");
    let p = |key: &str| perm.and_then(|p| p.get(key));
    let mut items = Vec::new();

    for (label, key) in [
        ("파일 수정 (edit)", "edit"),
        ("셸 실행 (bash)", "bash"),
        ("웹 요청 (webfetch)", "webfetch"),
        ("웹 검색 (websearch)", "websearch"),
        ("프로젝트 밖 접근", "external_directory"),
    ] {
        // external_directory defaults to ask; the rest default to allow (docs).
        let default = if key == "external_directory" {
            "ask"
        } else {
            "allow"
        };
        let (value, ok) = opencode_perm(p(key), default);
        items.push(sec(label, value, ok));
    }

    let share = v.get("share").and_then(|s| s.as_str());
    items.push(sec(
        "대화 공유 (share)",
        share.unwrap_or("manual (기본값)"),
        match share {
            Some("disabled") => Some(true),
            Some("auto") => Some(false),
            _ => None,
        },
    ));

    let auto = v.get("autoupdate");
    let (av, aok) = match auto {
        Some(serde_json::Value::Bool(false)) => ("false".to_string(), Some(true)),
        Some(serde_json::Value::Bool(true)) => ("true".to_string(), None),
        Some(serde_json::Value::String(s)) => (s.clone(), None),
        _ => ("true (기본값)".to_string(), None),
    };
    items.push(sec("자동 업데이트", av, aok));

    let mcp_count = v
        .get("mcp")
        .and_then(|m| m.as_object())
        .map(|m| m.len())
        .unwrap_or(0);
    items.push(sec("MCP 서버", format!("{mcp_count}개"), None));

    Ok(items)
}

fn codex_security_status(text: &str) -> Result<Vec<AgentSecItem>, String> {
    let v: toml::Value = if text.trim().is_empty() {
        toml::Value::Table(Default::default())
    } else {
        toml::from_str(text).map_err(|e| e.to_string())?
    };
    let s = |path: &[&str]| -> Option<&toml::Value> {
        let mut cur = &v;
        for k in path {
            cur = cur.get(k)?;
        }
        Some(cur)
    };
    let mut items = Vec::new();

    let approval = s(&["approval_policy"]).and_then(|x| x.as_str());
    items.push(sec(
        "승인 정책",
        approval.unwrap_or("(기본값)"),
        match approval {
            Some("untrusted") => Some(true),
            Some("never") => Some(false),
            _ => None,
        },
    ));

    let sandbox = s(&["sandbox_mode"]).and_then(|x| x.as_str());
    items.push(sec(
        "샌드박스",
        sandbox.unwrap_or("(기본값)"),
        match sandbox {
            Some("read-only") | Some("workspace-write") => Some(true),
            Some("danger-full-access") => Some(false),
            _ => None,
        },
    ));

    let net = s(&["sandbox_workspace_write", "network_access"]).and_then(|x| x.as_bool());
    items.push(sec(
        "샌드박스 네트워크",
        match net {
            Some(b) => b.to_string(),
            None => "false (기본값)".to_string(),
        },
        Some(net != Some(true)),
    ));

    let web = s(&["tools", "web_search"]).and_then(|x| x.as_bool());
    items.push(sec(
        "웹 검색 도구",
        match web {
            Some(b) => b.to_string(),
            None => "false (기본값)".to_string(),
        },
        Some(web != Some(true)),
    ));

    let mcp_count = s(&["mcp_servers"])
        .and_then(|m| m.as_table())
        .map(|m| m.len())
        .unwrap_or(0);
    items.push(sec("MCP 서버", format!("{mcp_count}개"), None));

    Ok(items)
}

/// Security-relevant settings summary for an agent config (Codex/OpenCode).
/// Errors on unparseable text — the frontend hides the panel then.
#[tauri::command]
pub fn agent_security_status(agent_id: String, text: String) -> Result<Vec<AgentSecItem>, String> {
    match agent_id.as_str() {
        "opencode" => opencode_security_status(&text),
        "codex" => codex_security_status(&text),
        other => Err(format!("no security status for agent: {other}")),
    }
}

// --- Policy simulator ----------------------------------------------------------

/// Simulate a query against the policy. `kind: "path"` evaluates the current
/// (possibly unsaved) editor rules; `kind: "command"` evaluates the selected
/// shell tool's raw specifiers from the *saved* settings files, since shell
/// rules live outside the neutral path model.
#[tauri::command]
pub fn simulate_access(
    project_root: String,
    scoped: ScopedRulesDto,
    query: String,
    kind: String,
    shell_tool: Option<String>,
) -> Result<SimResult, String> {
    match kind.as_str() {
        "path" => Ok(simulate::simulate_path(&scoped.to_core(), &query)),
        "command" => {
            let root = PathBuf::from(&project_root);
            let mut perms = Vec::new();
            let program_files = std::env::var_os("ProgramFiles")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from(r"C:\Program Files"));
            if let Some(loaded) = settings::load_file_managed_settings(&program_files) {
                perms.push((Scope::Managed, loaded.permissions));
            }
            for scope in [Scope::User, Scope::Project, Scope::Local] {
                let file = scope_file(&root, scope)?;
                let text = std::fs::read_to_string(&file).unwrap_or_default();
                let loaded = settings::parse(scope, &text).map_err(|e| e.to_string())?;
                perms.push((scope, loaded.permissions));
            }
            let tool = match shell_tool.as_deref().unwrap_or("PowerShell") {
                "Bash" => simulate::ShellTool::Bash,
                "PowerShell" => simulate::ShellTool::PowerShell,
                other => return Err(format!("unknown shell tool: {other}")),
            };
            Ok(simulate::simulate_command(&perms, &query, tool))
        }
        other => Err(format!("unknown simulate kind: {other}")),
    }
}

// --- Agent surface: hooks & MCP servers (read-only) ----------------------------

/// Everything configured outside the path-permission model: hooks (arbitrary
/// shell) and MCP servers.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSurface {
    pub hooks: Vec<HookEntry>,
    pub mcp_servers: Vec<McpServer>,
}

fn agent_surface(root: &Path, home: &Path) -> AgentSurface {
    let sp = settings::scope_paths(root, home);
    let mut hooks = Vec::new();
    let mut settings_values = Vec::new();
    for (scope, file) in [
        (Scope::User, &sp.user),
        (Scope::Project, &sp.project),
        (Scope::Local, &sp.local),
    ] {
        let text = std::fs::read_to_string(file).unwrap_or_default();
        if let Ok(loaded) = settings::parse(scope, &text) {
            hooks.extend(inspect::hooks_from_settings(scope, &loaded.raw));
            settings_values.push(loaded.raw);
        }
    }

    if let Some(loaded) = settings::load_file_managed_settings(&program_files_dir()) {
        hooks.extend(inspect::hooks_from_settings(Scope::Managed, &loaded.raw));
        settings_values.push(loaded.raw);
    }

    let mut mcp_servers = Vec::new();
    if let Ok(text) = std::fs::read_to_string(root.join(".mcp.json")) {
        mcp_servers.extend(inspect::parse_mcp_json("project (.mcp.json)", &text));
    }
    if let Ok(text) = std::fs::read_to_string(home.join(".claude.json")) {
        mcp_servers.extend(inspect::parse_claude_json_mcp(
            "user (~/.claude.json)",
            &text,
        ));
    }
    let approval = inspect::project_mcp_approval(&settings_values.iter().collect::<Vec<_>>());
    inspect::apply_project_mcp_approval(&mut mcp_servers, &approval);

    AgentSurface { hooks, mcp_servers }
}

/// Read-only view of hooks + MCP servers for the current project.
#[tauri::command]
pub fn inspect_agent_surface(project_root: String) -> Result<AgentSurface, String> {
    let home = paths::home_dir().map_err(|e| e.to_string())?;
    Ok(agent_surface(Path::new(&project_root), &home))
}

// --- External-change watcher ----------------------------------------------------

/// The active filesystem watcher (one per open project; replaced on re-watch).
pub struct WatchState(pub Mutex<Option<notify::RecommendedWatcher>>);

/// Filenames whose changes are relevant to the open project.
const WATCHED_NAMES: &[&str] = &["settings.json", "settings.local.json", ".mcp.json"];

/// Watch the project's settings files (all scopes) and `.mcp.json`; emits a
/// `settings-file-changed` event with the changed path. Watching the parent
/// directories (non-recursive) also catches files created after watch start.
#[tauri::command]
pub fn watch_project(
    app: tauri::AppHandle,
    state: State<WatchState>,
    project_root: String,
) -> Result<(), String> {
    use notify::{RecursiveMode, Watcher};

    let root = PathBuf::from(&project_root);
    let home = paths::home_dir().map_err(|e| e.to_string())?;
    let sp = settings::scope_paths(&root, &home);

    let dirs: Vec<PathBuf> = {
        let mut v: Vec<PathBuf> = [&sp.user, &sp.project, &sp.local, &root.join(".mcp.json")]
            .iter()
            .filter_map(|f| f.parent().map(|p| p.to_path_buf()))
            .collect();
        v.sort();
        v.dedup();
        v
    };

    let mut watcher =
        notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
            use notify::EventKind;
            let Ok(ev) = res else { return };
            if !matches!(
                ev.kind,
                EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
            ) {
                return;
            }
            for p in &ev.paths {
                let name = p.file_name().map(|n| n.to_string_lossy().to_string());
                if name.is_some_and(|n| WATCHED_NAMES.contains(&n.as_str())) {
                    let _ = tauri::Emitter::emit(
                        &app,
                        "settings-file-changed",
                        p.to_string_lossy().to_string(),
                    );
                }
            }
        })
        .map_err(|e| e.to_string())?;

    for d in dirs {
        if d.is_dir() {
            watcher
                .watch(&d, RecursiveMode::NonRecursive)
                .map_err(|e| e.to_string())?;
        }
    }

    // Replacing the previous watcher drops it, which stops its watches.
    *state.0.lock().map_err(|e| e.to_string())? = Some(watcher);
    Ok(())
}

/// Stop watching (e.g. when leaving the project page).
#[tauri::command]
pub fn unwatch_project(state: State<WatchState>) -> Result<(), String> {
    *state.0.lock().map_err(|e| e.to_string())? = None;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stored_metadata_is_restored_only_to_matching_scope_path_and_policy() {
        let mut dto = ScopedRulesDto {
            project: ScopeRules {
                rules: vec![
                    PolicyRule::new("src", Policy::Allow, AppliesTo::FolderAndChildren),
                    PolicyRule::new("secret", Policy::Deny, AppliesTo::FolderAndChildren),
                ],
                ..Default::default()
            },
            ..Default::default()
        };
        let mut saved = PolicyRule::new("src", Policy::Allow, AppliesTo::FolderAndChildren);
        saved.tools = Some(vec![agentguard_core::Tool::Read]);
        saved.reason = Some("source code".into());
        saved.risk_level = Some(RiskLevel::Low);
        saved.notes = Some("reviewed".into());
        // Same path but a stale policy must not annotate the external deny rule.
        let mut stale = PolicyRule::new("secret", Policy::Allow, AppliesTo::FolderAndChildren);
        stale.notes = Some("must not leak".into());

        merge_stored_rule_metadata(
            &mut dto,
            vec![
                db::ScopedRuleRow {
                    scope: Scope::Project,
                    rule: saved,
                },
                db::ScopedRuleRow {
                    scope: Scope::Project,
                    rule: stale,
                },
            ],
        );

        let src = &dto.project.rules[0];
        assert_eq!(src.tools, Some(vec![agentguard_core::Tool::Read]));
        assert_eq!(src.reason.as_deref(), Some("source code"));
        assert_eq!(src.risk_level, Some(RiskLevel::Low));
        assert_eq!(src.notes.as_deref(), Some("reviewed"));
        assert_eq!(dto.project.rules[1].notes, None);
    }

    #[test]
    fn managed_scope_has_no_writable_settings_path() {
        assert_eq!(
            scope_file(Path::new("."), Scope::Managed).unwrap_err(),
            "managed settings are read-only"
        );
    }

    #[test]
    fn managed_only_switch_excludes_lower_scope_rules_from_effective_policy() {
        let dto = ScopedRulesDto {
            managed: ScopeRules {
                rules: vec![PolicyRule::new(
                    "managed",
                    Policy::Deny,
                    AppliesTo::FolderAndChildren,
                )],
                enforce_managed_only: true,
                ..Default::default()
            },
            user: ScopeRules {
                rules: vec![PolicyRule::new(
                    "user",
                    Policy::Allow,
                    AppliesTo::FolderAndChildren,
                )],
                ..Default::default()
            },
            ..Default::default()
        };

        let core = dto.to_core();
        assert_eq!(core.managed.len(), 1);
        assert!(core.user.is_empty());
        assert!(core.project.is_empty());
        assert!(core.local.is_empty());
    }

    #[test]
    fn web_block_covers_windows_and_bash_http_clients() {
        let unique: std::collections::HashSet<_> = WEB_DENY.iter().collect();
        assert_eq!(unique.len(), WEB_DENY.len(), "duplicate web deny rule");

        let perms = vec![(
            Scope::User,
            Permissions {
                deny: WEB_DENY.iter().map(|rule| rule.to_string()).collect(),
                ..Default::default()
            },
        )];
        for command in ["curl https://example.com", "wget https://example.com"] {
            let result = simulate::simulate_command(&perms, command, simulate::ShellTool::Bash);
            assert_eq!(result.decision, Policy::Deny, "{command}");
        }
        for command in [
            "Invoke-WebRequest https://example.com",
            "Invoke-RestMethod https://example.com",
            "Start-BitsTransfer https://example.com C:/Temp/probe",
            "iwr https://example.com",
            "irm https://example.com",
            "curl https://example.com",
            "wget https://example.com",
            "curl.exe https://example.com",
            "wget.exe https://example.com",
        ] {
            let result =
                simulate::simulate_command(&perms, command, simulate::ShellTool::PowerShell);
            assert_eq!(result.decision, Policy::Deny, "{command}");
        }
    }
}
