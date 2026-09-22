use std::collections::HashMap;

use async_trait::async_trait;

mod kind;

pub use kind::TransactionKind;

#[async_trait]
pub trait BodyIo: Send {
    async fn read_all(&mut self) -> Vec<u8>;
}

enum Body {
    Incoming(Box<dyn BodyIo>),
    Full(Vec<u8>),
}

pub struct Transaction {
    pub meta: HashMap<String, String>,
    pub headers: HashMap<String, String>,
    pub method: TransactionKind,
    body: Option<Body>,
}

impl Transaction {
    pub fn new(
        meta: HashMap<String, String>,
        headers: HashMap<String, String>,
        method: TransactionKind,
    ) -> Self {
        Self {
            meta,
            headers,
            method,
            body: None,
        }
    }

    pub fn with_body(mut self, body: impl BodyIo + 'static) -> Self {
        self.body = Some(Body::Incoming(Box::new(body)));
        self
    }

    pub fn with_full(mut self, bytes: impl Into<Vec<u8>>) -> Self {
        self.body = Some(Body::Full(bytes.into()));
        self
    }

    pub async fn body(&mut self) -> Vec<u8> {
        match self.body.take() {
            Some(Body::Incoming(mut io)) => {
                let bytes = io.read_all().await;
                self.body = Some(Body::Full(bytes.clone()));
                bytes
            }
            Some(Body::Full(bytes)) => {
                self.body = Some(Body::Full(bytes.clone()));
                bytes
            }
            None => Vec::new(),
        }
    }
}
