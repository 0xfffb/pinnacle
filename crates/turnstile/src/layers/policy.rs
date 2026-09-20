use std::sync::Arc;

use pinnacle_core::Layer;

use crate::services::{Policy, PolicyEngine};

#[derive(Clone)]
pub struct PolicyLayer {
    policy: Arc<dyn PolicyEngine>,
}

impl PolicyLayer {
    pub fn new(policy: Arc<dyn PolicyEngine>) -> Self {
        Self { policy }
    }
}

impl<S> Layer<S> for PolicyLayer {
    type Service = Policy<S>;

    fn layer(&self, inner: S) -> Self::Service {
        Policy {
            policy: self.policy.clone(),
            inner,
        }
    }
}
