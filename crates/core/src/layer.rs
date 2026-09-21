//! Middleware as `async fn(req, next)` — Tower plumbing is provided for you.
//!
//! # How the pieces fit together
//!
//! ```text
//!  you write one of:
//!    ┌─ async fn(state, req, next) ──from_fn_with_state──┐
//!    │                                                   │
//!    └─ impl LayerService (struct) ───layer_service──────┤
//!                                                        ▼
//!                                              LayerServiceLayer   (tower::Layer)
//!                                                        │  .layer(inner)
//!                                                        ▼
//!                                                   LayerSvc        (tower::Service)
//!                                                        │  call(req)
//!                                                        ▼
//!                                              LayerService::call(req, Next)
//!                                                        │  next.run(req)
//!                                                        ▼
//!                                                   inner service
//! ```
//!
//! | You want… | Call |
//! |---|---|
//! | Plain `async fn` + shared state | [`from_fn_with_state`] |
//! | A struct with helper methods | [`layer_service`] |

mod from_fn;
mod next;
mod service;

pub use from_fn::from_fn_with_state;
pub use next::Next;
pub use service::{layer_service, LayerService, LayerServiceLayer};
