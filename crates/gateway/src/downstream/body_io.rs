use async_trait::async_trait;
use pingora::proxy::Session;
use pinnacle_core::BodyIo;

pub struct Body {
    inner: *mut Session,
}

impl Body {
    pub unsafe fn new(inner: *mut Session) -> Self {
        Self { inner }
    }
}

unsafe impl Send for Body {}

#[async_trait]
impl BodyIo for Body {
    async fn read_all(&mut self) -> Vec<u8> {
        let inner = unsafe { &mut *self.inner };
        let mut buf = Vec::new();
        while let Ok(Some(chunk)) = inner.read_request_body().await {
            buf.extend_from_slice(&chunk);
            if buf.len() > 16 * 1024 {
                break;
            }
        }
        buf
    }
}
