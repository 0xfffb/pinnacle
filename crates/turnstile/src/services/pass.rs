use std::sync::Arc;

use async_trait::async_trait;
use pinnacle_core::{LayerService, Next, Request};
use pinnacle_store::Store;

use crate::EdgeOutcome;

#[derive(Clone)]
pub struct Pass {
    store: Arc<dyn Store>,
}

impl Pass {
    pub fn new(store: Arc<dyn Store>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl LayerService for Pass {
    type Request = Request;
    type Response = EdgeOutcome;

    async fn call(&self, req: Request, next: Next<Request, EdgeOutcome>) -> EdgeOutcome {
        let _ = &self.store;
        next.run(req).await
    }
}
