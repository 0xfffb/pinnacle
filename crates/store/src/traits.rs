/// Why an identity was banned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BanRecord {
    pub reason: String,
}

impl BanRecord {
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}

/// Challenge session waiting for client verification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChallengeSession {
    pub kind: String,
    pub challenge_id: String,
    pub payload: String,
}

impl ChallengeSession {
    pub fn new(
        kind: impl Into<String>,
        challenge_id: impl Into<String>,
        payload: impl Into<String>,
    ) -> Self {
        Self {
            kind: kind.into(),
            challenge_id: challenge_id.into(),
            payload: payload.into(),
        }
    }
}

/// Backend for counters, bans, challenge sessions, and pass marks.
pub trait Store: Send + Sync {
    fn incr_request(&self, key: &str) -> u32;
    fn request_count(&self, key: &str) -> u32;
    fn is_banned(&self, key: &str) -> bool;
    fn ban(&self, key: &str, reason: &str);
    fn unban(&self, key: &str);
    fn ban_reason(&self, key: &str) -> Option<String>;
    fn ban_count(&self) -> u32;
    fn list_bans(&self) -> Vec<(String, BanRecord)>;
    fn put_challenge(&self, key: &str, session: ChallengeSession);
    fn get_challenge(&self, key: &str) -> Option<ChallengeSession>;
    fn take_challenge(&self, key: &str) -> Option<ChallengeSession>;
    /// Issue a pass token for `key` (overwrites any previous token).
    fn issue_pass(&self, key: &str) -> String;
    /// Validate that `token` is the current pass token for `key`.
    fn validate_pass(&self, key: &str, token: &str) -> bool;
    fn has_pass(&self, key: &str) -> bool;
    fn clear_passed(&self, key: &str);
}
