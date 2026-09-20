//! Cookie-based JS fingerprint challenge service.
//!
//! - Challenge HTML / `fp.js`: `Set-Cookie: __pinnacle_cid=…`
//! - Verify success: `Set-Cookie: __pinnacle_pass=…`
//! - Every normal request: validate `__pinnacle_pass` against the store

use std::sync::Arc;

use async_trait::async_trait;
use pinnacle_core::{Decision, LayerService, Next, Request};
use pinnacle_store::{ChallengeSession, Store};
use serde::Deserialize;
use tracing::info;

use crate::EdgeOutcome;

pub const SCRIPT_PATH: &str = "/__pinnacle/fp.js";
pub const COOKIE_CID: &str = "__pinnacle_cid";
pub const COOKIE_PASS: &str = "__pinnacle_pass";
const EXPECTED_VERSION: &str = "1.0.0";
const KIND: &str = "cookie";
const CHALLENGE_ID: &str = "chg_cookie";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Wire {
    Script,
    Verify,
}

struct Challenge {
    kind: String,
    id: String,
    payload: String,
}

struct VerifyResult {
    ok: bool,
}

impl VerifyResult {
    fn passed() -> Self {
        Self { ok: true }
    }

    fn failed() -> Self {
        Self { ok: false }
    }
}

#[derive(Debug, Deserialize)]
struct Report {
    version: String,
    #[serde(default)]
    automation: Automation,
}

#[derive(Debug, Default, Deserialize)]
struct Automation {
    #[serde(default)]
    webdriver: bool,
    #[serde(default)]
    headless: bool,
    #[serde(default)]
    cdp: bool,
    #[serde(default)]
    selenium: bool,
    #[serde(default)]
    phantom: bool,
    #[serde(default)]
    puppeteer: bool,
    #[serde(default)]
    playwright: bool,
}

impl Automation {
    fn is_bot(&self) -> bool {
        self.webdriver
            || self.headless
            || self.cdp
            || self.selenium
            || self.phantom
            || self.puppeteer
            || self.playwright
    }
}

fn cookie_value<'a>(header: &'a str, name: &str) -> Option<&'a str> {
    header.split(';').find_map(|part| {
        let part = part.trim();
        part.split_once('=')
            .filter(|(k, v)| k.trim() == name && !v.trim().is_empty())
            .map(|(_, v)| v.trim())
    })
}

fn set_cookie(name: &str, value: &str) -> String {
    format!("{name}={value}; Path=/; SameSite=Lax")
}

/// Cookie challenger: issue / classify / verify fingerprint reports.
#[derive(Debug, Default, Clone, Copy)]
pub struct CookieChallenger;

impl CookieChallenger {
    fn issue(&self) -> Challenge {
        Challenge {
            kind: KIND.into(),
            id: CHALLENGE_ID.into(),
            payload: EXPECTED_VERSION.into(),
        }
    }

    fn classify(&self, method: &str, path: &str, cookie_header: &str) -> Option<Wire> {
        let has_cid = cookie_value(cookie_header, COOKIE_CID).is_some();
        match method {
            "GET" if path == SCRIPT_PATH => Some(Wire::Script),
            "POST" if has_cid => Some(Wire::Verify),
            _ => None,
        }
    }

    fn verify(&self, challenge: &Challenge, response: &str) -> VerifyResult {
        if challenge.kind != KIND {
            return VerifyResult::failed();
        }
        self.verify_json(response)
    }

    fn verify_json(&self, raw: &str) -> VerifyResult {
        let Ok(report) = serde_json::from_str::<Report>(raw) else {
            return VerifyResult::failed();
        };
        if report.version != EXPECTED_VERSION || report.automation.is_bot() {
            VerifyResult::failed()
        } else {
            VerifyResult::passed()
        }
    }

    fn challenge_page(&self) -> String {
        include_str!("../../../../../assets/challenge.html").replace("__SCRIPT_PATH__", SCRIPT_PATH)
    }

    fn challenge_script(&self) -> &'static str {
        include_str!("../../../../../assets/fingerprint.js")
    }
}

