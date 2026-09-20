//! Middleware request context shared across stages.

use std::collections::HashMap;

use crate::Action;

/// Well-known context keys.
pub const PATH: &str = "path";
pub const IP: &str = "ip";
pub const USER_AGENT: &str = "user_agent";
pub const METHOD: &str = "method";
pub const REQUEST_COUNT: &str = "request_count";
pub const OUTCOME: &str = "outcome";

/// Context passed through the middleware chain (string map, read by key).
#[derive(Debug, Clone, Default)]
pub struct Context {
    values: HashMap<String, String>,
}

impl Context {
    pub fn new(path: impl Into<String>, ip: impl Into<String>, user_agent: impl Into<String>) -> Self {
        let mut values = HashMap::new();
        values.insert(PATH.into(), path.into());
        values.insert(IP.into(), ip.into());
        values.insert(USER_AGENT.into(), user_agent.into());
        values.insert(METHOD.into(), "GET".into());
        values.insert(REQUEST_COUNT.into(), "0".into());
        Self { values }
    }

    pub fn with_method(self, method: impl Into<String>) -> Self {
        self.with(METHOD, method.into().to_ascii_uppercase())
    }

    /// Value of a cookie from the `Cookie` header, if present.
    pub fn cookie(&self, name: &str) -> Option<&str> {
        let header = self.header("cookie")?;
        for part in header.split(';') {
            let part = part.trim();
            if let Some((k, v)) = part.split_once('=') {
                if k.trim() == name {
                    return Some(v.trim());
                }
            }
        }
        None
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(String::as_str)
    }

    pub fn get_or<'a>(&'a self, key: &str, default: &'a str) -> &'a str {
        self.get(key).unwrap_or(default)
    }

    pub fn get_u32(&self, key: &str) -> u32 {
        self.get(key).and_then(|s| s.parse().ok()).unwrap_or(0)
    }

    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.values.insert(key.into(), value.into());
    }

    pub fn extend(&mut self, iter: impl IntoIterator<Item = (String, String)>) {
        self.values.extend(iter);
    }

    pub fn with(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.set(key, value);
        self
    }

    pub fn with_count(self, count: u32) -> Self {
        self.with(REQUEST_COUNT, count.to_string())
    }

    pub fn with_header(self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.with(name.into().to_ascii_lowercase(), value)
    }

    pub fn header(&self, name: &str) -> Option<&str> {
        self.get(&name.to_ascii_lowercase())
    }

    pub fn outcome(&self) -> Option<Action> {
        match self.get(OUTCOME)? {
            "allow" => Some(Action::Allow),
            "challenge" => Some(Action::Challenge),
            "block" => Some(Action::Block),
            _ => None,
        }
    }

    pub fn set_outcome(&mut self, action: Action) {
        let v = match action {
            Action::Allow => "allow",
            Action::Challenge => "challenge",
            Action::Block => "block",
        };
        self.set(OUTCOME, v);
    }
}
