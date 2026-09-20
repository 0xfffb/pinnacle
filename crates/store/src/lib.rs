//! Pluggable state store (memory today, Redis/DB later).

mod memory;
mod traits;

pub use memory::MemoryStore;
pub use traits::{BanRecord, ChallengeSession, Store};
