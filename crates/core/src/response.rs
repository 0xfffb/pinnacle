use bytes::Bytes;
use http::header::{CONTENT_TYPE, HeaderValue, SET_COOKIE};
use http::{Response, StatusCode};

#[derive(Debug, Clone)]
pub struct Respond(Response<Bytes>);

impl Respond {
    pub fn text(status: StatusCode, body: impl Into<Bytes>) -> Self {
        Self(
            Response::builder()
                .status(status)
                .header(CONTENT_TYPE, "text/plain; charset=utf-8")
                .body(body.into())
                .unwrap(),
        )
    }

    pub fn html(status: StatusCode, body: impl Into<Bytes>) -> Self {
        Self(
            Response::builder()
                .status(status)
                .header(CONTENT_TYPE, "text/html; charset=utf-8")
                .body(body.into())
                .unwrap(),
        )
    }

    pub fn javascript(body: impl Into<Bytes>) -> Self {
        Self(
            Response::builder()
                .status(StatusCode::OK)
                .header(CONTENT_TYPE, "application/javascript; charset=utf-8")
                .body(body.into())
                .unwrap(),
        )
    }

    pub fn with_cookie(mut self, cookie: impl AsRef<str>) -> Self {
        self.0.headers_mut().append(
            SET_COOKIE,
            HeaderValue::from_str(cookie.as_ref()).expect("cookie"),
        );
        self
    }

    pub fn into_response(self) -> Response<Bytes> {
        self.0
    }
}

impl From<Respond> for Response<Bytes> {
    fn from(value: Respond) -> Self {
        value.0
    }
}

impl From<Respond> for Option<Response<Bytes>> {
    fn from(value: Respond) -> Self {
        Some(value.0)
    }
}
