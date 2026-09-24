use serde::{Deserialize, Serialize};

// ── ban ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct BanRequest {
    pub ip: String,
    #[serde(default = "default_reason")]
    pub reason: String,
}

fn default_reason() -> String {
    "manual".into()
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BanResponse {
    pub ok: bool,
}

// ── stats ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct StatsResponse {
    pub requests_total: u32,
    pub banned_total: u32,
}

// ── inspect ───────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct InspectResponse {
    pub ip: String,
    pub banned: bool,
    pub ban_reason: Option<String>,
    pub pass_valid: bool,
    pub request_count: u32,
}

// ── bans list ─────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct BanEntry {
    pub ip: String,
    pub reason: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BansResponse {
    pub bans: Vec<BanEntry>,
}
