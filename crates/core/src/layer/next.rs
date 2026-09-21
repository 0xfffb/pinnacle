//! Continuation to the inner middleware stack.

use std::convert::Infallible;

use tower::util::BoxCloneSyncService;
use tower::ServiceExt;

/// Continuation to the inner middleware stack.
///
/// Call [`Next::run`] to forward the request downstream.
pub struct Next<Req, Res> {
    pub(super) inner: BoxCloneSyncService<Req, Res, Infallible>,
}

impl<Req, Res> Next<Req, Res>
where
    Req: Send + 'static,
    Res: Send + 'static,
{
    /// Forward `req` to the remainder of the stack.
    pub async fn run(self, req: Req) -> Res {
        match ServiceExt::oneshot(self.inner, req).await {
            Ok(res) => res,
            Err(e) => match e {},
        }
    }
}
