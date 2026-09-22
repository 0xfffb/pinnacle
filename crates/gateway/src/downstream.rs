mod body_io;
mod build;
mod write;

use pingora::proxy::Session;
use pinnacle_core::{Disposition, Transaction};

pub struct Downstream<'a> {
    inner: &'a mut Session,
}

impl<'a> Downstream<'a> {
    pub fn new(inner: &'a mut Session) -> Self {
        Self { inner }
    }

    pub fn transaction(&mut self) -> Transaction {
        build::transaction(self.inner)
    }

    pub async fn apply(&mut self, disposition: Disposition) -> pingora::Result<bool> {
        match disposition.reply() {
            Some(reply) => write::reply(self.inner, reply).await,
            None => Ok(false),
        }
    }
}
