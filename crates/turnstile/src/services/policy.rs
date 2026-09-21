//! Policy rule engine layer.
//!
//! Evaluates the configured [`PolicyEngine`] against the request context.
//! Matched rules short-circuit the stack; unmatched requests continue downstream.

mod rules;

use std::sync::Arc;

use pinnacle_core::{Action, Next, Request};

use crate::EdgeOutcome;

pub use rules::{PolicyDecision, PolicyEffect, PolicyEngine, PolicySet, Rule};

pub async fn policy(
    engine: Arc<dyn PolicyEngine>,
    req: Request,
    next: Next<Request, EdgeOutcome>,
) -> EdgeOutcome {
    let decision = engine.evaluate(&req.ctx);
    match decision.action() {
        Some(Action::Allow) => EdgeOutcome::Forward,
        Some(Action::Block) => EdgeOutcome::text(403, decision.detail),
        Some(Action::Challenge) => EdgeOutcome::Challenge,
        None => next.run(req).await,
    }
}
