use bytes::Bytes;
use http::header::CONTENT_LENGTH;
use pingora::http::ResponseHeader;
use pingora::prelude::*;
use pingora::proxy::Session;
use pinnacle_core::Response;

pub async fn response(session: &mut Session, res: Response<Bytes>) -> Result<bool> {
    let (parts, body) = res.into_parts();
    let mut header = ResponseHeader::build(parts.status.as_u16(), None)?;

    for (name, value) in parts.headers.iter() {
        if name == CONTENT_LENGTH {
            continue;
        }
        let name = name.as_str().to_owned();
        header.insert_header(name, value)?;
    }
    header.insert_header("Content-Length", body.len().to_string())?;

    session
        .write_response_header(Box::new(header), false)
        .await?;
    session.write_response_body(Some(body), true).await?;
    Ok(true)
}
