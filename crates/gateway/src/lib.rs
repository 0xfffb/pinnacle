mod downstream;

use async_trait::async_trait;
use pingora::prelude::*;
use pingora::proxy::{ProxyHttp, Session};
use pinnacle_turnstile::Turnstile;

pub use downstream::Downstream;

pub struct Gateway {
    upstream: (String, u16),
    turnstile: Turnstile,
}

impl Gateway {
    pub fn new(upstream: (String, u16), turnstile: Turnstile) -> Self {
        Self {
            upstream,
            turnstile,
        }
    }
}

#[async_trait]
impl ProxyHttp for Gateway {
    type CTX = ();

    fn new_ctx(&self) {}

    async fn request_filter(&self, session: &mut Session, _ctx: &mut ()) -> Result<bool> {
        let mut downstream = Downstream::new(session);
        let req = downstream.request().await;
        let decision = self.turnstile.decide(req).await;
        downstream.apply(decision).await
    }

    async fn upstream_peer(&self, _: &mut Session, _: &mut ()) -> Result<Box<HttpPeer>> {
        Ok(Box::new(HttpPeer::new(
            (self.upstream.0.as_str(), self.upstream.1),
            false,
            String::new(),
        )))
    }
}
