use pinnacle_core::{Bytes, Decision, Next, Request};

use crate::service::BannedService;
use crate::state::TurnstileState;

pub async fn banned(state: TurnstileState, req: Request<Bytes>, next: Next) -> Decision {
    BannedService::new(state).call(req, next).await
}
