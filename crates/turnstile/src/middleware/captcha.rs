use pinnacle_core::{Bytes, Decision, Next, Request};

use crate::service::CaptchaChallengeService;
use crate::state::TurnstileState;

pub async fn captcha(state: TurnstileState, req: Request<Bytes>, next: Next) -> Decision {
    CaptchaChallengeService::new(state).call(req, next).await
}
