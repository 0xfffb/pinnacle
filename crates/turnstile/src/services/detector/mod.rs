//! Heuristic risk detector layer.
//!
//! Skipped if the context already carries an outcome. Otherwise delegates to
//! the configured [`Detector`] and maps the [`RiskVerdict`] to a stack decision.

mod heuristic;

use std::sync::Arc;

use pinnacle_core::{Action, Context, Next, Request};

use crate::EdgeOutcome;

pub use heuristic::HeuristicDetector;

// ── Public types ──────────────────────────────────────────────────────────────

/// Result produced by a [`Detector`] for a given request context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RiskVerdict {
    /// Risk score in the range `0..=100`.
    pub score: u8,
    /// Recommended action.
    pub action: Action,
    /// Human-readable reasons that contributed to the verdict.
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

/// Evaluates a request context and returns a [`RiskVerdict`].
pub trait Detector: Send + Sync {
    fn evaluate(&self, ctx: &Context) -> RiskVerdict;
}

// ── Layer function ────────────────────────────────────────────────────────────

pub async fn detect(
    detector: Arc<dyn Detector>,
    req: Request,
    next: Next<Request, EdgeOutcome>,
) -> EdgeOutcome {
    if req.ctx.outcome().is_some() {
        return next.run(req).await;
    }
    match detector.evaluate(&req.ctx).action {
        Action::Allow => next.run(req).await,
        Action::Block => EdgeOutcome::text(403, "blocked"),
        Action::Challenge => EdgeOutcome::Challenge,
    }
}
