// Prevent an extra console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

/// Ensure the app-data dir exists and the SQLite schema is initialized.
/// Failures are logged but do not abort startup (the window still opens).
fn bootstrap_storage() {
    match agentguard_core::paths::ensure_app_data_dir() {
        Ok(dir) => {
            eprintln!("[agentguard] data dir: {}", dir.display());
            match agentguard_core::paths::db_path().and_then(|p| agentguard_core::db::open(&p)) {
                Ok(_) => eprintln!(
                    "[agentguard] db ready (schema v{})",
                    agentguard_core::db::SCHEMA_VERSION
                ),
                Err(e) => eprintln!("[agentguard] db init failed: {e}"),
            }
        }
        Err(e) => eprintln!("[agentguard] could not create data dir: {e}"),
    }
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|_app| {
            bootstrap_storage();
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::list_recent_projects
        ])
        .run(tauri::generate_context!())
        .expect("error while running Agent Guard");
}
