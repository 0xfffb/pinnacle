//! Edge decision produced by middleware.

use crate::Action;

/// Final (or intermediate short-circuit) decision from a middleware stage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decision {
    pub action: Action,
    pub detail: String,
    pub stage: &'static str,
}

impl Decision {
    pub fn allow(stage: &'static str, detail: impl Into<String>) -> Self {
        Self {
            action: Action::Allow,
            detail: detail.into(),
            stage,
        }
    }

    pub fn block(stage: &'static str, detail: impl Into<String>) -> Self {
        Self {
            action: Action::Block,
            detail: detail.into(),
            stage,
        }
    }

    pub fn challenge(stage: &'static str, detail: impl Into<String>) -> Self {
        Self {
            action: Action::Challenge,
            detail: detail.into(),
            stage,
        }
    }
}
