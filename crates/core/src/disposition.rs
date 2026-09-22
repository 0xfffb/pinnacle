mod reply;

pub use reply::Reply;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Disposition {
    Allow,
    Respond {
        reply: Reply,
        reason: Option<String>,
    },
    Reject {
        reply: Reply,
        reason: Option<String>,
    },
}

impl Disposition {
    pub fn allow() -> Self {
        Self::Allow
    }

    pub fn respond(reply: Reply) -> Self {
        Self::Respond {
            reply,
            reason: None,
        }
    }

    pub fn reason(mut self, reason: impl Into<String>) -> Self {
        if let Self::Respond {
            reason: slot, ..
        } = &mut self
        {
            *slot = Some(reason.into());
        }
        self
    }

    pub fn is_allow(&self) -> bool {
        matches!(self, Self::Allow)
    }

    pub fn reply(&self) -> Option<&Reply> {
        match self {
            Self::Allow => None,
            Self::Respond { reply, .. } => Some(reply),
            Self::Reject { reply, .. } => Some(reply),
        }
    }

    pub fn reason_str(&self) -> Option<&str> {
        match self {
            Self::Allow => None,
            Self::Respond { reason, .. } => reason.as_deref(),
            Self::Reject { reason, .. } => reason.as_deref(),
        }
    }
}
