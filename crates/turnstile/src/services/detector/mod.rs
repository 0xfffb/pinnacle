//! Detector feature: layer service + detector providers.

mod heuristic;

use std::sync::Arc;

use async_trait::async_trait;
use pinnacle_core::{Action, Context, LayerService, Next, Request};

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
pub struct Detect {
    detector: Arc<dyn Detector>,
}

impl Detect {
    pub fn new(detector: Arc<dyn Detector>) -> Self {
        Self { detector }
    }
}

#[async_trait]
impl LayerService for Detect {
    type Request = Request;
    type Response = EdgeOutcome;

    async fn call(&self, req: Request, next: Next<Request, EdgeOutcome>) -> EdgeOutcome {
        if req.ctx.outcome().is_some() {
            return next.run(req).await;
        }

        match self.detector.evaluate(&req.ctx).action {
            Action::Allow => next.run(req).await,
            Action::Block => EdgeOutcome::text(403, "blocked"),
            Action::Challenge => EdgeOutcome::Challenge,
        }
    }
}
