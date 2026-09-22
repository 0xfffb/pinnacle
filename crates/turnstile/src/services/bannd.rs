use pinnacle_core::{Bytes, ClientIp, Decision, Next, Request, Respond, StatusCode};

use crate::state::TurnstileState;

pub async fn banned(state: TurnstileState, req: Request<Bytes>, next: Next) -> Decision {
    let ip = req
        .extensions()
        .get::<ClientIp>()
        .map(ClientIp::as_str)
        .unwrap_or("");
    if state.store.is_banned(ip) {
        return Respond::text(StatusCode::FORBIDDEN, "store_banned").into();
    }
    next.run(req).await
}
