use std::sync::Arc;

use async_trait::async_trait;
use pinnacle_core::{LayerService, Next, Request};
use pinnacle_store::Store;

use crate::EdgeOutcome;

#[derive(Clone)]
pub struct Count {
    store: Arc<dyn Store>,
}

impl Count {
    pub fn new(store: Arc<dyn Store>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl LayerService for Count {
    type Request = Request;
    type Response = EdgeOutcome;

    async fn call(&self, mut req: Request, next: Next<Request, EdgeOutcome>) -> EdgeOutcome {
        let ip = req.ctx.get_or(pinnacle_core::IP, "").to_owned();
        req.ctx.set(
            pinnacle_core::REQUEST_COUNT,
            self.store.incr_request(&ip).to_string(),
        );
        next.run(req).await
    }
}
