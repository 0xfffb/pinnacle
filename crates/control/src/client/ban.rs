use super::http::ControlClient;
use crate::types::{BanResponse, BansResponse};

impl ControlClient {
    pub async fn ban(&self, ip: &str, reason: &str) -> anyhow::Result<BanResponse> {
        let bytes = self
            .post_json("/bans", serde_json::json!({ "ip": ip, "reason": reason }))
            .await?;
        Ok(serde_json::from_slice(&bytes)?)
    }

    pub async fn unban(&self, ip: &str) -> anyhow::Result<BanResponse> {
        let bytes = self.delete(&format!("/bans/{ip}")).await?;
        Ok(serde_json::from_slice(&bytes)?)
    }

    pub async fn bans(&self) -> anyhow::Result<BansResponse> {
        let bytes = self.get("/bans").await?;
        Ok(serde_json::from_slice(&bytes)?)
    }
}
