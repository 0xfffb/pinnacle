//! Policy rules and in-process rule engine.

use pinnacle_core::{Action, Context};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyEffect {
    Allow,
    Block,
    Challenge,
    Continue,
}

impl PolicyEffect {
    pub fn to_action(self) -> Option<Action> {
        match self {
            Self::Allow => Some(Action::Allow),
            Self::Block => Some(Action::Block),
            Self::Challenge => Some(Action::Challenge),
            Self::Continue => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyDecision {
    pub effect: PolicyEffect,
    pub matched_rule: Option<&'static str>,
    pub detail: String,
}

impl PolicyDecision {
    pub fn allow(rule: &'static str, detail: impl Into<String>) -> Self {
        Self {
            effect: PolicyEffect::Allow,
            matched_rule: Some(rule),
            detail: detail.into(),
        }
    }

    pub fn block(rule: &'static str, detail: impl Into<String>) -> Self {
        Self {
            effect: PolicyEffect::Block,
            matched_rule: Some(rule),
            detail: detail.into(),
        }
    }

    pub fn challenge(rule: &'static str, detail: impl Into<String>) -> Self {
        Self {
            effect: PolicyEffect::Challenge,
            matched_rule: Some(rule),
            detail: detail.into(),
        }
    }

    pub fn cont(detail: impl Into<String>) -> Self {
        Self {
            effect: PolicyEffect::Continue,
            matched_rule: None,
            detail: detail.into(),
        }
    }

    pub fn action(&self) -> Option<Action> {
        self.effect.to_action()
    }
}

pub trait PolicyEngine: Send + Sync {
    fn evaluate(&self, ctx: &Context) -> PolicyDecision;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Rule {
    IpAllow { ip: String },
    IpBlock { ip: String, reason: String },
    PathPrefix { prefix: String, action: Action },
    PathRate {
        prefix: String,
        max_count: u32,
        action: Action,
    },
    UaAllowContains { needle: String },
}

impl Rule {
    pub fn matches(&self, ctx: &Context) -> Option<PolicyDecision> {
        let ip = ctx.get_or(pinnacle_core::IP, "");
        let path = ctx.get_or(pinnacle_core::PATH, "");
        let ua = ctx.get_or(pinnacle_core::USER_AGENT, "");
        let request_count = ctx.get_u32(pinnacle_core::REQUEST_COUNT);

        match self {
            Self::IpAllow { ip: allow } if ip == allow => {
                Some(PolicyDecision::allow("ip_allow", format!("allowlisted_ip={allow}")))
            }
            Self::IpBlock { ip: block, reason } if ip == block => {
                Some(PolicyDecision::block("ip_block", reason.clone()))
            }
            Self::PathPrefix { prefix, action } if path.starts_with(prefix.as_str()) => {
                let detail = format!("path_prefix={prefix}");
                Some(match action {
                    Action::Allow => PolicyDecision::allow("path_prefix", detail),
                    Action::Challenge => PolicyDecision::challenge("path_prefix", detail),
                    Action::Block => PolicyDecision::block("path_prefix", detail),
                })
            }
            Self::UaAllowContains { needle }
                if ua
                    .to_ascii_lowercase()
                    .contains(&needle.to_ascii_lowercase()) =>
            {
                Some(PolicyDecision::allow(
                    "ua_allow",
                    format!("allowlisted_ua_contains={needle}"),
                ))
            }
            Self::PathRate {
                prefix,
                max_count,
                action,
            } if path.starts_with(prefix.as_str()) && request_count > *max_count => {
                let detail =
                    format!("path_prefix={prefix} count={request_count} max={max_count}");
                Some(match action {
                    Action::Allow => PolicyDecision::allow("path_rate", detail),
                    Action::Challenge => PolicyDecision::challenge("path_rate", detail),
                    Action::Block => PolicyDecision::block("path_rate", detail),
                })
            }
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct PolicySet {
    pub rules: Vec<Rule>,
}

impl PolicySet {
    pub fn new(rules: Vec<Rule>) -> Self {
        Self { rules }
    }
}

impl PolicyEngine for PolicySet {
    fn evaluate(&self, ctx: &Context) -> PolicyDecision {
        self.rules
            .iter()
            .find_map(|rule| rule.matches(ctx))
            .unwrap_or_else(|| PolicyDecision::cont("no_policy_match"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_policy() -> PolicySet {
        PolicySet::new(vec![
            Rule::IpAllow {
                ip: "127.0.0.1".into(),
            },
            Rule::IpBlock {
                ip: "198.51.100.66".into(),
                reason: "known_bad_ip".into(),
            },
            Rule::PathRate {
                prefix: "/api/".into(),
                max_count: 120,
                action: Action::Block,
            },
            Rule::PathRate {
                prefix: "/api/".into(),
                max_count: 50,
                action: Action::Challenge,
            },
        ])
    }

    fn sample(ip: &str, path: &str, count: u32) -> Context {
        Context::new(path, ip, "Mozilla/5.0").with_count(count)
    }

    #[test]
    fn allowlists_localhost() {
        assert_eq!(
            sample_policy()
                .evaluate(&sample("127.0.0.1", "/api/x", 999))
                .effect,
            PolicyEffect::Allow
        );
    }

    #[test]
    fn blocks_known_bad_ip() {
        assert_eq!(
            sample_policy()
                .evaluate(&sample("198.51.100.66", "/api/x", 1))
                .effect,
            PolicyEffect::Block
        );
    }

    #[test]
    fn path_rate_challenges_then_blocks() {
        let policy = sample_policy();
        assert_eq!(
            policy.evaluate(&sample("203.0.113.1", "/api/x", 51)).effect,
            PolicyEffect::Challenge
        );
        assert_eq!(
            policy
                .evaluate(&sample("203.0.113.1", "/api/x", 121))
                .effect,
            PolicyEffect::Block
        );
    }

    #[test]
    fn continues_when_no_match() {
        assert_eq!(
            sample_policy()
                .evaluate(&sample("203.0.113.1", "/api/x", 3))
                .effect,
            PolicyEffect::Continue
        );
    }
}
