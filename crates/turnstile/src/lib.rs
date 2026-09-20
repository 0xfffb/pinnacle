//! Turnstile: tower-based anti-bot service stack.

mod layers;
mod outcome;
mod services;

use std::convert::Infallible;
use std::sync::Arc;

use tracing::info;

pub use layers::{BanLayer, ChallengeLayer, CountLayer, DetectorLayer, PassLayer, PolicyLayer};
pub use outcome::EdgeOutcome;
pub use services::{
    CookieChallenger, CookieChallengerService, Detector, Forward, HeuristicDetector,
    PolicyDecision, PolicyEffect, PolicyEngine, PolicySet, RiskVerdict, Rule, COOKIE_CID,
    COOKIE_PASS, SCRIPT_PATH,
};
pub use pinnacle_core::{
    Action, BoxCloneSyncService, Context, Decision, Layer, Request, Service, ServiceBuilder,
    ServiceExt, SessionIo,
};

/// Default stack (outer → inner).
pub const LAYERS: &[&str] = &["challenge", "ban", "count", "policy", "detector", "forward"];

/// Default stack (outer → inner):
/// challenge → ban → count → policy → detector → forward
pub struct Turnstile {
    services: BoxCloneSyncService<Request, EdgeOutcome, Infallible>,
}

impl Turnstile {
    pub fn new(policy: PolicySet) -> Self {
        let store = Arc::new(pinnacle_store::MemoryStore::new());

        let mut stack = String::from("turnstile stack (outer → inner)");
        for (i, layer) in LAYERS.iter().enumerate() {
            stack.push_str(&format!("\n  [{}] {layer}", i + 1));
        }
        info!("{stack}");

        // First `.layer` is outermost (tower::ServiceBuilder / Stack order).
        let services = ServiceBuilder::new()
            .layer(ChallengeLayer::new(store.clone()))
            .layer(BanLayer::new(store.clone()))
            .layer(CountLayer::new(store.clone()))
            .layer(PolicyLayer::new(Arc::new(policy)))
            .layer(DetectorLayer::new(Arc::new(HeuristicDetector)))
            .service(Forward);

        Self {
            services: BoxCloneSyncService::new(services),
        }
    }

    pub async fn call(&self, req: Request) -> EdgeOutcome {
        let path = req.ctx.get_or(pinnacle_core::PATH, "").to_owned();
        let ip = req.ctx.get_or(pinnacle_core::IP, "").to_owned();
        let service = self.services.clone();
        let outcome = match ServiceExt::oneshot(service, req).await {
            Ok(o) => o,
            Err(e) => match e {},
        };
        info!(%path, %ip, outcome = %outcome.log_label(), "decision");
        outcome
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pinnacle_core::Context;

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
        let ts = Turnstile::new(PolicySet::new(vec![]));
        let req = Request::new(
            Context::new(SCRIPT_PATH, "1.1.1.1", "Mozilla/5.0").with_method("GET"),
        );
        let out = futures::executor::block_on(ts.call(req));
        match out {
            EdgeOutcome::Respond {
                status: 200,
                content_type,
                body,
                ..
            } => {
                assert!(content_type.contains("javascript"), "{content_type}");
                assert!(!body.is_empty());
            }
            other => panic!("expected js, got {other:?}"),
        }
    }

    #[test]
    fn root_issues_cid_cookie() {
        let ts = Turnstile::new(PolicySet::new(vec![]));
        let req = Request::new(Context::new("/", "1.1.1.1", "Mozilla/5.0"));
        let out = futures::executor::block_on(ts.call(req));
        match out {
            EdgeOutcome::Respond {
                status: 503,
                content_type,
                body,
                cookies,
            } => {
                assert!(content_type.contains("html"));
                assert!(String::from_utf8_lossy(&body).contains(SCRIPT_PATH));
                assert!(
                    cookies.iter().any(|c| c.starts_with(&format!("{COOKIE_CID}="))),
                    "missing cid Set-Cookie: {cookies:?}"
                );
            }
            other => panic!("expected challenge html, got {other:?}"),
        }
    }

    #[test]
    fn verify_issues_pass_cookie_required_every_request() {
        let ts = Turnstile::new(PolicySet::new(vec![]));
        let _ = futures::executor::block_on(ts.call(Request::new(Context::new(
            "/",
            "9.9.9.9",
            "Mozilla/5.0",
        ))));

        let body = br#"{"version":"1.0.0","automation":{"webdriver":false}}"#;
        let verify = Request::new(
            Context::new("/", "9.9.9.9", "Mozilla/5.0")
                .with_method("POST")
                .with_header("cookie", format!("{COOKIE_CID}=chg_cookie")),
        )
        .with_body(body.to_vec());
        let out = futures::executor::block_on(ts.call(verify));
        let pass_cookie = match out {
            EdgeOutcome::Respond {
                status: 200,
                cookies,
                ..
            } => {
                let raw = cookies
                    .iter()
                    .find(|c| c.starts_with(&format!("{COOKIE_PASS}=")))
                    .expect("pass Set-Cookie");
                cookie_pair(raw)
            }
            other => panic!("expected verify 200, got {other:?}"),
        };

        // Without pass cookie → challenged again.
        let no_cookie = Request::new(Context::new("/", "9.9.9.9", "Mozilla/5.0"));
        assert!(matches!(
            futures::executor::block_on(ts.call(no_cookie)),
            EdgeOutcome::Respond { status: 503, .. }
        ));

        // With valid pass cookie → forward.
        let with_pass = Request::new(
            Context::new("/", "9.9.9.9", "Mozilla/5.0").with_header("cookie", pass_cookie),
        );
        assert_eq!(
            futures::executor::block_on(ts.call(with_pass)),
            EdgeOutcome::Forward
        );
    }
}
