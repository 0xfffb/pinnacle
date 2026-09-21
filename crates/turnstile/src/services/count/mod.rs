//! Request counter layer.
//!
//! Increments the per-IP request counter and stores the value in the request
//! context under [`pinnacle_core::REQUEST_COUNT`] for downstream layers.

use pinnacle_core::{Next, Request};

use crate::state::TurnstileState;
use crate::EdgeOutcome;

pub async fn count(
    state: TurnstileState,
    mut req: Request,
    next: Next<Request, EdgeOutcome>,
) -> EdgeOutcome {
    let ip = req.ctx.get_or(pinnacle_core::IP, "").to_owned();
    req.ctx.set(
        pinnacle_core::REQUEST_COUNT,
        state.store.incr_request(&ip).to_string(),
    );
    next.run(req).await
}
