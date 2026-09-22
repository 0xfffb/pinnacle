use pinnacle_core::{Bytes, Decision, Next, Request, Respond, StatusCode};

use crate::state::TurnstileState;

const CAPTCHA_PATH: &str = "/__captcha";

#[derive(Clone)]
pub struct CaptchaChallengeService {
    state: TurnstileState,
}

impl CaptchaChallengeService {
    pub fn new(state: TurnstileState) -> Self {
        Self { state }
    }

    pub async fn call(self, req: Request<Bytes>, next: Next) -> Decision {
        let _ = self.state;
        if req.uri().path() == CAPTCHA_PATH {
            return Respond::html(StatusCode::ACCEPTED, "<h1>Captcha</h1>").into();
        }
        next.run(req).await
    }
}
