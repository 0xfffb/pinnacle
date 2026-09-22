use std::convert::Infallible;

use tower::util::BoxCloneSyncService;
use tower::{Layer, ServiceExt};

use super::{AsLayer, LayerService};
use crate::{Disposition, Transaction};

type Service = BoxCloneSyncService<Transaction, Disposition, Infallible>;
type Wrap<St> = Box<dyn FnOnce(St, Service) -> Service>;

pub struct StackBuilder<St> {
    state: St,
    wraps: Vec<Wrap<St>>,
    default: Option<Disposition>,
}

impl<St> StackBuilder<St>
where
    St: Clone + 'static,
{
    pub fn new(state: St) -> Self {
        Self {
            state,
            wraps: Vec::new(),
            default: None,
        }
    }

    pub fn with<L, F>(mut self, factory: F) -> Self
    where
        F: FnOnce(St) -> L + 'static,
        L: LayerService + 'static,
    {
        self.wraps.push(Box::new(move |state, inner| {
            BoxCloneSyncService::new(AsLayer::new(factory(state)).layer(inner))
        }));
        self
    }

    pub fn default(mut self, disposition: Disposition) -> Self {
        self.default = Some(disposition);
        self
    }

    pub fn build(self) -> Stack {
        let disposition = self
            .default
            .expect("StackBuilder::default() required before build");

        let mut service: Service = BoxCloneSyncService::new(tower::service_fn(
            move |_transaction: Transaction| {
                let disposition = disposition.clone();
                async move { Ok::<_, Infallible>(disposition) }
            },
        ));

        for wrap in self.wraps.into_iter().rev() {
            service = wrap(self.state.clone(), service);
        }

        Stack { inner: service }
    }
}

#[derive(Clone)]
pub struct Stack {
    inner: Service,
}

impl Stack {
    pub fn builder<St: Clone + 'static>(state: St) -> StackBuilder<St> {
        StackBuilder::new(state)
    }

    pub async fn decide(&self, transaction: Transaction) -> Disposition {
        match self.inner.clone().oneshot(transaction).await {
            Ok(disposition) => disposition,
            Err(e) => match e {},
        }
    }
}
