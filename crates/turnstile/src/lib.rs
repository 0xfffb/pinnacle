mod services;
mod state;

use std::sync::Arc;

use pinnacle_core::{Bytes, Decision, Request, Stack};
use tracing::info;

pub use services::{COOKIE_CID, COOKIE_PASS, PATH};
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

        let stack = Stack::builder()
            .layer(services::cookie)
            .layer(services::captcha)
            .layer(services::banned)
            .with_state(state);

        Self { stack }
    }

    pub async fn decide(&self, req: Request<Bytes>) -> Decision {
        self.stack.decide(req).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::Method;
    use pinnacle_core::{Bytes, ClientIp};

    fn req(method: Method, path: &str, ip: &str, cookie: &str, body: Bytes) -> Request<Bytes> {
        let mut r = Request::builder()
            .method(method)
            .uri(path)
            .header("cookie", cookie)
            .body(body)
            .unwrap();
        r.extensions_mut().insert(ClientIp(ip.into()));
        r
    }

    fn set_cookie_pair(res: &http::Response<Bytes>, name: &str) -> String {
        res.headers()
            .get_all("set-cookie")
            .iter()
            .filter_map(|v| v.to_str().ok())
            .find(|c| c.starts_with(&format!("{name}=")))
            .map(|c| c.split(';').next().unwrap_or(c).trim().to_owned())
            .expect("set-cookie")
    }

    #[test]
    fn script_served_at_path() {
        let ts = Turnstile::new();
        let out = futures::executor::block_on(ts.decide(req(
            Method::GET,
            PATH,
            "1.1.1.1",
            "",
            Bytes::new(),
        )));
        let res = out.expect("respond");
        assert_eq!(res.status(), 200);
        assert!(!res.body().is_empty());
    }

    #[test]
    fn root_issues_cid_cookie() {
        let ts = Turnstile::new();
        let out = futures::executor::block_on(ts.decide(req(
            Method::GET,
            "/",
            "1.1.1.1",
            "",
            Bytes::new(),
        )));
        let res = out.expect("respond");
        assert_eq!(res.status(), 503);
        assert!(
            res.headers()
                .get_all("set-cookie")
                .iter()
                .any(|c| c.to_str().unwrap_or("").starts_with(&format!("{COOKIE_CID}=")))
        );
    }

    #[test]
    fn verify_issues_pass_cookie_required_every_request() {
        let ts = Turnstile::new();
        let _ = futures::executor::block_on(ts.decide(req(
            Method::GET,
            "/",
            "9.9.9.9",
            "",
            Bytes::new(),
        )));

        let body = Bytes::from_static(br#"{"version":"1.0.0","automation":{"webdriver":false}}"#);
        let out = futures::executor::block_on(ts.decide(req(
            Method::POST,
            PATH,
            "9.9.9.9",
            &format!("{COOKIE_CID}=chg_cookie"),
            body,
        )));
        let res = out.expect("respond");
        assert_eq!(res.status(), 200);
        let pass = set_cookie_pair(&res, COOKIE_PASS);

        let no_cookie = futures::executor::block_on(ts.decide(req(
            Method::GET,
            "/",
            "9.9.9.9",
            "",
            Bytes::new(),
        )));
        assert_eq!(no_cookie.unwrap().status(), 503);

        let ok = futures::executor::block_on(ts.decide(req(
            Method::GET,
            "/",
            "9.9.9.9",
            &pass,
            Bytes::new(),
        )));
        assert!(ok.is_none());
    }
}
