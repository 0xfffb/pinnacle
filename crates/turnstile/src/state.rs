use std::sync::Arc;

use pinnacle_store::Store;

#[derive(Clone)]
pub struct TurnstileState {
    pub store: Arc<dyn Store>,
}
