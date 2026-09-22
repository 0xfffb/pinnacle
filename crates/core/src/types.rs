use bytes::Bytes;
use http::Response;

/// `None` = proxy upstream; `Some` = write this response.
pub type Decision = Option<Response<Bytes>>;

#[derive(Clone, Debug)]
pub struct ClientIp(pub String);

impl ClientIp {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
