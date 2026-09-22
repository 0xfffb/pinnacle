use async_trait::async_trait;
use pinnacle_core::{Disposition, LayerService, Next, Reply, Transaction};

use crate::state::TurnstileState;

const CAPTCHA_PATH: &str = "/__captcha";

#[derive(Clone)]
pub struct CaptchaChallengeService {
    state: TurnstileState,
}

impl CaptchaChallengeService {
    pub fn new(state: TurnstileState) -> Self {
        Self { state }
    }
}

#[async_trait]
impl LayerService for CaptchaChallengeService {
    async fn forward(
        &self,
        transaction: Transaction,
        next: Next<Transaction, Disposition>,
    ) -> Disposition {
        let _ = &self.state;
        let path = transaction.meta.get("path").cloned().unwrap_or_default();
        if path == CAPTCHA_PATH {
            return Disposition::respond(Reply::html(202, "<h1>Captcha</h1>"));
        }
        next.forward(transaction).await
    }
}
