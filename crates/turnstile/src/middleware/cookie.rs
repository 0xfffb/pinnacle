use pinnacle_core::{Bytes, Decision, Next, Request};

use crate::service::CookieChallengeService;
use crate::state::TurnstileState;

pub async fn cookie(state: TurnstileState, req: Request<Bytes>, next: Next) -> Decision {
    CookieChallengeService::new(state).call(req, next).await
}
