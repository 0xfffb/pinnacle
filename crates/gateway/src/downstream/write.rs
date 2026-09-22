use bytes::Bytes;
use pingora::http::ResponseHeader;
use pingora::prelude::*;
use pingora::proxy::Session;
use pinnacle_core::Reply;

pub async fn reply(inner: &mut Session, reply: &Reply) -> Result<bool> {
    let mut header = ResponseHeader::build(reply.status, None)?;
    header.insert_header("Content-Type", reply.content_type)?;
    header.insert_header("Content-Length", reply.body.len().to_string())?;
    for cookie in &reply.cookies {
        header.append_header("Set-Cookie", cookie)?;
    }
    inner.write_response_header(Box::new(header), false).await?;
    inner
        .write_response_body(Some(Bytes::copy_from_slice(&reply.body)), true)
        .await?;
    Ok(true)
}
