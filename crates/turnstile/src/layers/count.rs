use std::sync::Arc;

use pinnacle_core::Layer;
use pinnacle_store::Store;

use crate::services::Count;

#[derive(Clone)]
pub struct CountLayer {
    store: Arc<dyn Store>,
}

impl CountLayer {
    pub fn new(store: Arc<dyn Store>) -> Self {
        Self { store }
    }
}

impl<S> Layer<S> for CountLayer {
    type Service = Count<S>;

    fn layer(&self, inner: S) -> Self::Service {
        Count {
            store: self.store.clone(),
            inner,
        }
    }
}
