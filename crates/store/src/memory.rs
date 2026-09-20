use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{BanRecord, ChallengeSession, Store};

/// Thread-safe in-memory [`Store`] (dev / bootstrap default).
#[derive(Debug, Default)]
pub struct MemoryStore {
    inner: Mutex<StoreInner>,
}

#[derive(Debug, Default)]
struct StoreInner {
    counts: HashMap<String, u32>,
    bans: HashMap<String, BanRecord>,
    challenges: HashMap<String, ChallengeSession>,
    /// key → pass token
    passed: HashMap<String, String>,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self::default()
    }
}

fn new_pass_token() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("p_{nanos:x}")
}

impl Store for MemoryStore {
    fn incr_request(&self, key: &str) -> u32 {
        let mut inner = self.inner.lock().expect("store lock");
        let entry = inner.counts.entry(key.to_string()).or_insert(0);
        *entry = entry.saturating_add(1);
        *entry
    }

    fn request_count(&self, key: &str) -> u32 {
        let inner = self.inner.lock().expect("store lock");
        inner.counts.get(key).copied().unwrap_or(0)
    }

    fn is_banned(&self, key: &str) -> bool {
        let inner = self.inner.lock().expect("store lock");
        inner.bans.contains_key(key)
    }

    fn ban(&self, key: &str, reason: &str) {
        let mut inner = self.inner.lock().expect("store lock");
        inner.bans.insert(key.to_string(), BanRecord::new(reason));
    }

    fn unban(&self, key: &str) {
        let mut inner = self.inner.lock().expect("store lock");
        inner.bans.remove(key);
    }

    fn put_challenge(&self, key: &str, session: ChallengeSession) {
        let mut inner = self.inner.lock().expect("store lock");
        inner.challenges.insert(key.to_string(), session);
    }

    fn get_challenge(&self, key: &str) -> Option<ChallengeSession> {
        let inner = self.inner.lock().expect("store lock");
        inner.challenges.get(key).cloned()
    }

    fn take_challenge(&self, key: &str) -> Option<ChallengeSession> {
        let mut inner = self.inner.lock().expect("store lock");
        inner.challenges.remove(key)
    }

    fn issue_pass(&self, key: &str) -> String {
        let token = new_pass_token();
        let mut inner = self.inner.lock().expect("store lock");
        inner.passed.insert(key.to_string(), token.clone());
        token
    }

    fn validate_pass(&self, key: &str, token: &str) -> bool {
        let inner = self.inner.lock().expect("store lock");
        inner.passed.get(key).is_some_and(|t| t == token)
    }

    fn clear_passed(&self, key: &str) {
        let mut inner = self.inner.lock().expect("store lock");
        inner.passed.remove(key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn increments_and_bans() {
        let store = MemoryStore::new();
        assert_eq!(store.incr_request("1.1.1.1"), 1);
        assert_eq!(store.incr_request("1.1.1.1"), 2);
        assert!(!store.is_banned("1.1.1.1"));
        store.ban("1.1.1.1", "abuse");
        assert!(store.is_banned("1.1.1.1"));
    }

    #[test]
    fn pass_token_roundtrip() {
        let store = MemoryStore::new();
        let token = store.issue_pass("1.1.1.1");
        assert!(store.validate_pass("1.1.1.1", &token));
        assert!(!store.validate_pass("1.1.1.1", "wrong"));
        store.clear_passed("1.1.1.1");
        assert!(!store.validate_pass("1.1.1.1", &token));
    }

    #[test]
    fn challenge_session_roundtrip() {
        let store = MemoryStore::new();
        store.put_challenge(
            "1.1.1.1",
            ChallengeSession::new("token_echo", "chg_1", "secret"),
        );
        let session = store.take_challenge("1.1.1.1").expect("session");
        assert_eq!(session.payload, "secret");
        assert!(store.take_challenge("1.1.1.1").is_none());
    }
}
