//! Request passed through the tower service stack.

use async_trait::async_trait;

use crate::Context;

/// Downstream session I/O (Pingora session, test double, …).
#[async_trait]
pub trait SessionIo: Send {
    async fn read_body(&mut self) -> Vec<u8>;
}

/// Edge request: context plus optional session.
///
/// `session` is only valid for the duration of one `Turnstile::call`
/// (gateway must keep the real session alive across that await).
pub struct Request {
    pub ctx: Context,
    session: Option<*mut dyn SessionIo>,
    /// Filled after the first successful read (or set by tests).
    cached: Option<Vec<u8>>,
}

// SAFETY: gateway never shares one `Request` across tasks; call finishes
// before the session borrow ends.
unsafe impl Send for Request {}

impl Request {
    pub fn new(ctx: Context) -> Self {
        Self {
            ctx,
            session: None,
            cached: None,
        }
    }

    pub fn with_session(ctx: Context, session: &mut dyn SessionIo) -> Self {
        let ptr: *mut dyn SessionIo = session;
        Self {
            ctx,
            // SAFETY: caller keeps `session` alive for the whole `Turnstile::call`.
            session: Some(unsafe {
                std::mem::transmute::<*mut dyn SessionIo, *mut dyn SessionIo>(ptr)
            }),
            cached: None
        }
    }

    pub fn with_body(mut self, body: impl Into<Vec<u8>>) -> Self {
        self.cached = Some(body.into());
        self
    }

    /// Read body once from the session (subsequent calls reuse the cache).
    pub async fn read_body(&mut self) -> Vec<u8> {
        if let Some(body) = &self.cached {
            return body.clone();
        }
        let body = match self.session {
            Some(ptr) => unsafe { &mut *ptr }.read_body().await,
            None => Vec::new(),
        };
        self.cached = Some(body.clone());
        body
    }

    /// Consume the body (session read happens at most once).
    pub async fn take_body(&mut self) -> Vec<u8> {
        if let Some(body) = self.cached.take() {
            self.session = None;
            return body;
        }
        match self.session.take() {
            Some(ptr) => unsafe { &mut *ptr }.read_body().await,
            None => Vec::new(),
        }
    }
}
