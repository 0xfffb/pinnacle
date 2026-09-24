use std::sync::Arc;

use axum::extract::{Path, State};
use axum::Json;

use pinnacle_store::Store;

use crate::types::InspectResponse;

pub struct InspectHandler;

impl InspectHandler {
    pub async fn inspect(
        State(store): State<Arc<dyn Store>>,
        Path(ip): Path<String>,
    ) -> Json<InspectResponse> {
        Json(InspectResponse {
            banned: store.is_banned(&ip),
            ban_reason: store.ban_reason(&ip),
            pass_valid: store.has_pass(&ip),
            request_count: store.request_count(&ip),
            ip,
        })
    }
}
