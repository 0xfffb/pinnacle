#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reply {
    pub status: u16,
    pub content_type: &'static str,
    pub body: Vec<u8>,
    pub cookies: Vec<String>,
}

impl Reply {
    pub fn text(status: u16, body: impl Into<String>) -> Self {
        Self {
            status,
            content_type: "text/plain; charset=utf-8",
            body: body.into().into_bytes(),
            cookies: Vec::new(),
        }
    }

    pub fn html(status: u16, body: impl Into<String>) -> Self {
        Self {
            status,
            content_type: "text/html; charset=utf-8",
            body: body.into().into_bytes(),
            cookies: Vec::new(),
        }
    }

    pub fn javascript(body: impl Into<String>) -> Self {
        Self {
            status: 200,
            content_type: "application/javascript; charset=utf-8",
            body: body.into().into_bytes(),
            cookies: Vec::new(),
        }
    }

    pub fn empty(status: u16) -> Self {
        Self {
            status,
            content_type: "text/plain; charset=utf-8",
            body: Vec::new(),
            cookies: Vec::new(),
        }
    }

    pub fn with_cookie(mut self, cookie: impl Into<String>) -> Self {
        self.cookies.push(cookie.into());
        self
    }
}