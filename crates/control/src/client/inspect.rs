use super::http::ControlClient;
use crate::types::InspectResponse;

impl ControlClient {
    pub async fn inspect(&self, ip: &str) -> anyhow::Result<InspectResponse> {
        let bytes = self.get(&format!("/inspect/{ip}")).await?;
        Ok(serde_json::from_slice(&bytes)?)
    }
}
