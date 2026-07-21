//! Backup-before-save and restore (req §8.10, §9.4; `docs/data-model.md` §4).
//!
//! The timestamp is supplied by the caller (the Tauri layer) so this module stays
//! clock-independent and deterministically testable.

use std::{
    fs::{File, OpenOptions},
    io,
    path::{Path, PathBuf},
};

fn validate_filename(filename: &str) -> crate::Result<()> {
    let path = Path::new(filename);
    if filename.is_empty()
        || path.file_name().is_none()
        || path.file_name() != Some(filename.as_ref())
        || path.components().count() != 1
        || filename.chars().any(|ch| {
            ch.is_control() || matches!(ch, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*')
        })
    {
        return Err(crate::Error::Other(
            "backup filename must be a single safe path component".into(),
        ));
    }
    Ok(())
}

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
    let mut source = match File::open(original) {
        Ok(source) => source,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    validate_filename(filename)?;
    std::fs::create_dir_all(backups_dir)?;
    let path = Path::new(filename);
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("backup");
    let extension = path.extension().and_then(|s| s.to_str());
    let mut first_attempt = true;

    loop {
        let candidate_name = if first_attempt {
            first_attempt = false;
            filename.to_string()
        } else {
            let suffix = uuid::Uuid::new_v4();
            match extension {
                Some(ext) => format!("{stem}_{suffix}.{ext}"),
                None => format!("{stem}_{suffix}"),
            }
        };
        let dest = backups_dir.join(candidate_name);
        let mut destination = match OpenOptions::new().write(true).create_new(true).open(&dest) {
            Ok(destination) => destination,
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.into()),
        };

        if let Err(error) = io::copy(&mut source, &mut destination) {
            drop(destination);
            let _ = std::fs::remove_file(&dest);
            return Err(error.into());
        }
        return Ok(Some(dest));
    }
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

    #[test]
    fn repeated_backup_never_overwrites_existing_copy() {
        let root = tmp();
        let original = root.join("settings.json");
        atomic_write(&original, "{\"version\":1}\n").unwrap();
        let backups = root.join("backups");

        let first = backup(&original, &backups, "same.json").unwrap().unwrap();
        atomic_write(&original, "{\"version\":2}\n").unwrap();
        let second = backup(&original, &backups, "same.json").unwrap().unwrap();

        assert_ne!(first, second);
        assert_eq!(fs::read_to_string(first).unwrap(), "{\"version\":1}\n");
        assert_eq!(fs::read_to_string(second).unwrap(), "{\"version\":2}\n");
    }

    #[test]
    fn backup_rejects_path_traversal_filename() {
        let root = tmp();
        let original = root.join("settings.json");
        atomic_write(&original, "{}\n").unwrap();

        let err = backup(&original, &root.join("backups"), "../escaped.json")
            .unwrap_err()
            .to_string();
        assert!(err.contains("single safe path component"));
        assert!(!root.join("escaped.json").exists());
    }

    #[test]
    fn backup_rejects_windows_alternate_data_stream_filename() {
        let root = tmp();
        let original = root.join("settings.json");
        atomic_write(&original, "{}\n").unwrap();

        let err = backup(&original, &root.join("backups"), "settings.json:payload")
            .unwrap_err()
            .to_string();
        assert!(err.contains("single safe path component"));
    }
}
