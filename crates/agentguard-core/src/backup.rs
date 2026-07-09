//! Backup-before-save and restore (req §8.10, §9.4; `docs/data-model.md` §4).
//!
//! The timestamp is supplied by the caller (the Tauri layer) so this module stays
//! clock-independent and deterministically testable.

use std::path::{Path, PathBuf};

/// Build a backup filename: `{yyyy-MM-dd}_{HHmmss}_{project}_{scope}.json`.
/// `project` is omitted for the user scope.
pub fn backup_filename(timestamp: &str, project: Option<&str>, scope_label: &str) -> String {
    match project {
        Some(p) => format!("{timestamp}_{p}_{scope_label}.json"),
        None => format!("{timestamp}_{scope_label}.json"),
    }
}

/// Copy `original` into `backups_dir` under a timestamped name. Returns the backup
/// path. If `original` does not exist there is nothing to back up (`Ok(None)`).
pub fn backup(
    original: &Path,
    backups_dir: &Path,
    filename: &str,
) -> crate::Result<Option<PathBuf>> {
    if !original.exists() {
        return Ok(None);
    }
    std::fs::create_dir_all(backups_dir)?;
    let dest = backups_dir.join(filename);
    std::fs::copy(original, &dest)?;
    Ok(Some(dest))
}

/// Atomically write `contents` to `target` (temp file in the same dir, then rename),
/// so a crash mid-write never leaves a truncated settings file (req §9.4).
pub fn atomic_write(target: &Path, contents: &str) -> crate::Result<()> {
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = target.with_extension("json.tmp");
    std::fs::write(&tmp, contents)?;
    std::fs::rename(&tmp, target)?;
    Ok(())
}

/// Restore a backup file onto `target` (atomic). Caller should back up the current
/// `target` first if it wants that state preserved.
pub fn restore(backup_path: &Path, target: &Path) -> crate::Result<()> {
    let contents = std::fs::read_to_string(backup_path)?;
    atomic_write(target, &contents)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn tmp() -> PathBuf {
        static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let n = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let dir = std::env::temp_dir()
            .join(format!("ag_backup_{}", std::process::id()))
            .join(n.to_string());
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn filename_formats() {
        assert_eq!(
            backup_filename("2026-07-09_173000", Some("my-project"), "project-settings"),
            "2026-07-09_173000_my-project_project-settings.json"
        );
        assert_eq!(
            backup_filename("2026-07-09_173000", None, "user-settings"),
            "2026-07-09_173000_user-settings.json"
        );
    }

    #[test]
    fn backup_then_restore_round_trips() {
        let root = tmp();
        let original = root.join(".claude").join("settings.json");
        atomic_write(&original, "{\"a\":1}\n").unwrap();

        let backups = root.join("backups");
        let name = backup_filename("2026-07-09_173000", Some("proj"), "project-settings");
        let bpath = backup(&original, &backups, &name).unwrap().unwrap();
        assert!(bpath.exists());

        // Mutate, then restore.
        atomic_write(&original, "{\"a\":2}\n").unwrap();
        restore(&bpath, &original).unwrap();
        assert_eq!(fs::read_to_string(&original).unwrap(), "{\"a\":1}\n");
    }

    #[test]
    fn backup_missing_original_is_none() {
        let root = tmp();
        let res = backup(&root.join("nope.json"), &root.join("backups"), "x.json").unwrap();
        assert!(res.is_none());
    }
}
