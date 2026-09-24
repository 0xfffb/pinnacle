use std::sync::Arc;

use axum::extract::{Path, State};
use axum::Json;
use tracing::info;

use pinnacle_store::Store;

use crate::types::{BanEntry, BanRequest, BanResponse, BansResponse};

pub struct BanHandler;

impl BanHandler {
    pub async fn ban(
        State(store): State<Arc<dyn Store>>,
        Json(req): Json<BanRequest>,
    ) -> Json<BanResponse> {
        store.ban(&req.ip, &req.reason);
        info!(ip = %req.ip, reason = %req.reason, "banned");
        Json(BanResponse { ok: true })
    }

    pub async fn unban(
        State(store): State<Arc<dyn Store>>,
        Path(ip): Path<String>,
    ) -> Json<BanResponse> {
        store.unban(&ip);
        info!(ip = %ip, "unbanned");
        Json(BanResponse { ok: true })
    }

    pub async fn list(State(store): State<Arc<dyn Store>>) -> Json<BansResponse> {
        let bans = store
            .list_bans()
            .into_iter()
            .map(|(ip, record)| BanEntry { ip, reason: record.reason })
            .collect();
        Json(BansResponse { bans })
    }
}
