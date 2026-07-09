//! App data directory resolution (see `docs/data-model.md` §1).
//!
//! Windows: `%APPDATA%\AgentGuard\`. Other platforms use the standard config dir
//! so the core stays testable/runnable off-Windows.

use std::path::PathBuf;

const APP_DIR_NAME: &str = "AgentGuard";

/// Root app-data directory (created lazily by [`ensure_app_data_dir`]).
pub fn app_data_dir() -> crate::Result<PathBuf> {
    let base = directories::BaseDirs::new()
        .ok_or_else(|| crate::Error::Other("cannot resolve home directory".into()))?;
    // config_dir() maps to %APPDATA% (Roaming) on Windows.
    Ok(base.config_dir().join(APP_DIR_NAME))
}

/// Ensure the app-data dir and `backups/` subdir exist; return the app-data dir.
pub fn ensure_app_data_dir() -> crate::Result<PathBuf> {
    let dir = app_data_dir()?;
    std::fs::create_dir_all(dir.join("backups"))?;
    Ok(dir)
}

/// Path to the SQLite database file.
pub fn db_path() -> crate::Result<PathBuf> {
    Ok(app_data_dir()?.join("app.db"))
}

/// Path to `app-config.json`.
pub fn app_config_path() -> crate::Result<PathBuf> {
    Ok(app_data_dir()?.join("app-config.json"))
}

/// Directory where timestamped settings backups are written.
pub fn backups_dir() -> crate::Result<PathBuf> {
    Ok(app_data_dir()?.join("backups"))
}
