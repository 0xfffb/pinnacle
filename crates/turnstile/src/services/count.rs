use std::convert::Infallible;
use std::sync::Arc;
use std::task::{Context, Poll};

use pinnacle_core::{Request, Service, ServiceExt};
use pinnacle_store::Store;

use super::EdgeFut;
use crate::EdgeOutcome;

#[derive(Clone)]
pub struct Count<S> {
    pub(crate) store: Arc<dyn Store>,
    pub(crate) inner: S,
}

impl<S> Service<Request> for Count<S>
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

    fn call(&mut self, mut req: Request) -> Self::Future {
        let ip = req.ctx.get_or(pinnacle_core::IP, "").to_owned();
        req.ctx.set(
            pinnacle_core::REQUEST_COUNT,
            self.store.incr_request(&ip).to_string(),
        );
        let inner = self.inner.clone();
        Box::pin(async move { ServiceExt::oneshot(inner, req).await })
    }
}
