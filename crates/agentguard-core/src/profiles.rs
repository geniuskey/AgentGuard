//! Security profiles → baseline rule mapping.
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
    /// Only sensitive patterns denied; everything else open.
    FastDev,
    /// No automatic rules; the user drives everything.
    Custom,
}

/// Infer how a scanned candidate path should be applied.
fn infer_applies_to(path: &str) -> AppliesTo {
    let name = path.rsplit('/').next().unwrap_or(path);
    if name == ".env" || name.starts_with(".env.") {
        AppliesTo::Pattern
    } else if name.contains('.') {
        // has an extension -> treat as a single file
        AppliesTo::File
    } else {
        AppliesTo::FolderAndChildren
    }
}

/// Build the baseline rule set for `profile` from a project scan.
///
/// Returned rules are candidates for the Project scope; the user reviews them in the
/// Preview/Diff before saving. Conservative and Balanced deny sensitive paths and
/// allow source; Fast Dev only denies; Custom returns nothing.
pub fn baseline_rules(profile: Profile, scan: &ScanResult) -> Vec<PolicyRule> {
    let mut rules = Vec::new();
    let deny = |p: &str| PolicyRule::new(p, Policy::Deny, infer_applies_to(p));
    let allow = |p: &str| PolicyRule::new(p, Policy::Allow, infer_applies_to(p));

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
        assert_eq!(infer_applies_to(".env"), AppliesTo::Pattern);
        assert_eq!(infer_applies_to("README.md"), AppliesTo::File);
        assert_eq!(infer_applies_to("src"), AppliesTo::FolderAndChildren);
    }

    #[test]
    fn custom_returns_nothing() {
        assert!(baseline_rules(Profile::Custom, &scan()).is_empty());
    }
}
