//! [`LayerService`]: write `call`, Tower wiring is blanket-provided.

use std::convert::Infallible;
use std::task::{Context, Poll};

use async_trait::async_trait;
use futures::future::BoxFuture;

use crate::tower::{BoxCloneSyncService, Layer, Service, ServiceExt};

/// Continuation to the inner stack.
pub struct Next<Req, Res> {
    inner: BoxCloneSyncService<Req, Res, Infallible>,
}

impl<Req, Res> Next<Req, Res>
where
    Req: Send + 'static,
    Res: Send + 'static,
{
    pub async fn run(self, req: Req) -> Res {
        match ServiceExt::oneshot(self.inner, req).await {
            Ok(res) => res,
            Err(e) => match e {},
        }
    }
}

/// Business logic for one stack layer. Implement [`call`](LayerService::call) only.
#[async_trait]
pub trait LayerService: Clone + Send + Sync + 'static {
    type Request: Send + 'static;
    type Response: Send + 'static;

    async fn call(
        &self,
        req: Self::Request,
        next: Next<Self::Request, Self::Response>,
    ) -> Self::Response;
}

/// Tower [`Service`] wrapper around a [`LayerService`].
#[derive(Clone)]
pub struct LayerSvc<L, S> {
    logic: L,
    inner: S,
}

impl<L, S> LayerSvc<L, S> {
    pub fn new(logic: L, inner: S) -> Self {
        Self { logic, inner }
    }
}

/// [`Layer`] factory: `.layer(layer_service(Ban::new(store)))`
#[derive(Clone)]
pub struct LayerServiceLayer<L> {
    logic: L,
}

pub fn layer_service<L: LayerService>(logic: L) -> LayerServiceLayer<L> {
    LayerServiceLayer { logic }
}

impl<L: LayerService, S> Layer<S> for LayerServiceLayer<L> {
    type Service = LayerSvc<L, S>;

    fn layer(&self, inner: S) -> Self::Service {
        LayerSvc::new(self.logic.clone(), inner)
    }
}

impl<L, S> Service<L::Request> for LayerSvc<L, S>
where
    L: LayerService,
    S: Service<L::Request, Response = L::Response, Error = Infallible>
        + Clone
        + Send
        + Sync
        + 'static,
    S::Future: Send + 'static,
{
    type Response = L::Response;
    type Error = Infallible;
    type Future = BoxFuture<'static, Result<L::Response, Infallible>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: L::Request) -> Self::Future {
        let logic = self.logic.clone();
        let inner = self.inner.clone();
        Box::pin(async move {
            Ok(logic
                .call(
                    req,
                    Next {
                        inner: BoxCloneSyncService::new(inner),
                    },
                )
                .await)
        })
    }
}
