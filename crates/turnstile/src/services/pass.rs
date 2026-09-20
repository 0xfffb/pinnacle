use std::convert::Infallible;
use std::sync::Arc;
use std::task::{Context, Poll};

use pinnacle_core::{Request, Service, ServiceExt};
use pinnacle_store::Store;

use super::EdgeFut;
use crate::EdgeOutcome;

#[derive(Clone)]
pub struct Pass<S> {
    pub(crate) store: Arc<dyn Store>,
    pub(crate) inner: S,
}

impl<S> Service<Request> for Pass<S>
where
    S: Service<Request, Response = EdgeOutcome, Error = Infallible> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = EdgeOutcome;
    type Error = Infallible;
    type Future = EdgeFut;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        // Pass state is gated by Challenge; do not skip detector/policy.
        let _ = &self.store;
        let inner = self.inner.clone();
        Box::pin(async move { ServiceExt::oneshot(inner, req).await })
    }
}
