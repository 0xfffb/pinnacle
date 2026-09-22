#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransactionKind {
    Get,
    Post,
    Put,
    Delete,
    Patch,
    Options,
    Head,
    Trace,
    Connect,
    Other(String),
}

impl From<&str> for TransactionKind {
    fn from(value: &str) -> Self {
        match value.to_ascii_uppercase().as_str() {
            "GET" => Self::Get,
            "POST" => Self::Post,
            "PUT" => Self::Put,
            "DELETE" => Self::Delete,
            "PATCH" => Self::Patch,
            "OPTIONS" => Self::Options,
            "HEAD" => Self::Head,
            "TRACE" => Self::Trace,
            "CONNECT" => Self::Connect,
            other => Self::Other(other.to_string()),
        }
    }
}

impl ToString for TransactionKind {
    fn to_string(&self) -> String {
        match self {
            Self::Get => "GET".to_string(),
            Self::Post => "POST".to_string(),
            Self::Put => "PUT".to_string(),
            Self::Delete => "DELETE".to_string(),
            Self::Patch => "PATCH".to_string(),
            Self::Options => "OPTIONS".to_string(),
            Self::Head => "HEAD".to_string(),
            Self::Trace => "TRACE".to_string(),
            Self::Connect => "CONNECT".to_string(),
            Self::Other(other) => other.to_string(),
        }
    }
}