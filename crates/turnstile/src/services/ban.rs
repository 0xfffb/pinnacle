use std::convert::Infallible;
use std::sync::Arc;
use std::task::{Context, Poll};

use pinnacle_core::{Request, Service, ServiceExt};
use pinnacle_store::Store;

use super::{ok, EdgeFut};
use crate::EdgeOutcome;

#[derive(Clone)]
pub struct Ban<S> {
    pub(crate) store: Arc<dyn Store>,
    pub(crate) inner: S,
}

impl<S> Service<Request> for Ban<S>
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
        if self.store.is_banned(req.ctx.get_or(pinnacle_core::IP, "")) {
            return ok(EdgeOutcome::text(403, "store_banned"));
        }
        let inner = self.inner.clone();
        Box::pin(async move { ServiceExt::oneshot(inner, req).await })
    }
}
