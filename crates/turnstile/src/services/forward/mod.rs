//! Terminal service: allow upstream.

use std::convert::Infallible;
use std::task::{Context, Poll};

use pinnacle_core::{Request, Service};

use crate::EdgeOutcome;

#[derive(Debug, Default, Clone, Copy)]
pub struct Forward;

impl Service<Request> for Forward {
    type Response = EdgeOutcome;
    type Error = Infallible;
    type Future = std::future::Ready<Result<EdgeOutcome, Infallible>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _req: Request) -> Self::Future {
        std::future::ready(Ok(EdgeOutcome::Forward))
    }
}
