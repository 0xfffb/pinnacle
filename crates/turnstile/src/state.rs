//! Shared application state injected into store-backed turnstile layers.
//!
//! [`TurnstileState`] is cloned once per request invocation; all heavy data
//! must be behind an [`Arc`] so cloning remains O(1).

use std::sync::Arc;

use pinnacle_store::Store;

/// State shared across store-backed turnstile layers.
///
/// Currently holds only the storage backend. Additional shared fields
/// (e.g. rate-limit config, feature flags) should be added here as needed.
///
/// Pass to [`from_fn_with_state`] so each layer function receives a clone
/// without managing its own `Arc` fields.
///
/// [`from_fn_with_state`]: pinnacle_core::from_fn_with_state
#[derive(Clone)]
pub struct TurnstileState {
    /// Storage backend: ban list, challenge sessions, pass tokens, counters.
    pub store: Arc<dyn Store>,
}
