use std::convert::Infallible;
use std::future::Future;
use std::marker::PhantomData;
use std::task::{Context, Poll};

use async_trait::async_trait;
use futures::future::BoxFuture;
use tower::util::BoxCloneSyncService;
use tower::{Layer, Service};

use super::Next;
use crate::{Disposition, Transaction};

#[async_trait]
pub trait LayerService: Clone + Send + Sync + 'static {
    async fn forward(
        &self,
        transaction: Transaction,
        next: Next<Transaction, Disposition>,
    ) -> Disposition;
}

#[derive(Clone)]
pub struct AsLayer<L> {
    logic: L,
}

impl<L: LayerService> AsLayer<L> {
    pub fn new(logic: L) -> Self {
        Self { logic }
    }
}

impl<L: LayerService, S> Layer<S> for AsLayer<L> {
    type Service = Layered<L, S>;

    fn layer(&self, inner: S) -> Self::Service {
        Layered {
            logic: self.logic.clone(),
            inner,
        }
    }
}

#[derive(Clone)]
pub struct Layered<L, S> {
    logic: L,
    inner: S,
}

impl<L, S> Service<Transaction> for Layered<L, S>
where
    L: LayerService,
    S: Service<Transaction, Response = Disposition, Error = Infallible>
        + Clone
        + Send
        + Sync
        + 'static,
    S::Future: Send + 'static,
{
    type Response = Disposition;
    type Error = Infallible;
    type Future = BoxFuture<'static, Result<Disposition, Infallible>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, transaction: Transaction) -> Self::Future {
        let logic = self.logic.clone();
        let inner = self.inner.clone();
        Box::pin(async move {
            Ok(logic
                .forward(
                    transaction,
                    Next {
                        inner: BoxCloneSyncService::new(inner),
                    },
                )
                .await)
        })
    }
}

pub struct FromFn<St, F> {
    state: St,
    f: F,
    _phantom: PhantomData<fn()>,
}

impl<St, F> Clone for FromFn<St, F>
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
impl<St, F, Fut> LayerService for FromFn<St, F>
where
    St: Clone + Send + Sync + 'static,
    F: Fn(St, Transaction, Next<Transaction, Disposition>) -> Fut
        + Clone
        + Send
        + Sync
        + 'static,
    Fut: Future<Output = Disposition> + Send + 'static,
{
    async fn forward(
        &self,
        transaction: Transaction,
        next: Next<Transaction, Disposition>,
    ) -> Disposition {
        (self.f)(self.state.clone(), transaction, next).await
    }
}

pub fn from_fn<St, F, Fut>(state: St, f: F) -> AsLayer<FromFn<St, F>>
where
    St: Clone + Send + Sync + 'static,
    F: Fn(St, Transaction, Next<Transaction, Disposition>) -> Fut
        + Clone
        + Send
        + Sync
        + 'static,
    Fut: Future<Output = Disposition> + Send + 'static,
{
    AsLayer::new(FromFn {
        state,
        f,
        _phantom: PhantomData,
    })
}
