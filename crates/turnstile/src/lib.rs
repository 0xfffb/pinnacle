mod services;
mod state;

use std::sync::Arc;

use pinnacle_core::Stack;
use tracing::info;

pub use pinnacle_core::{Disposition, Next, Reply, StackBuilder, Transaction};
pub use services::{
    BannedService, CaptchaChallengeService, CookieChallengeService, COOKIE_CID, COOKIE_PASS, PATH,
};
pub use state::TurnstileState;

pub const LAYERS: &[&str] = &["cookie", "captcha", "banned"];

#[derive(Clone)]
pub struct Turnstile {
    stack: Stack,
}

impl Turnstile {
    pub fn new() -> Self {
        let state = TurnstileState {
            store: Arc::new(pinnacle_store::MemoryStore::new()),
        };

        let mut log = String::from("turnstile stack (outer → inner)");
        for (i, layer) in LAYERS.iter().enumerate() {
            log.push_str(&format!("\n  [{}] {layer}", i + 1));
        }
        info!("{log}");

        let stack = Stack::builder(state)
            .with(CookieChallengeService::new)
            .with(CaptchaChallengeService::new)
            .with(BannedService::new)
            .default(Disposition::allow())
            .build();

        Self { stack }
    }

    pub async fn decide(&self, transaction: Transaction) -> Disposition {
        self.stack.decide(transaction).await
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use pinnacle_core::TransactionKind;

    fn transaction(path: &str, ip: &str, method: TransactionKind) -> Transaction {
        let mut meta = HashMap::new();
        meta.insert("path".into(), path.into());
        meta.insert("ip".into(), ip.into());
        meta.insert("user_agent".into(), "Mozilla/5.0".into());
        Transaction::new(meta, HashMap::new(), method)
    }

    fn cookie_pair(set_cookie: &str) -> String {
        set_cookie
            .split(';')
            .next()
            .unwrap_or(set_cookie)
            .trim()
            .to_owned()
    }

    #[test]
    fn script_served_at_path() {
        let ts = Turnstile::new();
        let out = futures::executor::block_on(ts.decide(transaction(
            PATH,
            "1.1.1.1",
            TransactionKind::Get,
        )));
        let reply = out.reply().expect("respond");
        assert_eq!(reply.status, 200);
        assert!(reply.content_type.contains("javascript"));
        assert!(!reply.body.is_empty());
    }

    #[test]
    fn root_issues_cid_cookie() {
        let ts = Turnstile::new();
        let out = futures::executor::block_on(ts.decide(transaction(
            "/",
            "1.1.1.1",
            TransactionKind::Get,
        )));
        let reply = out.reply().expect("respond");
        assert_eq!(reply.status, 503);
        assert!(reply.content_type.contains("html"));
        assert!(String::from_utf8_lossy(&reply.body).contains(PATH));
        assert!(
            reply
                .cookies
                .iter()
                .any(|c| c.starts_with(&format!("{COOKIE_CID}="))),
            "missing cid Set-Cookie: {:?}",
            reply.cookies
        );
    }

    #[test]
    fn verify_issues_pass_cookie_required_every_request() {
        let ts = Turnstile::new();
        let _ = futures::executor::block_on(ts.decide(transaction(
            "/",
            "9.9.9.9",
            TransactionKind::Get,
        )));

        let body = br#"{"version":"1.0.0","automation":{"webdriver":false}}"#;
        let mut headers = HashMap::new();
        headers.insert("cookie".into(), format!("{COOKIE_CID}=chg_cookie"));
        let mut meta = HashMap::new();
        meta.insert("path".into(), PATH.into());
        meta.insert("ip".into(), "9.9.9.9".into());
        let verify =
            Transaction::new(meta, headers, TransactionKind::Post).with_full(body.to_vec());
        let out = futures::executor::block_on(ts.decide(verify));
        let reply = out.reply().expect("respond");
        assert_eq!(reply.status, 200);
        let pass_cookie = reply
            .cookies
            .iter()
            .find(|c| c.starts_with(&format!("{COOKIE_PASS}=")))
            .map(|c| cookie_pair(c))
            .expect("pass Set-Cookie");

        let no_cookie = futures::executor::block_on(ts.decide(transaction(
            "/",
            "9.9.9.9",
            TransactionKind::Get,
        )));
        assert_eq!(no_cookie.reply().map(|r| r.status), Some(503));

        let mut headers = HashMap::new();
        headers.insert("cookie".into(), pass_cookie);
        let mut meta = HashMap::new();
        meta.insert("path".into(), "/".into());
        meta.insert("ip".into(), "9.9.9.9".into());
        let with_pass = Transaction::new(meta, headers, TransactionKind::Get);
        assert!(futures::executor::block_on(ts.decide(with_pass)).is_allow());
    }
}
