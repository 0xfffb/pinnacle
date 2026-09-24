use super::http::ControlClient;
use crate::types::StatsResponse;

impl ControlClient {
    pub async fn stats(&self) -> anyhow::Result<StatsResponse> {
        let bytes = self.get("/stats").await?;
        Ok(serde_json::from_slice(&bytes)?)
    }
}
