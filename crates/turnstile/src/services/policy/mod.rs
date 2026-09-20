//! Policy feature: layer service + rule engine.

mod rules;

use std::sync::Arc;

use async_trait::async_trait;
use pinnacle_core::{Action, LayerService, Next, Request};

use crate::EdgeOutcome;

pub use rules::{PolicyDecision, PolicyEffect, PolicyEngine, PolicySet, Rule};

#[derive(Clone)]
pub struct Policy {
    policy: Arc<dyn PolicyEngine>,
}

impl Policy {
    pub fn new(policy: Arc<dyn PolicyEngine>) -> Self {
        Self { policy }
    }
}

#[async_trait]
impl LayerService for Policy {
    type Request = Request;
    type Response = EdgeOutcome;

    async fn call(&self, req: Request, next: Next<Request, EdgeOutcome>) -> EdgeOutcome {
        let decision = self.policy.evaluate(&req.ctx);
        match decision.action() {
            Some(Action::Allow) => EdgeOutcome::Forward,
            Some(Action::Block) => EdgeOutcome::text(403, decision.detail),
            Some(Action::Challenge) => EdgeOutcome::Challenge,
            None => next.run(req).await,
        }
    }
}
