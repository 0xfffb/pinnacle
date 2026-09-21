//! Turnstile: tower-based anti-bot middleware stack.
//!
//! # Quick start
//!
//! ```rust,ignore
//! let ts = Turnstile::new(PolicySet::new(vec![]));
//! let outcome = ts.call(req).await;
//! ```
//!
//! # Adding a custom layer
//!
//! ```rust,ignore
//! use pinnacle_turnstile::{from_fn_with_state, Next, Request, TurnstileState, EdgeOutcome};
//!
//! async fn my_layer(
//!     state: TurnstileState,
//!     req: Request,
//!     next: Next<Request, EdgeOutcome>,
//! ) -> EdgeOutcome {
//!     // … your logic …
//!     next.run(req).await
//! }
//! ```

mod outcome;
mod services;
mod state;

use std::convert::Infallible;
use std::sync::Arc;

use tracing::info;

pub use outcome::EdgeOutcome;
pub use pinnacle_core::{
    from_fn_with_state, layer_service, Action, BoxCloneSyncService, Context, Decision, Layer,
    LayerService, Next, Request, Service, ServiceBuilder, ServiceExt, SessionIo,
};
pub use services::{
    banned, challenge, count, detect, policy, CookieChallenger, Detector, Forward,
    HeuristicDetector, PolicyDecision, PolicyEffect, PolicyEngine, PolicySet, RiskVerdict, Rule,
    COOKIE_CID, COOKIE_PASS, SCRIPT_PATH,
};
pub use state::TurnstileState;

/// Default stack order (outer → inner).
pub const LAYERS: &[&str] = &["challenge", "banned", "count", "policy", "detector", "forward"];

/// Pre-assembled turnstile middleware stack.
///
/// Default stack (outer → inner):
/// `challenge` → `banned` → `count` → `policy` → `detector` → `forward`
pub struct Turnstile {
    services: BoxCloneSyncService<Request, EdgeOutcome, Infallible>,
}

impl Turnstile {
    /// Build the default stack from a [`PolicySet`].
    pub fn new(policy_set: PolicySet) -> Self {
        // Store-backed state shared by challenge / banned / count.
        let state = TurnstileState {
            store: Arc::new(pinnacle_store::MemoryStore::new()),
        };

        // Each layer receives only the dependency it actually needs.
        let policy_engine: Arc<dyn PolicyEngine> = Arc::new(policy_set);
        let detector: Arc<dyn Detector> = Arc::new(HeuristicDetector);

        let mut stack = String::from("turnstile stack (outer → inner)");
        for (i, layer) in LAYERS.iter().enumerate() {
            stack.push_str(&format!("\n  [{}] {layer}", i + 1));
        }
        info!("{stack}");

        // First `.layer` call is outermost (ServiceBuilder / Stack ordering).
        let services = ServiceBuilder::new()
            .layer(from_fn_with_state(state.clone(), challenge))
            .layer(from_fn_with_state(state.clone(), banned))
            .layer(from_fn_with_state(state.clone(), count))
            .layer(from_fn_with_state(policy_engine, policy))
            .layer(from_fn_with_state(detector, detect))
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
        info!(%path, %ip, %outcome, "decision");
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
        let req =
            Request::new(Context::new(SCRIPT_PATH, "1.1.1.1", "Mozilla/5.0").with_method("GET"));
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
                    cookies
                        .iter()
                        .any(|c| c.starts_with(&format!("{COOKIE_CID}="))),
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

        let no_cookie = Request::new(Context::new("/", "9.9.9.9", "Mozilla/5.0"));
        assert!(matches!(
            futures::executor::block_on(ts.call(no_cookie)),
            EdgeOutcome::Respond { status: 503, .. }
        ));

        let with_pass = Request::new(
            Context::new("/", "9.9.9.9", "Mozilla/5.0").with_header("cookie", pass_cookie),
        );
        assert_eq!(
            futures::executor::block_on(ts.call(with_pass)),
            EdgeOutcome::Forward
        );
    }
}
