mod middleware;
mod service;
mod state;

use std::sync::Arc;

use pinnacle_core::{Bytes, Decision, Request, Stack};
use tracing::info;

pub use state::{CookieEndpoint, TurnstileState};

pub const LAYERS: &[&str] = &["cookie", "captcha", "banned"];

#[derive(Clone)]
pub struct Turnstile {
    stack: Stack,
    endpoints: Arc<CookieEndpoint>,
}

impl Turnstile {
    pub fn new() -> Self {
        let endpoints = Arc::new(CookieEndpoint {
            path: CookieEndpoint::random_path(),
            cookie_cid: "EPIN-CID".to_string(),
            cookie_pass: "EPIN-A-S3CR3T".to_string(),
        });

        info!(
            path = %endpoints.path,
            cid = %endpoints.cookie_cid,
            "cookie challenge endpoint"
        );

        let state = TurnstileState {
            store: Arc::new(pinnacle_store::MemoryStore::new()),
            endpoints: endpoints.clone(),
        };

        let mut log = String::from("turnstile stack (outer → inner)");
        for (i, layer) in LAYERS.iter().enumerate() {
            log.push_str(&format!("\n  [{}] {layer}", i + 1));
        }
        info!("{log}");

        let stack = Stack::builder()
            .layer(middleware::cookie)
            .layer(middleware::captcha)
            .layer(middleware::banned)
            .with_state(state);

        Self { stack, endpoints }
    }

    pub fn endpoints(&self) -> &CookieEndpoint {
        &self.endpoints
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
        let path = ts.endpoints().path.clone();
        let out = futures::executor::block_on(ts.decide(req(
            Method::GET,
            &path,
            "1.1.1.1",
            "",
            Bytes::new(),
        )));
        let res = out.expect("respond");
        assert_eq!(res.status(), 200);
        assert!(!res.body().is_empty());
        assert!(
            res.headers()
                .get_all("set-cookie")
                .iter()
                .any(|c| c.to_str().unwrap_or("").starts_with(&format!(
                    "{}=",
                    ts.endpoints().cookie_cid
                )))
        );
    }

    #[test]
    fn root_issues_cid_cookie() {
        let ts = Turnstile::new();
        let cid = ts.endpoints().cookie_cid.clone();
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
                .any(|c| c.to_str().unwrap_or("").starts_with(&format!("{cid}=")))
        );
    }

    #[test]
    fn verify_issues_pass_cookie_required_every_request() {
        let ts = Turnstile::new();
        let path = ts.endpoints().path.clone();
        let cid = ts.endpoints().cookie_cid.clone();
        let pass_name = ts.endpoints().cookie_pass.clone();

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
            &path,
            "9.9.9.9",
            &format!("{cid}=chg_cookie"),
            body,
        )));
        let res = out.expect("respond");
        assert_eq!(res.status(), 200);
        let pass = set_cookie_pair(&res, &pass_name);

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
