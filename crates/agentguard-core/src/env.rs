//! AWS Bedrock / proxy environment inspection (req §8.13, §10.6).
//!
//! Read-only and value-masking: Agent Guard never stores or transmits secrets (MVP
//! "Won't Have"). We report presence and safe metadata only. See `docs/security.md` §4.

use serde::{Deserialize, Serialize};

/// The environment variables we surface, in display order.
pub const WATCHED_VARS: [&str; 11] = [
    "AWS_REGION",
    "AWS_DEFAULT_REGION",
    "AWS_PROFILE",
    "AWS_ACCESS_KEY_ID",
    "AWS_SECRET_ACCESS_KEY",
    "AWS_SESSION_TOKEN",
    "HTTPS_PROXY",
    "HTTP_PROXY",
    "NO_PROXY",
    "REQUESTS_CA_BUNDLE",
    "SSL_CERT_FILE",
];

/// Variables whose *values* must never be shown (only presence).
const SECRET_VARS: [&str; 2] = ["AWS_SECRET_ACCESS_KEY", "AWS_SESSION_TOKEN"];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EnvVar {
    pub name: String,
    pub present: bool,
    /// Masked/safe display value (empty when absent; `"********"` for secrets).
    pub display: String,
    pub is_secret: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvStatus {
    pub vars: Vec<EnvVar>,
    /// True if a raw secret is present in the environment (informational warning).
    pub has_secret_in_env: bool,
    /// True if `AWS_PROFILE` is set (the recommended auth path).
    pub uses_profile: bool,
}

fn mask(name: &str, value: &str) -> String {
    if SECRET_VARS.contains(&name) {
        "********".to_string()
    } else if name == "AWS_ACCESS_KEY_ID" && value.len() > 4 {
        // Show only the last 4 chars of the access key id.
        format!("****{}", &value[value.len() - 4..])
    } else {
        value.to_string()
    }
}

/// Read the watched variables from a lookup function (injected for testability).
pub fn status_from<F>(lookup: F) -> EnvStatus
where
    F: Fn(&str) -> Option<String>,
{
    let mut vars = Vec::new();
    let mut has_secret_in_env = false;
    let mut uses_profile = false;

    for &name in WATCHED_VARS.iter() {
        let is_secret = SECRET_VARS.contains(&name);
        match lookup(name) {
            Some(v) if !v.is_empty() => {
                if is_secret {
                    has_secret_in_env = true;
                }
                if name == "AWS_PROFILE" {
                    uses_profile = true;
                }
                vars.push(EnvVar {
                    name: name.to_string(),
                    present: true,
                    display: mask(name, &v),
                    is_secret,
                });
            }
            _ => vars.push(EnvVar {
                name: name.to_string(),
                present: false,
                display: String::new(),
                is_secret,
            }),
        }
    }

    EnvStatus {
        vars,
        has_secret_in_env,
        uses_profile,
    }
}

/// Read the watched variables from the real process environment.
pub fn status() -> EnvStatus {
    status_from(|n| std::env::var(n).ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn masks_secrets_and_access_key() {
        let mut m = HashMap::new();
        m.insert("AWS_PROFILE".to_string(), "dev".to_string());
        m.insert(
            "AWS_ACCESS_KEY_ID".to_string(),
            "AKIAEXAMPLE1234".to_string(),
        );
        m.insert(
            "AWS_SECRET_ACCESS_KEY".to_string(),
            "supersecret".to_string(),
        );

        let st = status_from(|n| m.get(n).cloned());
        assert!(st.uses_profile);
        assert!(st.has_secret_in_env);

        let key = st
            .vars
            .iter()
            .find(|v| v.name == "AWS_ACCESS_KEY_ID")
            .unwrap();
        assert_eq!(key.display, "****1234");
        let secret = st
            .vars
            .iter()
            .find(|v| v.name == "AWS_SECRET_ACCESS_KEY")
            .unwrap();
        assert_eq!(secret.display, "********");
        let region = st.vars.iter().find(|v| v.name == "AWS_REGION").unwrap();
        assert!(!region.present);
    }
}
