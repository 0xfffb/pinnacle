mod build;
mod write;

use pingora::proxy::Session;
use pinnacle_core::{Bytes, Decision, Request};

pub struct Downstream<'a> {
    inner: &'a mut Session,
}

impl<'a> Downstream<'a> {
    pub fn new(inner: &'a mut Session) -> Self {
        Self { inner }
    }

    pub async fn request(&mut self) -> Request<Bytes> {
        build::request(self.inner).await
    }

    pub async fn apply(&mut self, decision: Decision) -> pingora::Result<bool> {
        match decision {
            Some(res) => write::response(self.inner, res).await,
            None => Ok(false),
        }
    }
}
