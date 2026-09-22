use async_trait::async_trait;
use pinnacle_core::{Disposition, LayerService, Next, Reply, Transaction};

use crate::state::TurnstileState;

#[derive(Clone)]
pub struct BannedService {
    state: TurnstileState,
}

impl BannedService {
    pub fn new(state: TurnstileState) -> Self {
        Self { state }
    }
}

#[async_trait]
impl LayerService for BannedService {
    async fn forward(
        &self,
        transaction: Transaction,
        next: Next<Transaction, Disposition>,
    ) -> Disposition {
        let ip = transaction.meta.get("ip").map(String::as_str).unwrap_or("");
        if self.state.store.is_banned(ip) {
            return Disposition::respond(Reply::text(403, "store_banned"));
        }
        next.forward(transaction).await
    }
}
