//! `.gitignore` inspection/suggestion for `settings.local.json` (req §7.3, §9.1).

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_existing_entries() {
        assert!(ignores_local_settings(".claude/settings.local.json\n"));
        assert!(ignores_local_settings("node_modules\n.claude/\n"));
        assert!(!ignores_local_settings("node_modules\ndist\n"));
        assert!(!ignores_local_settings(""));
    }
}
