//! Shared helpers for turnstile tower services.

use std::convert::Infallible;

use futures::future::BoxFuture;

use crate::EdgeOutcome;

pub(crate) type EdgeFut = BoxFuture<'static, Result<EdgeOutcome, Infallible>>;

pub(crate) fn ok(outcome: EdgeOutcome) -> EdgeFut {
    Box::pin(async move { Ok(outcome) })
}
