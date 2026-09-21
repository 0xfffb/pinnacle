//! Cookie-based JS fingerprint challenge layer.
//!
//! Request routing:
//! - `GET /__pinnacle/fp.js`                  → serves the fingerprint script
//! - `POST` + `__pinnacle_cid` cookie present → verifies the fingerprint report
//! - Any other request                        → validates `__pinnacle_pass` cookie;
//!                                              issues a challenge page if absent/invalid

use pinnacle_core::{Next, Request};
use pinnacle_store::{ChallengeSession, Store};
use serde::Deserialize;
use tracing::info;

use crate::state::TurnstileState;
use crate::EdgeOutcome;

// ── Public constants ──────────────────────────────────────────────────────────

pub const SCRIPT_PATH: &str = "/__pinnacle/fp.js";
pub const COOKIE_CID: &str = "__pinnacle_cid";
pub const COOKIE_PASS: &str = "__pinnacle_pass";

// ── Private constants ─────────────────────────────────────────────────────────

const EXPECTED_VERSION: &str = "1.0.0";
const KIND: &str = "cookie";
const CHALLENGE_ID: &str = "chg_cookie";

// ── Internal types ────────────────────────────────────────────────────────────

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

// ── Free utilities ────────────────────────────────────────────────────────────

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

// ── CookieChallenger (pure logic, no I/O) ────────────────────────────────────

/// Pure challenge logic: issue challenges, classify requests, verify reports.
///
/// This struct contains no I/O or state; it is safe to instantiate as
/// `CookieChallenger` wherever needed.
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

// ── Private helpers (store I/O) ───────────────────────────────────────────────

fn issue_page(store: &dyn Store, challenger: CookieChallenger, ip: &str) -> EdgeOutcome {
    let ch = challenger.issue();
    let id = ch.id.clone();
    store.put_challenge(ip, ChallengeSession::new(ch.kind, ch.id, ch.payload));
    EdgeOutcome::html(503, challenger.challenge_page())
        .with_cookie(set_cookie(COOKIE_CID, &id))
}

fn script_response(store: &dyn Store, challenger: CookieChallenger, ip: &str) -> EdgeOutcome {
    let mut out = EdgeOutcome::js(challenger.challenge_script());
    if let Some(session) = store.get_challenge(ip) {
        out = out.with_cookie(set_cookie(COOKIE_CID, &session.challenge_id));
    }
    out
}

/// Returns the issued pass token on success, or `None` if verification failed.
fn check_verify(
    store: &dyn Store,
    challenger: CookieChallenger,
    ip: &str,
    body: &str,
) -> Option<String> {
    let challenge = store
        .take_challenge(ip)
        .map(|s| Challenge {
            kind: s.kind,
            id: s.challenge_id,
            payload: s.payload,
        })
        .unwrap_or_else(|| challenger.issue());

    if challenger.verify(&challenge, body).ok {
        Some(store.issue_pass(ip))
    } else {
        store.ban(ip, "challenge_failed");
        None
    }
}

fn has_valid_pass(store: &dyn Store, ip: &str, cookie_header: &str) -> bool {
    cookie_value(cookie_header, COOKIE_PASS)
        .is_some_and(|token| store.validate_pass(ip, token))
}

// ── Layer function ────────────────────────────────────────────────────────────

/// Cookie JS-fingerprint challenge layer.
///
/// Register with [`from_fn_with_state`](pinnacle_core::from_fn_with_state):
///
/// ```rust,ignore
/// .layer(from_fn_with_state(state.clone(), challenge))
/// ```
pub async fn challenge(
    state: TurnstileState,
    mut req: Request,
    next: Next<Request, EdgeOutcome>,
) -> EdgeOutcome {
    let challenger = CookieChallenger;
    let method = req.ctx.get_or(pinnacle_core::METHOD, "GET").to_owned();
    let path = req.ctx.get_or(pinnacle_core::PATH, "").to_owned();
    let cookie = req.ctx.header("cookie").unwrap_or("").to_owned();
    let ip = req.ctx.get_or(pinnacle_core::IP, "").to_owned();

    if let Some(wire) = challenger.classify(&method, &path, &cookie) {
        match wire {
            Wire::Script => return script_response(&*state.store, challenger, &ip),
            Wire::Verify => {
                let body = String::from_utf8_lossy(&req.take_body().await).into_owned();
                return match check_verify(&*state.store, challenger, &ip, &body) {
                        Some(token) => {
                            info!(%ip, ok = true, "verify");
                            EdgeOutcome::empty(200).with_cookie(set_cookie(COOKIE_PASS, &token))
                        }
                        None => {
                            info!(%ip, ok = false, "verify");
                            EdgeOutcome::text(403, "challenge_failed")
                        }
                    };
            }
        }
    }

    if !has_valid_pass(&*state.store, &ip, &cookie) {
        return issue_page(&*state.store, challenger, &ip);
    }

    match next.run(req).await {
        EdgeOutcome::Challenge => issue_page(&*state.store, challenger, &ip),
        other => other,
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

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
