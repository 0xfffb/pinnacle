//! `GET/POST /__pinnacle` → script / verify; else require pass cookie.

use async_trait::async_trait;
use pinnacle_core::{Disposition, LayerService, Next, Reply, Transaction, TransactionKind};
use pinnacle_store::ChallengeSession;
use tracing::info;

use crate::state::TurnstileState;

pub const PATH: &str = "/__pinnacle";
pub const COOKIE_CID: &str = "__pinnacle_cid";
pub const COOKIE_PASS: &str = "__pinnacle_pass";

const VERSION: &str = "1.0.0";
const KIND: &str = "cookie";
const CID: &str = "chg_cookie";

fn cookie<'a>(header: &'a str, name: &str) -> Option<&'a str> {
    header.split(';').find_map(|p| {
        let p = p.trim();
        p.split_once('=')
            .filter(|(k, v)| *k == name && !v.is_empty())
            .map(|(_, v)| v.trim())
    })
}

fn set_cookie(name: &str, value: &str) -> String {
    format!("{name}={value}; Path=/; SameSite=Lax")
}

fn report_ok(raw: &str) -> bool {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(raw) else {
        return false;
    };
    if v.get("version").and_then(|x| x.as_str()) != Some(VERSION) {
        return false;
    }
    !v.get("automation")
        .and_then(|a| a.as_object())
        .is_some_and(|o| o.values().any(|x| x.as_bool() == Some(true)))
}

#[derive(Clone)]
pub struct CookieChallengeService {
    state: TurnstileState,
}

impl CookieChallengeService {
    pub fn new(state: TurnstileState) -> Self {
        Self { state }
    }

    fn script(&self, ip: &str) -> Disposition {
        let mut reply = Reply::javascript(include_str!("../../../../../assets/fingerprint.js"));
        if let Some(s) = self.state.store.get_challenge(ip) {
            reply = reply.with_cookie(set_cookie(COOKIE_CID, &s.challenge_id));
        }
        Disposition::respond(reply)
    }

    fn challenge(&self, ip: &str) -> Disposition {
        self.state
            .store
            .put_challenge(ip, ChallengeSession::new(KIND, CID, VERSION));
        let html =
            include_str!("../../../../../assets/challenge.html").replace("__SCRIPT_PATH__", PATH);
        Disposition::respond(Reply::html(503, html).with_cookie(set_cookie(COOKIE_CID, CID)))
    }

    fn verify(&self, ip: &str, body: &str) -> Disposition {
        let kind_ok = self
            .state
            .store
            .take_challenge(ip)
            .map(|s| s.kind == KIND)
            .unwrap_or(true);
        let ok = kind_ok && report_ok(body);
        info!(%ip, ok, "verify");
        if ok {
            let token = self.state.store.issue_pass(ip);
            Disposition::respond(Reply::empty(200).with_cookie(set_cookie(COOKIE_PASS, &token)))
        } else {
            self.state.store.ban(ip, "challenge_failed");
            Disposition::respond(Reply::text(403, "challenge_failed"))
        }
    }

}

#[async_trait]
impl LayerService for CookieChallengeService {
    async fn forward(
        &self,
        mut transaction: Transaction,
        next: Next<Transaction, Disposition>,
    ) -> Disposition {
        let path = transaction.meta.get("path").cloned().unwrap_or_default();
        let ip = transaction.meta.get("ip").cloned().unwrap_or_default();
        let cookies = transaction.headers.get("cookie").cloned().unwrap_or_default();

        if path == PATH {
            match transaction.method {
                TransactionKind::Get => return self.script(&ip),
                TransactionKind::Post => {
                    let body = String::from_utf8_lossy(&transaction.body().await).into_owned();
                    return self.verify(&ip, &body);
                }
                _ => {}
            }
        }

        let pass_ok = cookie(&cookies, COOKIE_PASS)
            .is_some_and(|t| self.state.store.validate_pass(&ip, t));
        if pass_ok {
            next.forward(transaction).await
        } else {
            self.challenge(&ip)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_ok_clean() {
        assert!(report_ok(
            r#"{"version":"1.0.0","automation":{"webdriver":false}}"#
        ));
    }

    #[test]
    fn report_rejects_bot() {
        assert!(!report_ok(
            r#"{"version":"1.0.0","automation":{"webdriver":true}}"#
        ));
    }
}
