//! Adapt a plain `async fn(state, req, next)` into a [`LayerService`].

use std::future::Future;
use std::marker::PhantomData;

use async_trait::async_trait;

use super::service::{layer_service, LayerService, LayerServiceLayer};
use super::Next;

/// Adapts `async fn(state, req, next) -> res` into a [`LayerService`].
///
/// Implementation detail of [`from_fn_with_state`]; do not construct directly.
pub struct FnLayerService<St, Req, Res, F> {
    state: St,
    f: F,
    _phantom: PhantomData<fn(Req) -> Res>,
}

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

/// Create a Tower [`Layer`] from a plain async function and shared state.
///
/// `state` is cloned on every request — keep large data behind
/// [`Arc`](std::sync::Arc).
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
///     next.run(req).await
/// }
///
/// ServiceBuilder::new()
///     .layer(from_fn_with_state(app_state, my_layer))
///     .service(terminal);
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
