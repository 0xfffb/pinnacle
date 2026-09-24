mod ban;
mod inspect;
mod stats;

use std::sync::Arc;

use axum::routing::{delete, get, post};
use axum::Router;
use tokio::net::UnixListener;
use tracing::info;

use pinnacle_store::Store;

pub struct ControlServer {
    store: Arc<dyn Store>,
    socket_path: String,
}

impl ControlServer {
    pub fn new(store: Arc<dyn Store>, socket_path: impl Into<String>) -> Self {
        Self {
            store,
            socket_path: socket_path.into(),
        }
    }

    fn router(&self) -> Router {
        Router::new()
            .route("/stats",        get(stats::StatsHandler::stats))
            .route("/bans",         get(ban::BanHandler::list).post(ban::BanHandler::ban))
            .route("/bans/{ip}",    delete(ban::BanHandler::unban))
            .route("/inspect/{ip}", get(inspect::InspectHandler::inspect))
            .with_state(self.store.clone())
    }

    /// Start the control API. Runs until the future is cancelled/dropped.
    pub async fn start(&self) {
        let _ = std::fs::remove_file(&self.socket_path);
        let listener = match UnixListener::bind(&self.socket_path) {
            Ok(l) => l,
            Err(e) => {
                tracing::error!(socket = %self.socket_path, err = %e, "control API bind failed");
                return;
            }
        };
        info!(socket = %self.socket_path, "control API listening");
        if let Err(e) = axum::serve(listener, self.router()).await {
            tracing::error!(err = %e, "control API error");
        }
    }
}
