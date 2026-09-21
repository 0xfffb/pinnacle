//! IP ban check layer.
//!
//! Rejects requests from banned IPs before they reach any downstream layer.

use pinnacle_core::{Next, Request};

use crate::state::TurnstileState;
use crate::EdgeOutcome;

pub async fn banned(
    state: TurnstileState,
    req: Request,
    next: Next<Request, EdgeOutcome>,
) -> EdgeOutcome {
    if state.store.is_banned(req.ctx.get_or(pinnacle_core::IP, "")) {
        return EdgeOutcome::text(403, "store_banned");
    }
    next.run(req).await
}
