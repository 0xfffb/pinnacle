use bytes::Bytes;
use http::header::CONTENT_TYPE;
use http::Method;
use pingora::proxy::Session;
use pinnacle_core::{ClientIp, Request};

const MAX_BODY: usize = 16 * 1024;

pub async fn request(session: &mut Session) -> Request<Bytes> {
    let (method, uri, version, headers, ip) = {
        let header = session.req_header();
        let ip = session
            .client_addr()
            .and_then(|a| a.as_inet())
            .map(|a| a.ip().to_string())
            .unwrap_or_else(|| "0.0.0.0".into());
        (
            header.method.clone(),
            header.uri.clone(),
            header.version,
            header.headers.clone(),
            ip,
        )
    };

    // Buffer small JSON POSTs (challenge verify). Leave other bodies for upstream.
    let json_post = method == Method::POST
        && headers
            .get(CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .is_some_and(|t| t.contains("application/json"));
    let body = if json_post {
        read_body(session).await
    } else {
        Bytes::new()
    };

    let mut builder = http::Request::builder()
        .method(method)
        .uri(uri)
        .version(version);
    for (name, value) in headers.iter() {
        builder = builder.header(name, value);
    }

    let mut req = builder.body(body).expect("request");
    req.extensions_mut().insert(ClientIp(ip));
    req
}

async fn read_body(session: &mut Session) -> Bytes {
    let mut body = Vec::new();
    while let Ok(Some(chunk)) = session.read_request_body().await {
        body.extend_from_slice(&chunk);
        if body.len() > MAX_BODY {
            break;
        }
    }
    Bytes::from(body)
}
