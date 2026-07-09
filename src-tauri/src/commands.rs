//! Tauri command layer: thin wrappers that call into `agentguard_core`.
//!
//! Iteration 0 exposes just enough for the Home screen to render and to prove the
//! app-data bootstrap works. Real commands (open_project, load/save_settings,
//! compute_effective, …) arrive in later iterations — see `docs/architecture.md` §3.

use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    pub data_dir: String,
    pub db_schema_version: i64,
}

/// Basic app info + resolved data directory (used by the Home screen).
#[tauri::command]
pub fn app_info() -> Result<AppInfo, String> {
    let data_dir = agentguard_core::paths::app_data_dir().map_err(|e| e.to_string())?;
    Ok(AppInfo {
        name: "Agent Guard".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        data_dir: data_dir.to_string_lossy().to_string(),
        db_schema_version: agentguard_core::db::SCHEMA_VERSION,
    })
}

/// Recent projects for the Home screen. Iteration 0: empty (persistence lands later).
#[tauri::command]
pub fn list_recent_projects() -> Result<Vec<serde_json::Value>, String> {
    Ok(Vec::new())
}
