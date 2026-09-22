use std::convert::Infallible;
use std::future::Future;
use std::task::{Context, Poll};

use futures::future::BoxFuture;
use tower::util::BoxCloneSyncService;
use tower::{Layer, Service, ServiceExt};

use crate::{Bytes, Decision, Request};

/// Axum-style next handle.
pub struct Next {
    inner: BoxCloneSyncService<Request<Bytes>, Decision, Infallible>,
}

impl Next {
    pub async fn run(self, req: Request<Bytes>) -> Decision {
        match self.inner.oneshot(req).await {
            Ok(d) => d,
            Err(e) => match e {},
        }
    }
}

#[derive(Clone)]
pub struct FromFnLayer<S, F> {
    state: S,
    f: F,
}

/// Like `axum::middleware::from_fn_with_state`.
pub fn from_fn_with_state<S, F, Fut>(state: S, f: F) -> FromFnLayer<S, F>
where
    S: Clone + Send + Sync + 'static,
    F: Fn(S, Request<Bytes>, Next) -> Fut + Clone + Send + Sync + 'static,
    Fut: Future<Output = Decision> + Send + 'static,
{
    FromFnLayer { state, f }
}

impl<S, F, Serv> Layer<Serv> for FromFnLayer<S, F>
where
    S: Clone,
    F: Clone,
{
    type Service = FromFn<S, F, Serv>;

    fn layer(&self, inner: Serv) -> Self::Service {
        FromFn {
            state: self.state.clone(),
            f: self.f.clone(),
            inner,
        }
    }
}

#[derive(Clone)]
pub struct FromFn<S, F, Serv> {
    state: S,
    f: F,
    inner: Serv,
}

impl<S, F, Serv, Fut> Service<Request<Bytes>> for FromFn<S, F, Serv>
where
    S: Clone + Send + Sync + 'static,
    F: Fn(S, Request<Bytes>, Next) -> Fut + Clone + Send + 'static,
    Fut: Future<Output = Decision> + Send + 'static,
    Serv: Service<Request<Bytes>, Response = Decision, Error = Infallible>
        + Clone
        + Send
        + Sync
        + 'static,
    Serv::Future: Send + 'static,
{
    type Response = Decision;
    type Error = Infallible;
    type Future = BoxFuture<'static, Result<Decision, Infallible>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Bytes>) -> Self::Future {
        let state = self.state.clone();
        let f = self.f.clone();
        let inner = self.inner.clone();
        Box::pin(async move {
            Ok(f(
                state,
                req,
                Next {
                    inner: BoxCloneSyncService::new(inner),
                },
            )
            .await)
        })
    }
}
