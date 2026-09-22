use pinnacle_core::{Bytes, ClientIp, Decision, Next, Request, Respond, StatusCode};
use pinnacle_store::ChallengeSession;
use tracing::info;

use crate::state::TurnstileState;

pub const PATH: &str = "/__pinnacle";
pub const COOKIE_CID: &str = "__pinnacle_cid";
pub const COOKIE_PASS: &str = "__pinnacle_pass";

const VERSION: &str = "1.0.0";
const KIND: &str = "cookie";
const CID: &str = "chg_cookie";

fn cookie_val<'a>(header: &'a str, name: &str) -> Option<&'a str> {
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

fn client_ip(req: &Request<Bytes>) -> &str {
    req.extensions()
        .get::<ClientIp>()
        .map(ClientIp::as_str)
        .unwrap_or("")
}

pub async fn cookie(state: TurnstileState, req: Request<Bytes>, next: Next) -> Decision {
    let path = req.uri().path().to_owned();
    let ip = client_ip(&req).to_owned();
    let cookies = req
        .headers()
        .get(http::header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_owned();

    if path == PATH {
        match *req.method() {
            http::Method::GET => {
                let mut res =
                    Respond::javascript(include_str!("../../../../../assets/fingerprint.js"));
                if let Some(s) = state.store.get_challenge(&ip) {
                    res = res.with_cookie(set_cookie(COOKIE_CID, &s.challenge_id));
                }
                return res.into();
            }
            http::Method::POST => {
                let body = String::from_utf8_lossy(req.body());
                let kind_ok = state
                    .store
                    .take_challenge(&ip)
                    .map(|s| s.kind == KIND)
                    .unwrap_or(true);
                let ok = kind_ok && report_ok(&body);
                info!(%ip, ok, "verify");
                return if ok {
                    let token = state.store.issue_pass(&ip);
                    Respond::text(StatusCode::OK, "")
                        .with_cookie(set_cookie(COOKIE_PASS, &token))
                        .into()
                } else {
                    state.store.ban(&ip, "challenge_failed");
                    Respond::text(StatusCode::FORBIDDEN, "challenge_failed").into()
                };
            }
            _ => {}
        }
    }

    let pass_ok =
        cookie_val(&cookies, COOKIE_PASS).is_some_and(|t| state.store.validate_pass(&ip, t));
    if pass_ok {
        next.run(req).await
    } else {
        state
            .store
            .put_challenge(&ip, ChallengeSession::new(KIND, CID, VERSION));
        let page = include_str!("../../../../../assets/challenge.html")
            .replace("__SCRIPT_PATH__", PATH);
        Respond::html(StatusCode::SERVICE_UNAVAILABLE, page)
            .with_cookie(set_cookie(COOKIE_CID, CID))
            .into()
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
