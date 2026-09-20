use std::sync::Arc;

use pinnacle_core::Layer;
use pinnacle_store::Store;

use crate::services::Pass;

#[derive(Clone)]
pub struct PassLayer {
    store: Arc<dyn Store>,
}

impl PassLayer {
    pub fn new(store: Arc<dyn Store>) -> Self {
        Self { store }
    }
}

impl<S> Layer<S> for PassLayer {
    type Service = Pass<S>;

    fn layer(&self, inner: S) -> Self::Service {
        Pass {
            store: self.store.clone(),
            inner,
        }
    }
}
