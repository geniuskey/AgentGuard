//! Tauri command layer: thin wrappers that call into `agentguard_core`.
//!
//! Timestamps are passed in from the frontend so the core stays clock-independent.
//! See `docs/architecture.md` §3 for the command contract.

use agentguard_core::db::{self, ProjectRecord};
use agentguard_core::effective::{self, EffectivePolicy, ScopedRules};
use agentguard_core::fs_scan::{self, DirEntry, ScanResult};
use agentguard_core::model::{PolicyRule, Scope};
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

/// Neutral rules for one scope plus its defaultMode.
#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ScopeRules {
    pub rules: Vec<PolicyRule>,
    pub default_mode: Option<String>,
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
    let (rules, _unmanaged) = policy::from_permissions(&loaded.permissions);
    Ok(ScopeRules {
        rules,
        default_mode: loaded.default_mode,
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
    let (_managed, unmanaged) = policy::from_permissions(&loaded.permissions);
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

    // Persist rule metadata + backup record to DB.
    {
        let mut conn = state.0.lock().map_err(|e| e.to_string())?;
        db::save_project_paths(
            &mut conn,
            &project_id,
            scope,
            &scope_rules.rules,
            &timestamp,
        )
        .map_err(|e| e.to_string())?;
        if let Some(bp) = &backup_path {
            db::record_backup(
                &conn,
                Some(&project_id),
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
        let conn = state.0.lock().map_err(|e| e.to_string())?;
        db::record_backup(
            &conn,
            Some(&project_id),
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
