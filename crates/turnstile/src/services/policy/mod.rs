//! Policy feature: service + rule engine.

mod rules;

use std::convert::Infallible;
use std::sync::Arc;
use std::task::{Context as TaskContext, Poll};

use pinnacle_core::{Action, Request, Service, ServiceExt};

use super::{ok, EdgeFut};
use crate::EdgeOutcome;

pub use rules::{PolicyDecision, PolicyEffect, PolicyEngine, PolicySet, Rule};

#[derive(Clone)]
pub struct Policy<S> {
    pub(crate) policy: Arc<dyn PolicyEngine>,
    pub(crate) inner: S,
}

impl<S> Service<Request> for Policy<S>
where
    S: Service<Request, Response = EdgeOutcome, Error = Infallible> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = EdgeOutcome;
    type Error = Infallible;
    type Future = EdgeFut;

    fn poll_ready(&mut self, cx: &mut TaskContext<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let decision = self.policy.evaluate(&req.ctx);
        match decision.action() {
            Some(Action::Allow) => ok(EdgeOutcome::Forward),
            Some(Action::Block) => ok(EdgeOutcome::text(403, decision.detail)),
            // Bubble to outer Challenge layer (onion response path).
            Some(Action::Challenge) => ok(EdgeOutcome::Challenge),
            None => {
                let inner = self.inner.clone();
                Box::pin(async move { ServiceExt::oneshot(inner, req).await })
            }
        }
    }
}
