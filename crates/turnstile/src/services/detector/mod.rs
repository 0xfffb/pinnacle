//! Detector feature: service + detector providers.

mod heuristic;

use std::convert::Infallible;
use std::sync::Arc;
use std::task::{Context as TaskContext, Poll};

use pinnacle_core::{Action, Context, Request, Service, ServiceExt};

use super::{ok, EdgeFut};
use crate::EdgeOutcome;

pub use heuristic::HeuristicDetector;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RiskVerdict {
    pub score: u8,
    pub action: Action,
    pub reasons: Vec<&'static str>,
}

impl RiskVerdict {
    pub fn new(score: u8, action: Action, reasons: Vec<&'static str>) -> Self {
        Self {
            score,
            action,
            reasons,
        }
    }
}

pub trait Detector: Send + Sync {
    fn evaluate(&self, ctx: &Context) -> RiskVerdict;
}

#[derive(Clone)]
pub struct Detect<S> {
    pub(crate) detector: Arc<dyn Detector>,
    pub(crate) inner: S,
}

impl<S> Service<Request> for Detect<S>
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
        // Only evaluate when no upstream policy already decided.
        if req.ctx.outcome().is_some() {
            let inner = self.inner.clone();
            return Box::pin(async move { ServiceExt::oneshot(inner, req).await });
        }

        match self.detector.evaluate(&req.ctx).action {
            Action::Allow => {
                let inner = self.inner.clone();
                Box::pin(async move { ServiceExt::oneshot(inner, req).await })
            }
            Action::Block => ok(EdgeOutcome::text(403, "blocked")),
            // Bubble to outer Challenge layer (onion response path).
            Action::Challenge => ok(EdgeOutcome::Challenge),
        }
    }
}
