//! Pingora session adapter: HTTP ↔ [`Context`] / [`SessionIo`].

use async_trait::async_trait;
use bytes::Bytes;
use pingora::http::ResponseHeader;
use pingora::prelude::*;
use pingora::proxy::Session;
use pinnacle_core::{Context, Request, SessionIo};
use pinnacle_turnstile::EdgeOutcome;

/// Thin helper around a Pingora downstream session.
pub struct Downstream<'a> {
    session: &'a mut Session,
}

impl<'a> Downstream<'a> {
    pub fn new(session: &'a mut Session) -> Self {
        Self { session }
    }

    /// Build a turnstile [`Request`] that can read this session's body.
    pub fn request(&mut self) -> Request {
        let ctx = self.context();
        Request::with_session(ctx, self)
    }

    pub fn context(&self) -> Context {
        let req = self.session.req_header();
        let mut ctx = Context::new(
            req.uri.path(),
            self.session
                .client_addr()
                .and_then(|a| a.as_inet())
                .map(|a| a.ip().to_string())
                .unwrap_or_else(|| "0.0.0.0".into()),
            self.header("user-agent").unwrap_or_default(),
        )
        .with_method(req.method.as_str());
        if let Some(v) = self.header("cookie") {
            ctx = ctx.with_header("cookie", v);
        }
        ctx
    }

    pub fn header(&self, name: &str) -> Option<String> {
        self.session
            .req_header()
            .headers
            .get(name)
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned)
    }

    pub async fn reply(
        &mut self,
        status: u16,
        content_type: &str,
        body: &[u8],
        cookies: &[String],
    ) -> Result<bool> {
        let mut header = ResponseHeader::build(status, None)?;
        header.insert_header("Content-Type", content_type)?;
        header.insert_header("Content-Length", body.len().to_string())?;
        header.insert_header("Cache-Control", "no-store")?;
        for cookie in cookies {
            header.append_header("Set-Cookie", cookie)?;
        }
        self.session
            .write_response_header(Box::new(header), false)
            .await?;
        self.session
            .write_response_body(Some(Bytes::copy_from_slice(body)), true)
            .await?;
        Ok(true)
    }

    pub async fn apply(&mut self, outcome: EdgeOutcome) -> Result<bool> {
        match outcome {
            EdgeOutcome::Forward => Ok(false),
            EdgeOutcome::Challenge => {
                self.reply(503, "text/plain; charset=utf-8", b"challenge_required", &[])
                    .await
            }
            EdgeOutcome::Respond {
                status,
                content_type,
                body,
                cookies,
            } => self.reply(status, content_type, &body, &cookies).await,
        }
    }
}

#[async_trait]
impl SessionIo for Downstream<'_> {
    async fn read_body(&mut self) -> Vec<u8> {
        let mut buf = Vec::new();
        while let Ok(Some(chunk)) = self.session.read_request_body().await {
            buf.extend_from_slice(&chunk);
            if buf.len() > 16 * 1024 {
                break;
            }
        }
        buf
    }
}
