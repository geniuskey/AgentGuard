//! Filesystem scanning: lazy directory listing + sensitive-path detection.
//! Uses only `std::fs` (no WebView), so it is unit-testable anywhere.
//! See `docs/risk-scanner.md` (patterns) and `docs/ui-spec.md` §4 (lazy tree).

use crate::risk::RiskSignals;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Directories skipped by default in the tree and scan (req §9.2).
pub const EXCLUDED_DIRS: [&str; 5] = ["node_modules", ".git", ".venv", "dist", "build"];

/// A single entry in the project tree (one depth at a time — lazy).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirEntry {
    /// Project-root-relative POSIX path.
    pub path: String,
    pub name: String,
    pub is_dir: bool,
    /// True for entries in [`EXCLUDED_DIRS`] (shown collapsed/dimmed, not expanded).
    pub excluded: bool,
}

/// List the direct children of `rel_dir` under `project_root` (one level).
pub fn list_dir(project_root: &Path, rel_dir: &str) -> crate::Result<Vec<DirEntry>> {
    let abs = if rel_dir.is_empty() {
        project_root.to_path_buf()
    } else {
        project_root.join(rel_dir)
    };
    let mut entries = Vec::new();
    for e in std::fs::read_dir(&abs)? {
        let e = e?;
        let name = e.file_name().to_string_lossy().to_string();
        let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
        let rel = if rel_dir.is_empty() {
            name.clone()
        } else {
            format!("{rel_dir}/{name}")
        };
        entries.push(DirEntry {
            path: rel,
            name: name.clone(),
            is_dir,
            excluded: is_dir && EXCLUDED_DIRS.contains(&name.as_str()),
        });
    }
    // Directories first, then files; each alphabetical.
    entries.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then(a.name.cmp(&b.name)));
    Ok(entries)
}

/// Result of a sensitive-path scan.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    pub signals: RiskSignals,
    /// Root-relative paths recommended for Deny.
    pub deny_candidates: Vec<String>,
    /// Root-relative paths recommended for Allow.
    pub allow_candidates: Vec<String>,
}

const DENY_DIRS: [&str; 20] = [
    "secrets",
    "secret",
    "credentials",
    "credential",
    "keys",
    "certs",
    "certificates",
    "raw",
    "data",
    "dataset",
    "datasets",
    "exports",
    "export",
    "backup",
    "backups",
    "dump",
    "dumps",
    "logs",
    "private",
    "confidential",
];
const ALLOW_DIRS: [&str; 6] = ["src", "source", "tests", "test", "docs", "doc"];
const ALLOW_FILES: [&str; 6] = [
    "README.md",
    "CLAUDE.md",
    "AGENTS.md",
    "package.json",
    "pyproject.toml",
    "Cargo.toml",
];

fn is_cert_or_key(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.ends_with(".pem")
        || lower.ends_with(".key")
        || lower.ends_with(".p12")
        || lower.ends_with(".pfx")
        || lower == "id_rsa"
        || lower == "id_ed25519"
}

fn is_env_file(name: &str) -> bool {
    name == ".env" || name.starts_with(".env.")
}

/// Scan the project root (one level deep) for sensitive/allow candidates and derive
/// the [`RiskSignals`] used by risk scoring. Root-level scan keeps it fast and
/// deterministic for the MVP (req §9.2).
pub fn scan(project_root: &Path) -> crate::Result<ScanResult> {
    let mut res = ScanResult::default();
    let mut has_src = false;
    let mut has_sensitive = false;

    for entry in list_dir(project_root, "")? {
        let name = entry.name.as_str();
        if entry.is_dir {
            let lower = name.to_ascii_lowercase();
            if DENY_DIRS.contains(&lower.as_str()) {
                res.deny_candidates.push(entry.path.clone());
                has_sensitive = true;
                match lower.as_str() {
                    "secrets" | "secret" => res.signals.has_secrets_dir = true,
                    "raw" => res.signals.has_raw_dir = true,
                    "data" => res.signals.has_data_dir = true,
                    _ => {}
                }
            }
            if ALLOW_DIRS.contains(&lower.as_str()) {
                res.allow_candidates.push(entry.path.clone());
                if lower == "src" || lower == "source" {
                    has_src = true;
                }
            }
        } else {
            if is_env_file(name) {
                res.deny_candidates.push(entry.path.clone());
                res.signals.has_env = true;
                has_sensitive = true;
            }
            if is_cert_or_key(name) {
                res.deny_candidates.push(entry.path.clone());
                res.signals.has_cert_or_key = true;
                has_sensitive = true;
            }
            if ALLOW_FILES.contains(&name) {
                res.allow_candidates.push(entry.path.clone());
            }
        }
    }

    res.signals.source_mixed_with_sensitive = has_src && has_sensitive;
    res.signals.missing_local_settings = !project_root
        .join(".claude")
        .join("settings.local.json")
        .exists();

    Ok(res)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn tmp() -> std::path::PathBuf {
        let base = std::env::temp_dir().join(format!("ag_fs_scan_{}", std::process::id()));
        // Unique-ish per test via a nested random-free counter using nanos-free approach:
        // use an atomic counter to avoid Date/rand (unavailable in workflow ctx, fine here).
        static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let n = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let dir = base.join(n.to_string());
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn list_dir_sorts_dirs_first_and_marks_excluded() {
        let root = tmp();
        fs::create_dir_all(root.join("src")).unwrap();
        fs::create_dir_all(root.join("node_modules")).unwrap();
        fs::write(root.join("README.md"), "x").unwrap();

        let entries = list_dir(&root, "").unwrap();
        assert_eq!(entries[0].is_dir, true); // dirs first
        let nm = entries.iter().find(|e| e.name == "node_modules").unwrap();
        assert!(nm.excluded);
        assert!(entries.iter().any(|e| e.name == "README.md" && !e.is_dir));
    }

    #[test]
    fn scan_detects_sensitive_and_allow_candidates() {
        let root = tmp();
        fs::create_dir_all(root.join("src")).unwrap();
        fs::create_dir_all(root.join("secrets")).unwrap();
        fs::create_dir_all(root.join("raw")).unwrap();
        fs::write(root.join(".env"), "TOKEN=1").unwrap();
        fs::write(root.join("server.pem"), "x").unwrap();
        fs::write(root.join("README.md"), "x").unwrap();

        let res = scan(&root).unwrap();
        assert!(res.signals.has_secrets_dir);
        assert!(res.signals.has_raw_dir);
        assert!(res.signals.has_env);
        assert!(res.signals.has_cert_or_key);
        assert!(res.signals.source_mixed_with_sensitive);
        assert!(res.signals.missing_local_settings);
        assert!(res.deny_candidates.iter().any(|p| p == ".env"));
        assert!(res.allow_candidates.iter().any(|p| p == "src"));
        assert!(res.allow_candidates.iter().any(|p| p == "README.md"));
    }
}
