use std::sync::Arc;

use async_trait::async_trait;
use pinnacle_core::{LayerService, Next, Request};
use pinnacle_store::Store;

use crate::EdgeOutcome;

#[derive(Clone)]
pub struct Bannd {
    store: Arc<dyn Store>,
}

impl Bannd {
    pub fn new(store: Arc<dyn Store>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl LayerService for Bannd {
    type Request = Request;
    type Response = EdgeOutcome;

    async fn call(&self, req: Request, next: Next<Request, EdgeOutcome>) -> EdgeOutcome {
        if self.store.is_banned(req.ctx.get_or(pinnacle_core::IP, "")) {
            return EdgeOutcome::text(403, "store_banned");
        }
        next.run(req).await
    }
}
