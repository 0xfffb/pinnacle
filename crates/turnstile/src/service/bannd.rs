use pinnacle_core::{Bytes, ClientIp, Decision, Next, Request, Respond, StatusCode};

use crate::state::TurnstileState;

#[derive(Clone)]
pub struct BannedService {
    state: TurnstileState,
}

impl BannedService {
    pub fn new(state: TurnstileState) -> Self {
        Self { state }
    }

    pub async fn call(self, req: Request<Bytes>, next: Next) -> Decision {
        let ip = req
            .extensions()
            .get::<ClientIp>()
            .map(ClientIp::as_str)
            .unwrap_or("");
        if self.state.store.is_banned(ip) {
            return Respond::text(StatusCode::FORBIDDEN, "store_banned").into();
        }
        next.run(req).await
    }
}
