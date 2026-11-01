//! Review candidate construction and legacy profile mapping (not offered in the UI).
//! See `docs/effective-policy.md` §2 and D2.

use crate::fs_scan::ScanResult;
use crate::model::{AppliesTo, Policy, PolicyRule};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Profile {
    /// Sensitive paths denied and general source allowed (strictest curated set).
    Conservative,
    /// Sensitive paths denied, general source allowed, uncertain paths ask.
    Balanced,
    /// Legacy mapping: sensitive candidates only; unmatched access is not decided here.
    FastDev,
    /// No automatic rules; the user drives everything.
    Custom,
}

/// Infer how a scanned candidate path should be applied.
fn infer_applies_to(path: &str, scan: &ScanResult) -> AppliesTo {
    // File kind comes from the filesystem, never from an extension heuristic.
    if scan
        .candidates
        .iter()
        .any(|item| item.path == path && item.is_dir)
    {
        AppliesTo::FolderAndChildren
    } else {
        AppliesTo::File
    }
}

/// Observations for explicit user review. Do not preselect Allow or Deny.
pub fn review_candidates(scan: &ScanResult) -> Vec<PolicyRule> {
    scan.candidates
        .iter()
        .map(|item| {
            let mut rule =
                PolicyRule::new(&item.path, Policy::Ask, infer_applies_to(&item.path, scan));
            rule.tools = Some(vec![crate::Tool::Read]);
            rule.reason = Some(item.reason.clone());
            rule
        })
        .collect()
}

/// Build the baseline rule set for `profile` from a project scan.
///
/// Returned rules are candidates for the Project scope; the user reviews them in the
/// Preview/Diff before saving. Conservative and Balanced deny sensitive paths and
/// allow source; Fast Dev only denies; Custom returns nothing.
pub fn baseline_rules(profile: Profile, scan: &ScanResult) -> Vec<PolicyRule> {
    let mut rules = Vec::new();
    let candidate = |p: &str, policy| {
        let mut rule = PolicyRule::new(p, policy, infer_applies_to(p, scan));
        rule.reason = scan
            .candidates
            .iter()
            .find(|item| item.path == p)
            .map(|item| item.reason.clone());
        rule
    };
    let deny = |p: &str| candidate(p, Policy::Deny);
    let allow = |p: &str| candidate(p, Policy::Allow);

    match profile {
        Profile::Custom => {}
        Profile::FastDev => {
            for p in &scan.deny_candidates {
                rules.push(deny(p));
            }
        }
        Profile::Conservative | Profile::Balanced => {
            for p in &scan.deny_candidates {
                rules.push(deny(p));
            }
            for p in &scan.allow_candidates {
                rules.push(allow(p));
            }
        }
    }
    rules
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs_scan::ScanResult;

    fn scan() -> ScanResult {
        ScanResult {
            deny_candidates: vec!["secrets".into(), ".env".into(), "server.pem".into()],
            allow_candidates: vec!["src".into(), "README.md".into()],
            ..Default::default()
        }
    }

    #[test]
    fn conservative_denies_sensitive_and_allows_source() {
        let rules = baseline_rules(Profile::Conservative, &scan());
        assert!(rules
            .iter()
            .any(|r| r.path == "src" && r.policy == Policy::Allow));
        assert!(rules
            .iter()
            .any(|r| r.path == "secrets" && r.policy == Policy::Deny));
    }

    #[test]
    fn fast_dev_only_denies() {
        let rules = baseline_rules(Profile::FastDev, &scan());
        assert!(rules.iter().all(|r| r.policy == Policy::Deny));
        assert_eq!(rules.len(), 3);
    }

    #[test]
    fn applies_to_inference() {
        let mut scan = scan();
        scan.candidates.push(crate::fs_scan::PathCandidate {
            path: "src".into(),
            is_dir: true,
            reason: String::new(),
        });
        assert_eq!(infer_applies_to(".env", &scan), AppliesTo::File);
        assert_eq!(infer_applies_to("id_rsa", &scan), AppliesTo::File);
        assert_eq!(infer_applies_to("id_ed25519", &scan), AppliesTo::File);
        assert_eq!(infer_applies_to("README.md", &scan), AppliesTo::File);
        assert_eq!(infer_applies_to("src", &scan), AppliesTo::FolderAndChildren);
    }

    #[test]
    fn custom_returns_nothing() {
        assert!(baseline_rules(Profile::Custom, &scan()).is_empty());
    }
    #[test]
    fn scanned_extensionless_keys_emit_exact_file_rules() {
        let root =
            std::env::temp_dir().join(format!("agentguard-candidates-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(root.join("src")).unwrap();
        for key in ["id_rsa", "id_ed25519"] {
            std::fs::write(root.join(key), "test fixture").unwrap();
        }
        let scan = crate::fs_scan::scan(&root).unwrap();
        let candidates = review_candidates(&scan);
        assert!(candidates
            .iter()
            .all(|rule| rule.policy == Policy::Ask && rule.tools == Some(vec![crate::Tool::Read])));
        let emitted = crate::policy::to_permissions(&candidates);
        assert!(emitted.ask.contains(&"Read(./id_rsa)".into()));
        assert!(emitted.ask.contains(&"Read(./id_ed25519)".into()));
        assert!(emitted.ask.contains(&"Read(./src/**)".into()));
        std::fs::remove_dir_all(root).unwrap();
    }
}
