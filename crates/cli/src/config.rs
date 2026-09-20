use std::fs;
use std::path::Path;

use pinnacle_core::Action;
use pinnacle_turnstile::{PolicySet, Rule};
use serde::Deserialize;

/// Configuration load failure.
#[derive(Debug)]
pub struct ConfigError {
    pub message: String,
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ConfigError {}

/// Top-level gateway configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default = "default_listen")]
    pub listen: String,
    #[serde(default = "default_upstream")]
    pub upstream: String,
    #[serde(default)]
    pub rules: Vec<RuleConfig>,
}

fn default_listen() -> String {
    "0.0.0.0:6188".into()
}

fn default_upstream() -> String {
    "127.0.0.1:8080".into()
}

/// Serializable action name used in config files.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActionName {
    Allow,
    Challenge,
    Block,
}

impl From<ActionName> for Action {
    fn from(value: ActionName) -> Self {
        match value {
            ActionName::Allow => Action::Allow,
            ActionName::Challenge => Action::Challenge,
            ActionName::Block => Action::Block,
        }
    }
}

/// One policy rule as written in TOML.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RuleConfig {
    IpAllow { ip: String },
    IpBlock { ip: String, reason: String },
    PathPrefix {
        prefix: String,
        action: ActionName,
    },
    PathRate {
        prefix: String,
        max_count: u32,
        action: ActionName,
    },
    UaAllowContains { needle: String },
}

impl From<&RuleConfig> for Rule {
    fn from(value: &RuleConfig) -> Self {
        match value {
            RuleConfig::IpAllow { ip } => Rule::IpAllow { ip: ip.clone() },
            RuleConfig::IpBlock { ip, reason } => Rule::IpBlock {
                ip: ip.clone(),
                reason: reason.clone(),
            },
            RuleConfig::PathPrefix { prefix, action } => Rule::PathPrefix {
                prefix: prefix.clone(),
                action: Action::from(*action),
            },
            RuleConfig::PathRate {
                prefix,
                max_count,
                action,
            } => Rule::PathRate {
                prefix: prefix.clone(),
                max_count: *max_count,
                action: Action::from(*action),
            },
            RuleConfig::UaAllowContains { needle } => Rule::UaAllowContains {
                needle: needle.clone(),
            },
        }
    }
}

impl Config {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        let raw = fs::read_to_string(path).map_err(|e| ConfigError {
            message: format!("read {}: {e}", path.display()),
        })?;
        Self::parse(&raw).map_err(|e| ConfigError {
            message: format!("parse {}: {e}", path.display()),
        })
    }

    pub fn parse(raw: &str) -> Result<Self, String> {
        toml::from_str(raw).map_err(|e| e.to_string())
    }

    pub fn policy(&self) -> PolicySet {
        PolicySet::new(self.rules.iter().map(Rule::from).collect())
    }

    pub fn upstream_peer(&self) -> Result<(String, u16), String> {
        Self::parse_host_port(&self.upstream)
    }

    pub fn validate_listen(&self) -> Result<(), String> {
        Self::parse_host_port(&self.listen).map(|_| ())
    }

    pub fn parse_host_port(addr: &str) -> Result<(String, u16), String> {
        let (host, port) = addr
            .rsplit_once(':')
            .ok_or_else(|| format!("expected host:port, got {addr}"))?;
        let port: u16 = port
            .parse()
            .map_err(|_| format!("invalid port in {addr}"))?;
        if host.is_empty() {
            return Err("empty host".into());
        }
        Ok((host.to_string(), port))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pinnacle_turnstile::{PolicyEffect, PolicyEngine};

    #[test]
    fn parses_and_builds_policy() {
        let cfg = Config::parse(
            r#"
listen = "0.0.0.0:7000"
upstream = "10.0.0.1:9000"

[[rules]]
type = "ip_allow"
ip = "127.0.0.1"

[[rules]]
type = "path_rate"
prefix = "/api/"
max_count = 10
action = "challenge"
"#,
        )
        .unwrap();

        assert_eq!(cfg.listen, "0.0.0.0:7000");
        assert_eq!(cfg.upstream_peer().unwrap(), ("10.0.0.1".into(), 9000));
        assert_eq!(
            cfg.rules[1],
            RuleConfig::PathRate {
                prefix: "/api/".into(),
                max_count: 10,
                action: ActionName::Challenge,
            }
        );

        let policy = cfg.policy();
        let ctx =
            pinnacle_core::Context::new("/api/x", "203.0.113.1", "ua").with_count(11);
        assert_eq!(policy.evaluate(&ctx).effect, PolicyEffect::Challenge);
    }
}