/// Layer service: script / verify / pass-cookie gate / map inner `Challenge`.
#[derive(Clone)]
pub struct CookieChallengerService {
    store: Arc<dyn Store>,
    challenger: CookieChallenger,
}

impl CookieChallengerService {
    pub fn new(store: Arc<dyn Store>) -> Self {
        Self {
            store,
            challenger: CookieChallenger,
        }
    }

    fn issue_page(&self, ip: &str) -> EdgeOutcome {
        let ch = self.challenger.issue();
        let id = ch.id.clone();
        self.store
            .put_challenge(ip, ChallengeSession::new(ch.kind, ch.id, ch.payload));
        EdgeOutcome::html(503, self.challenger.challenge_page())
            .with_cookie(set_cookie(COOKIE_CID, &id))
    }

    fn script_response(&self, ip: &str) -> EdgeOutcome {
        let mut out = EdgeOutcome::js(self.challenger.challenge_script());
        if let Some(session) = self.store.get_challenge(ip) {
            out = out.with_cookie(set_cookie(COOKIE_CID, &session.challenge_id));
        }
        out
    }

    fn check_verify(&self, ip: &str, body: &str) -> Result<String, Decision> {
        let challenge = self
            .store
            .take_challenge(ip)
            .map(|s| Challenge {
                kind: s.kind,
                id: s.challenge_id,
                payload: s.payload,
            })
            .unwrap_or_else(|| self.challenger.issue());

        if self.challenger.verify(&challenge, body).ok {
            Ok(self.store.issue_pass(ip))
        } else {
            self.store.ban(ip, "challenge_failed");
            Err(Decision::block("verify", "challenge_failed"))
        }
    }

    fn has_valid_pass(&self, ip: &str, cookie_header: &str) -> bool {
        cookie_value(cookie_header, COOKIE_PASS)
            .is_some_and(|token| self.store.validate_pass(ip, token))
    }
}

#[async_trait]
impl LayerService for CookieChallengerService {
    type Request = Request;
    type Response = EdgeOutcome;

    async fn call(&self, mut req: Request, next: Next<Request, EdgeOutcome>) -> EdgeOutcome {
        let method = req.ctx.get_or(pinnacle_core::METHOD, "GET").to_owned();
        let path = req.ctx.get_or(pinnacle_core::PATH, "").to_owned();
        let cookie = req.ctx.header("cookie").unwrap_or("").to_owned();
        let ip = req.ctx.get_or(pinnacle_core::IP, "").to_owned();

        if let Some(wire) = self.challenger.classify(&method, &path, &cookie) {
            match wire {
                Wire::Script => return self.script_response(&ip),
                Wire::Verify => {
                    let body = String::from_utf8_lossy(&req.take_body().await).into_owned();
                    return match self.check_verify(&ip, &body) {
                        Ok(token) => {
                            info!(%ip, ok = true, "verify");
                            EdgeOutcome::empty(200).with_cookie(set_cookie(COOKIE_PASS, &token))
                        }
                        Err(decision) => {
                            info!(%ip, ok = false, "verify");
                            let _ = decision;
                            EdgeOutcome::text(403, "challenge_failed")
                        }
                    };
                }
            }
        }

        if !self.has_valid_pass(&ip, &cookie) {
            return self.issue_page(&ip);
        }

        match next.run(req).await {
            EdgeOutcome::Challenge => self.issue_page(&ip),
            other => other,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_browser_passes() {
        let raw = r#"{"version":"1.0.0","automation":{"webdriver":false}}"#;
        assert!(CookieChallenger.verify_json(raw).ok);
    }

    #[test]
    fn webdriver_fails() {
        let raw = r#"{"version":"1.0.0","automation":{"webdriver":true}}"#;
        assert!(!CookieChallenger.verify_json(raw).ok);
    }

    #[test]
    fn classifies_script_by_path_and_verify_by_cookie() {
        let c = CookieChallenger;
        assert_eq!(c.classify("GET", SCRIPT_PATH, ""), Some(Wire::Script));
        assert_eq!(
            c.classify("POST", "/", "__pinnacle_cid=chg_cookie"),
            Some(Wire::Verify)
        );
        assert_eq!(c.classify("GET", "/", ""), None);
    }
}
