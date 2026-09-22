use std::convert::Infallible;
use std::future::Future;
use std::marker::PhantomData;
use std::task::{Context, Poll};

use async_trait::async_trait;
use futures::future::BoxFuture;
use tower::util::BoxCloneSyncService;
use tower::{Layer, Service};

use super::Next;

#[async_trait]
pub trait LayerService: Clone + Send + Sync + 'static {
    type Transaction: Send + 'static;
    type Disposition: Send + 'static;

    async fn call(
        &self,
        transaction: Self::Transaction,
        next: Next<Self::Transaction, Self::Disposition>,
    ) -> Self::Disposition;
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

impl<L, S> Service<L::Transaction> for Layered<L, S>
where
    L: LayerService,
    S: Service<L::Transaction, Response = L::Disposition, Error = Infallible>
        + Clone
        + Send
        + Sync
        + 'static,
    S::Future: Send + 'static,
{
    type Response = L::Disposition;
    type Error = Infallible;
    type Future = BoxFuture<'static, Result<L::Disposition, Infallible>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, transaction: L::Transaction) -> Self::Future {
        let logic = self.logic.clone();
        let inner = self.inner.clone();
        Box::pin(async move {
            Ok(logic
                .call(
                    transaction,
                    Next {
                        inner: BoxCloneSyncService::new(inner),
                    },
                )
                .await)
        })
    }
}

pub struct FromFn<St, Transaction, Disposition, F> {
    state: St,
    f: F,
    _phantom: PhantomData<fn(Transaction) -> Disposition>,
}

impl<St, Transaction, Disposition, F> Clone for FromFn<St, Transaction, Disposition, F>
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
impl<St, Transaction, Disposition, F, Fut> LayerService
    for FromFn<St, Transaction, Disposition, F>
where
    St: Clone + Send + Sync + 'static,
    Transaction: Send + 'static,
    Disposition: Send + 'static,
    F: Fn(St, Transaction, Next<Transaction, Disposition>) -> Fut
        + Clone
        + Send
        + Sync
        + 'static,
    Fut: Future<Output = Disposition> + Send + 'static,
{
    type Transaction = Transaction;
    type Disposition = Disposition;

    async fn call(
        &self,
        transaction: Transaction,
        next: Next<Transaction, Disposition>,
    ) -> Disposition {
        (self.f)(self.state.clone(), transaction, next).await
    }
}

pub fn from_fn<St, Transaction, Disposition, F, Fut>(
    state: St,
    f: F,
) -> AsLayer<FromFn<St, Transaction, Disposition, F>>
where
    St: Clone + Send + Sync + 'static,
    Transaction: Send + 'static,
    Disposition: Send + 'static,
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
