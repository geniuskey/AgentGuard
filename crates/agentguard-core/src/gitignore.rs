//! `.gitignore` inspection/suggestion for `settings.local.json` (req §7.3, §9.1),
//! plus git-ignored-path detection and the CLAUDE.md visibility note.
//!
//! Permission rules and search visibility are separate mechanisms in Claude Code:
//! an Allow rule opens *access* to a git-ignored path, but Grep (ripgrep) still
//! skips it. The practical fix is telling the agent about the path in CLAUDE.md —
//! see [`note_allowed_ignored_path`].

use std::path::Path;

/// The entry we recommend for the project `.gitignore`.
pub const LOCAL_SETTINGS_ENTRY: &str = ".claude/settings.local.json";

/// Whether the project's `.gitignore` already ignores the local settings file.
/// Matches the exact recommended entry or a broader `.claude/` ignore.
pub fn ignores_local_settings(gitignore_text: &str) -> bool {
    gitignore_text.lines().map(str::trim).any(|l| {
        l == LOCAL_SETTINGS_ENTRY
            || l == "/.claude/settings.local.json"
            || l == ".claude/"
            || l == "/.claude/"
            || l == ".claude"
    })
}

/// Status of the local-settings gitignore recommendation for a project.
pub fn status(project_root: &Path) -> (bool, bool) {
    let gi = project_root.join(".gitignore");
    let exists = gi.exists();
    let text = std::fs::read_to_string(&gi).unwrap_or_default();
    (exists, ignores_local_settings(&text))
}

/// Append the recommended entry to the project `.gitignore` (creating it if needed).
/// No-op if already ignored.
pub fn add_local_settings_entry(project_root: &Path) -> crate::Result<bool> {
    let gi = project_root.join(".gitignore");
    let mut text = std::fs::read_to_string(&gi).unwrap_or_default();
    if ignores_local_settings(&text) {
        return Ok(false);
    }
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    text.push_str("\n# Agent Guard: per-machine local settings (do not commit)\n");
    text.push_str(LOCAL_SETTINGS_ENTRY);
    text.push('\n');
    std::fs::write(&gi, text)?;
    Ok(true)
}

// --- Git-ignored path detection & agent visibility note --------------------------

/// Gitignore matcher for the project's top-level `.gitignore` (None when absent).
/// Nested `.gitignore` files are not consulted (MVP scope).
pub fn matcher(project_root: &Path) -> Option<ignore::gitignore::Gitignore> {
    let file = project_root.join(".gitignore");
    if !file.exists() {
        return None;
    }
    let mut b = ignore::gitignore::GitignoreBuilder::new(project_root);
    b.add(file);
    b.build().ok()
}

/// True when `rel_path` (or one of its ancestors) is matched by the project's
/// top-level `.gitignore`.
pub fn is_ignored(project_root: &Path, rel_path: &str) -> bool {
    let Some(gi) = matcher(project_root) else {
        return false;
    };
    let abs = project_root.join(rel_path);
    let is_dir = abs.is_dir();
    gi.matched_path_or_any_parents(&abs, is_dir).is_ignore()
}

/// Marker comment identifying the Agent Guard section in CLAUDE.md.
pub const IGNORED_NOTE_MARKER: &str = "<!-- agent-guard:ignored-allowed -->";

