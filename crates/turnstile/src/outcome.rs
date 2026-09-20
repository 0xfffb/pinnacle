//! Result of a turnstile edge evaluation (gateway only forwards this).

/// What the gateway should do after turnstile handles a request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EdgeOutcome {
    /// Proxy the request upstream.
    Forward,
    /// Inner layers request a challenge page; outer challenge service
    /// turns this into HTML on the way back out.
    Challenge,
    /// Send a terminal response to the client.
    Respond {
        status: u16,
        content_type: &'static str,
        body: Vec<u8>,
        /// `Set-Cookie` header values (one cookie per entry).
        cookies: Vec<String>,
    },
}

impl EdgeOutcome {
    pub fn text(status: u16, body: impl Into<String>) -> Self {
        Self::Respond {
            status,
            content_type: "text/plain; charset=utf-8",
            body: body.into().into_bytes(),
            cookies: Vec::new(),
        }
    }

    pub fn html(status: u16, body: impl Into<String>) -> Self {
        Self::Respond {
            status,
            content_type: "text/html; charset=utf-8",
            body: body.into().into_bytes(),
            cookies: Vec::new(),
        }
    }

    pub fn js(body: impl Into<String>) -> Self {
        Self::Respond {
            status: 200,
            content_type: "application/javascript; charset=utf-8",
            body: body.into().into_bytes(),
            cookies: Vec::new(),
        }
    }

    pub fn empty(status: u16) -> Self {
        Self::Respond {
            status,
            content_type: "text/plain; charset=utf-8",
            body: Vec::new(),
            cookies: Vec::new(),
        }
    }

    pub fn with_cookie(mut self, cookie: impl Into<String>) -> Self {
        if let Self::Respond { cookies, .. } = &mut self {
            cookies.push(cookie.into());
        }
        self
    }

    /// Compact log form (no response body).
    pub fn log_label(&self) -> String {
        match self {
            Self::Forward => "Forward".into(),
            Self::Challenge => "Challenge".into(),
            Self::Respond {
                status,
                content_type,
                body,
                cookies,
            } => format!(
                "Respond{{status={status}, content_type={content_type}, body_len={}, cookies={}}}",
                body.len(),
                cookies.len()
            ),
        }
    }
}
