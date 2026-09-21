//! Turnstile layer functions and supporting types.
//!
//! Each layer is a plain `async fn(State, Request, Next<…>) -> EdgeOutcome`
//! registered via [`from_fn_with_state`](pinnacle_core::from_fn_with_state).

mod bannd;
mod challenge;
mod count;
mod detector;
mod forward;
mod policy;

pub use bannd::banned;
pub use challenge::{challenge, CookieChallenger, COOKIE_CID, COOKIE_PASS, SCRIPT_PATH};
pub use count::count;
pub use detector::{detect, Detector, HeuristicDetector, RiskVerdict};
pub use forward::Forward;
pub use policy::{policy, PolicyDecision, PolicyEffect, PolicyEngine, PolicySet, Rule};
