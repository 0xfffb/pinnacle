use std::sync::Arc;

use pinnacle_core::Layer;
use pinnacle_store::Store;

use crate::services::Ban;

#[derive(Clone)]
pub struct BanLayer {
    store: Arc<dyn Store>,
}

impl BanLayer {
    pub fn new(store: Arc<dyn Store>) -> Self {
        Self { store }
    }
}

impl<S> Layer<S> for BanLayer {
    type Service = Ban<S>;

    fn layer(&self, inner: S) -> Self::Service {
        Ban {
            store: self.store.clone(),
            inner,
        }
    }
}
