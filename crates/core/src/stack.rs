use std::convert::Infallible;
use std::future::Future;
use std::marker::PhantomData;

use tower::util::BoxCloneSyncService;
use tower::{Layer, ServiceExt};

use super::{from_fn_with_state, Next};
use crate::{Bytes, Decision, Request};

type Service = BoxCloneSyncService<Request<Bytes>, Decision, Infallible>;
type Install<St> = Box<dyn FnOnce(St, Service) -> Service + Send>;

pub struct StackBuilder<St> {
    layers: Vec<Install<St>>,
    _phantom: PhantomData<St>,
}

impl<St> StackBuilder<St>
where
    St: Clone + Send + Sync + 'static,
{
    pub fn new() -> Self {
        Self {
            layers: Vec::new(),
            _phantom: PhantomData,
        }
    }

    pub fn layer<F, Fut>(mut self, f: F) -> Self
    where
        F: Fn(St, Request<Bytes>, Next) -> Fut + Clone + Send + Sync + 'static,
        Fut: Future<Output = Decision> + Send + 'static,
    {
        self.layers.push(Box::new(move |state, inner| {
            BoxCloneSyncService::new(from_fn_with_state(state, f).layer(inner))
        }));
        self
    }

    /// Axum-style: mount shared state last, then build.
    pub fn with_state(self, state: St) -> Stack {
        let mut svc: Service = BoxCloneSyncService::new(tower::service_fn(
            |_req: Request<Bytes>| async { Ok::<_, Infallible>(None) },
        ));

        for install in self.layers.into_iter().rev() {
            svc = install(state.clone(), svc);
        }

        Stack { inner: svc }
    }
}

#[derive(Clone)]
pub struct Stack {
    inner: Service,
}

impl Stack {
    pub fn builder<St: Clone + Send + Sync + 'static>() -> StackBuilder<St> {
        StackBuilder::new()
    }

    pub async fn decide(&self, req: Request<Bytes>) -> Decision {
        match self.inner.clone().oneshot(req).await {
            Ok(d) => d,
            Err(e) => match e {},
        }
    }
}
