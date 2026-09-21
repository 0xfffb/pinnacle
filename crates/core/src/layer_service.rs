//! [`LayerService`]: write `call`, Tower wiring is blanket-provided.
//!
//! Two ergonomic entry points:
//!
//! | Use case | Constructor |
//! |---|---|
//! | Plain `async fn` + shared state | [`from_fn_with_state`] |
//! | Custom struct with helper methods | [`layer_service`] |

use std::convert::Infallible;
use std::future::Future;
use std::marker::PhantomData;
use std::task::{Context, Poll};

use async_trait::async_trait;
use futures::future::BoxFuture;

use crate::tower::{BoxCloneSyncService, Layer, Service, ServiceExt};

// ── Next ────────────────────────────────────────────────────────────────────

/// Continuation to the inner stack.
///
/// Call [`Next::run`] to pass the request to the next layer.
pub struct Next<Req, Res> {
    inner: BoxCloneSyncService<Req, Res, Infallible>,
}

impl<Req, Res> Next<Req, Res>
where
    Req: Send + 'static,
    Res: Send + 'static,
{
    /// Forward `req` to the rest of the middleware stack.
    pub async fn run(self, req: Req) -> Res {
        match ServiceExt::oneshot(self.inner, req).await {
            Ok(res) => res,
            Err(e) => match e {},
        }
    }
}

// ── LayerService (struct-based) ──────────────────────────────────────────────

/// Business logic for one stack layer.
///
/// Implement only [`call`](LayerService::call); all Tower plumbing is provided
/// automatically via [`layer_service`].
///
/// Prefer this when your layer needs helper methods or complex internal state.
/// For simple cases, use [`from_fn_with_state`] instead.
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

// ── LayerSvc / LayerServiceLayer ─────────────────────────────────────────────

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

/// [`Layer`] created by [`layer_service`] or [`from_fn_with_state`].
#[derive(Clone)]
pub struct LayerServiceLayer<L> {
    logic: L,
}

/// Wrap a [`LayerService`] implementation as a Tower [`Layer`].
///
/// ```rust,ignore
/// .layer(layer_service(Bannd::new(store)))
/// ```
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

// ── from_fn_with_state (function-based) ──────────────────────────────────────

/// Internal wrapper used by [`from_fn_with_state`].
///
/// Users should not construct this directly; use [`from_fn_with_state`] instead.
pub struct FnLayerService<St, Req, Res, F> {
    state: St,
    f: F,
    _phantom: PhantomData<fn(Req) -> Res>,
}

// Manual Clone: only St and F need to be Clone; Req/Res live only in PhantomData.
impl<St, Req, Res, F> Clone for FnLayerService<St, Req, Res, F>
where
    St: Clone,
    F: Clone,
{
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
            f: self.f.clone(),
            _phantom: PhantomData,
        }
    }
}

#[async_trait]
impl<St, Req, Res, F, Fut> LayerService for FnLayerService<St, Req, Res, F>
where
    St: Clone + Send + Sync + 'static,
    Req: Send + 'static,
    Res: Send + 'static,
    F: Fn(St, Req, Next<Req, Res>) -> Fut + Clone + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send + 'static,
{
    type Request = Req;
    type Response = Res;

    async fn call(&self, req: Req, next: Next<Req, Res>) -> Res {
        (self.f)(self.state.clone(), req, next).await
    }
}

/// Create a [`Layer`] from a plain async function and shared state.
///
/// `state` is **cloned** into each invocation — keep large data behind an
/// [`Arc`](std::sync::Arc) so cloning stays cheap.
///
/// # Example
///
/// ```rust,ignore
/// #[derive(Clone)]
/// struct AppState { store: Arc<dyn Store> }
///
/// async fn my_layer(
///     state: AppState,
///     req: Request,
///     next: Next<Request, Response>,
/// ) -> Response {
///     // business logic …
///     next.run(req).await
/// }
///
/// ServiceBuilder::new()
///     .layer(from_fn_with_state(app_state, my_layer))
///     .service(terminal)
/// ```
pub fn from_fn_with_state<St, Req, Res, F, Fut>(
    state: St,
    f: F,
) -> LayerServiceLayer<FnLayerService<St, Req, Res, F>>
where
    St: Clone + Send + Sync + 'static,
    Req: Send + 'static,
    Res: Send + 'static,
    F: Fn(St, Req, Next<Req, Res>) -> Fut + Clone + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send + 'static,
{
    layer_service(FnLayerService {
        state,
        f,
        _phantom: PhantomData,
    })
}
