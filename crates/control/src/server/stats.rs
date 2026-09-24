use std::sync::Arc;

use axum::extract::State;
use axum::Json;

use pinnacle_store::Store;

use crate::types::StatsResponse;

pub struct StatsHandler;

impl StatsHandler {
    pub async fn stats(State(store): State<Arc<dyn Store>>) -> Json<StatsResponse> {
        Json(StatsResponse {
            requests_total: store.request_count("__total__"),
            banned_total: store.ban_count(),
        })
    }
}
