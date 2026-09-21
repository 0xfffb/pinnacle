//! [`LayerService`] trait and its Tower adapters.

use std::convert::Infallible;
use std::task::{Context, Poll};

use async_trait::async_trait;
use futures::future::BoxFuture;

use super::Next;
use tower::util::BoxCloneSyncService;
use tower::{Layer, Service};

/// Business logic for one middleware layer.
///
/// Implement [`call`](LayerService::call) only. Wire it into Tower with
/// [`layer_service`], or skip this trait entirely and use
/// [`from_fn_with_state`](super::from_fn_with_state).
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

/// Tower [`Layer`] that wraps a [`LayerService`].
///
/// Produced by [`layer_service`] / [`from_fn_with_state`](super::from_fn_with_state).
/// Pass it to [`ServiceBuilder::layer`](tower::ServiceBuilder::layer).
#[derive(Clone)]
pub struct LayerServiceLayer<L> {
    pub(super) logic: L,
}

/// Wrap a [`LayerService`] as a Tower [`Layer`].
///
/// ```rust,ignore
/// .layer(layer_service(MyLayer::new(deps)))
/// ```
pub fn layer_service<L: LayerService>(logic: L) -> LayerServiceLayer<L> {
    LayerServiceLayer { logic }
}

impl<L: LayerService, S> Layer<S> for LayerServiceLayer<L> {
    type Service = LayerSvc<L, S>;

    fn layer(&self, inner: S) -> Self::Service {
        LayerSvc {
            logic: self.logic.clone(),
            inner,
        }
    }
}

/// Tower [`Service`] produced by [`LayerServiceLayer`].
///
/// Holds the business logic (`L`) and the next service (`S`).
/// Not constructed directly — use [`layer_service`] /
/// [`from_fn_with_state`](super::from_fn_with_state).
#[derive(Clone)]
pub struct LayerSvc<L, S> {
    logic: L,
    inner: S,
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
