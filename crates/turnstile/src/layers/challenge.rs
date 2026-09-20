use std::sync::Arc;

use pinnacle_core::Layer;
use pinnacle_store::Store;

use crate::services::CookieChallengerService;

/// Wraps the inner stack with [`CookieChallengerService`].
#[derive(Clone)]
pub struct ChallengeLayer {
    store: Arc<dyn Store>,
}

impl ChallengeLayer {
    pub fn new(store: Arc<dyn Store>) -> Self {
        Self { store }
    }
}

impl<S> Layer<S> for ChallengeLayer {
    type Service = CookieChallengerService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        CookieChallengerService::new(self.store.clone(), inner)
    }
}
