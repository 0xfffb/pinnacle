use pinnacle_core::{Bytes, Decision, Next, Request, Respond, StatusCode};

use crate::state::TurnstileState;

const CAPTCHA_PATH: &str = "/__captcha";

pub async fn captcha(state: TurnstileState, req: Request<Bytes>, next: Next) -> Decision {
    let _ = state;
    if req.uri().path() == CAPTCHA_PATH {
        return Respond::html(StatusCode::ACCEPTED, "<h1>Captcha</h1>").into();
    }
    next.run(req).await
}
