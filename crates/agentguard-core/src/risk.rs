//! Deterministic project risk scoring (see `docs/risk-scanner.md` §4).

use crate::model::RiskLevel;
use serde::{Deserialize, Serialize};

/// Signals detected in a project, fed into [`score`]. Each flag contributes at most once.
#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct RiskSignals {
    pub has_env: bool,
    pub has_secrets_dir: bool,
    pub has_raw_dir: bool,
    pub has_data_dir: bool,
    pub has_cert_or_key: bool,
    /// `src/` exists alongside at least one sensitive item.
    pub source_mixed_with_sensitive: bool,
    /// `.claude/settings.local.json` is absent.
    pub missing_local_settings: bool,
    /// Effective policy is essentially unrestricted (root allow, no deny/ask).
    pub all_paths_allowed: bool,
    /// Hooks are configured in any scope (they run arbitrary shell commands
    /// outside the permission rules).
    pub has_hooks: bool,
    /// MCP servers are configured (they operate outside the file-permission model).
    pub has_mcp_servers: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RiskScore {
    pub score: u32,
    pub level: RiskLevel,
}

/// Compute the risk score (clamped to 100) and grade.
///
/// Weights mirror requirements §8.12 / `docs/risk-scanner.md` §4.
pub fn score(s: &RiskSignals) -> RiskScore {
    let mut score = 0u32;
    if s.has_env {
        score += 20;
    }
    if s.has_secrets_dir {
        score += 30;
    }
    if s.has_raw_dir {
        score += 20;
    }
    if s.has_data_dir {
        score += 15;
    }
    if s.has_cert_or_key {
        score += 30;
    }
    if s.source_mixed_with_sensitive {
        score += 20;
    }
    if s.missing_local_settings {
        score += 5;
    }
    if s.all_paths_allowed {
        score += 50;
    }
    if s.has_hooks {
        score += 15;
    }
    if s.has_mcp_servers {
        score += 10;
    }
    let score = score.min(100);
    RiskScore {
        score,
        level: grade(score),
    }
}

/// Map a score to a grade (requirements §8.12).
pub fn grade(score: u32) -> RiskLevel {
    match score {
        0..=20 => RiskLevel::Low,
        21..=60 => RiskLevel::Medium,
        _ => RiskLevel::High,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pixel_tools_example_is_high() {
        // docs/risk-scanner.md §6: .env + secrets/ + raw/ + data/ + mixed = 105 -> clamp 100 -> High
        let s = RiskSignals {
            has_env: true,
            has_secrets_dir: true,
            has_raw_dir: true,
            has_data_dir: true,
            source_mixed_with_sensitive: true,
            ..Default::default()
        };
        let r = score(&s);
        assert_eq!(r.score, 100);
        assert_eq!(r.level, RiskLevel::High);
    }

    #[test]
    fn empty_project_is_low() {
        let r = score(&RiskSignals::default());
        assert_eq!(r.score, 0);
        assert_eq!(r.level, RiskLevel::Low);
    }

    #[test]
    fn grade_boundaries() {
        assert_eq!(grade(20), RiskLevel::Low);
        assert_eq!(grade(21), RiskLevel::Medium);
        assert_eq!(grade(60), RiskLevel::Medium);
        assert_eq!(grade(61), RiskLevel::High);
    }

    #[test]
    fn hooks_and_mcp_add_risk() {
        let s = RiskSignals {
            has_hooks: true,
            has_mcp_servers: true,
            ..Default::default()
        };
        let r = score(&s);
        assert_eq!(r.score, 25);
        assert_eq!(r.level, RiskLevel::Medium);
    }

    #[test]
    fn missing_local_settings_alone_is_low() {
        let s = RiskSignals {
            missing_local_settings: true,
            ..Default::default()
        };
        assert_eq!(score(&s).level, RiskLevel::Low);
    }
}