/// Record in the project's CLAUDE.md that a git-ignored path is intentionally
/// accessible, so the agent knows to read it by explicit path or search it with
/// `rg --no-ignore`. Idempotent: returns `false` when the entry already exists.
pub fn note_allowed_ignored_path(project_root: &Path, rel_path: &str) -> crate::Result<bool> {
    let md = project_root.join("CLAUDE.md");
    let text = std::fs::read_to_string(&md).unwrap_or_default();
    let entry = format!("- `{rel_path}`");
    if text.lines().any(|l| l.trim() == entry) {
        return Ok(false);
    }

    let new_text = if text.contains(IGNORED_NOTE_MARKER) {
        // Insert after the section's last entry (before the next `## ` heading).
        let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
        let marker_idx = lines
            .iter()
            .position(|l| l.contains(IGNORED_NOTE_MARKER))
            .expect("contains checked above");
        let mut insert_at = None;
        let mut section_end = lines.len();
        // marker_idx + 1 is the section's own `## ` heading — start after it.
        for (i, line) in lines.iter().enumerate().skip(marker_idx + 2) {
            if line.trim_start().starts_with("- `") {
                insert_at = Some(i + 1);
            } else if line.starts_with("## ") {
                section_end = i;
                break;
            }
        }
        lines.insert(insert_at.unwrap_or(section_end), entry);
        let mut s = lines.join("\n");
        s.push('\n');
        s
    } else {
        let mut s = text;
        if !s.is_empty() && !s.ends_with('\n') {
            s.push('\n');
        }
        if !s.is_empty() {
            s.push('\n');
        }
        s.push_str(IGNORED_NOTE_MARKER);
        s.push('\n');
        s.push_str("## 접근 허용된 .gitignore 경로\n\n");
        s.push_str(
            "아래 경로는 `.gitignore`에 있지만 에이전트 접근이 허용되어 있다. Grep 등 검색\n\
             도구에는 나타나지 않을 수 있으니, 경로를 직접 지정해 읽거나 `rg --no-ignore`로\n\
             검색할 것.\n\n",
        );
        s.push_str(&entry);
        s.push('\n');
        s
    };

    std::fs::write(&md, new_text)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn detects_existing_entries() {
        assert!(ignores_local_settings(".claude/settings.local.json\n"));
        assert!(ignores_local_settings("node_modules\n.claude/\n"));
        assert!(!ignores_local_settings("node_modules\ndist\n"));
        assert!(!ignores_local_settings(""));
    }

    fn tmp() -> std::path::PathBuf {
        static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let n = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("ag_gitignore_{}_{n}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn is_ignored_matches_top_level_patterns() {
        let root = tmp();
        fs::write(root.join(".gitignore"), "dist/\n.env\n*.log\n").unwrap();
        fs::create_dir_all(root.join("dist/assets")).unwrap();
        fs::write(root.join(".env"), "X=1").unwrap();

        assert!(is_ignored(&root, "dist"));
        assert!(is_ignored(&root, "dist/assets")); // parent dir ignored
        assert!(is_ignored(&root, ".env"));
        assert!(is_ignored(&root, "debug.log"));
        assert!(!is_ignored(&root, "src"));
    }

    #[test]
    fn no_gitignore_means_nothing_ignored() {
        let root = tmp();
        assert!(!is_ignored(&root, "dist"));
    }

    #[test]
    fn note_creates_section_then_appends_and_dedupes() {
        let root = tmp();
        fs::write(root.join("CLAUDE.md"), "# My project\n\nRules here.\n").unwrap();

        assert!(note_allowed_ignored_path(&root, "dist").unwrap());
        assert!(note_allowed_ignored_path(&root, "data/fixtures").unwrap());
        // Duplicate is a no-op.
        assert!(!note_allowed_ignored_path(&root, "dist").unwrap());

        let text = fs::read_to_string(root.join("CLAUDE.md")).unwrap();
        assert_eq!(text.matches(IGNORED_NOTE_MARKER).count(), 1);
        assert!(text.contains("# My project")); // existing content preserved
        assert!(text.contains("- `dist`"));
        assert!(text.contains("- `data/fixtures`"));
        // Both entries live in the marker section (after the marker line).
        let marker_pos = text.find(IGNORED_NOTE_MARKER).unwrap();
        assert!(text.find("- `dist`").unwrap() > marker_pos);
    }

    #[test]
    fn note_creates_claude_md_when_missing() {
        let root = tmp();
        assert!(note_allowed_ignored_path(&root, "exports").unwrap());
        let text = fs::read_to_string(root.join("CLAUDE.md")).unwrap();
        assert!(text.starts_with(IGNORED_NOTE_MARKER));
        assert!(text.contains("- `exports`"));
    }
}
