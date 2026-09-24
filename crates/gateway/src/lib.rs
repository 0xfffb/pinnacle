mod downstream;

use async_trait::async_trait;
use pingora::prelude::*;
use pingora::proxy::{ProxyHttp, Session};
use pinnacle_core::{Bytes, Decision, Request};

pub use downstream::Downstream;

/// Anything that can decide what to do with a request.
#[async_trait]
pub trait Decider: Send + Sync {
    async fn decide(&self, req: Request<Bytes>) -> Decision;
}

pub struct Gateway<D> {
    upstream: (String, u16),
    decider: D,
}

impl<D: Decider> Gateway<D> {
    pub fn new(upstream: (String, u16), decider: D) -> Self {
        Self { upstream, decider }
    }
}

#[async_trait]
impl<D: Decider + 'static> ProxyHttp for Gateway<D> {
    type CTX = ();

    fn new_ctx(&self) {}

    async fn request_filter(&self, session: &mut Session, _ctx: &mut ()) -> Result<bool> {
        let mut downstream = Downstream::new(session);
        let req = downstream.request().await;
        let decision = self.decider.decide(req).await;
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
