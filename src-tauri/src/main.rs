// Prevent an extra console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

use commands::Db;
use std::sync::Mutex;

/// Ensure the app-data dir exists and open the SQLite database with the schema
/// initialized. Returns the connection to be held in Tauri state.
fn open_database() -> agentguard_core::Result<rusqlite::Connection> {
    let dir = agentguard_core::paths::ensure_app_data_dir()?;
    eprintln!("[agentguard] data dir: {}", dir.display());
    let db_path = agentguard_core::paths::db_path()?;
    let conn = agentguard_core::db::open(&db_path)?;
    eprintln!(
        "[agentguard] db ready (schema v{})",
        agentguard_core::db::SCHEMA_VERSION
    );
    Ok(conn)
}

fn main() {
    // If storage bootstrap fails we still open the window with an in-memory DB so
    // the app is usable and the error is visible, rather than crashing on launch.
    let conn = open_database().unwrap_or_else(|e| {
        eprintln!("[agentguard] storage bootstrap failed: {e}; using in-memory db");
        let conn = rusqlite::Connection::open_in_memory().expect("in-memory db");
        let _ = agentguard_core::db::init(&conn);
        conn
    });

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(Db(Mutex::new(conn)))
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::open_project,
            commands::list_dir,
            commands::load_settings,
            commands::compute_effective,
            commands::effective_for,
            commands::to_settings_preview,
            commands::build_diff,
            commands::save_settings,
            commands::list_recent_projects,
            commands::read_raw_settings,
            commands::save_raw_settings,
            commands::validate_json,
            commands::list_backups,
            commands::preview_backup,
            commands::restore_backup,
            commands::scan_recommendation_rules,
            commands::apply_profile,
            commands::get_env_status,
            commands::gitignore_status,
            commands::add_local_to_gitignore,
            commands::policy_report,
            commands::export_template,
            commands::import_template,
            commands::write_text_file,
            commands::read_text_file,
            commands::list_agent_globals,
            commands::get_agent_global,
            commands::read_agent_config,
            commands::validate_config,
            commands::save_agent_config,
            commands::home_relative_pattern,
            commands::intranet_recommendation_rules,
            commands::intranet_recommendation,
            commands::web_block_specifiers,
            commands::list_drives,
            commands::list_system_dir,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Agent Guard");
}
