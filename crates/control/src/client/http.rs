use anyhow::Context;
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::{Method, Request};
use hyper_util::client::legacy::Client;
use hyperlocal::{UnixConnector, Uri};

pub struct ControlClient {
    pub(super) socket: String,
    pub(super) client: Client<UnixConnector, Full<Bytes>>,
}

impl ControlClient {
    pub fn new(socket: impl Into<String>) -> Self {
        Self {
            socket: socket.into(),
            client: Client::builder(hyper_util::rt::TokioExecutor::new())
                .build(UnixConnector),
        }
    }

    pub(super) async fn get(&self, path: &str) -> anyhow::Result<Bytes> {
        let uri: hyper::Uri = Uri::new(&self.socket, path).into();
        let req = Request::builder()
            .method(Method::GET)
            .uri(uri)
            .body(Full::new(Bytes::new()))?;
        let res = self
            .client
            .request(req)
            .await
            .context("control API unreachable — is pinnacle serve running?")?;
        Ok(res.into_body().collect().await?.to_bytes())
    }

    pub(super) async fn post_json(
        &self,
        path: &str,
        body: serde_json::Value,
    ) -> anyhow::Result<Bytes> {
        let uri: hyper::Uri = Uri::new(&self.socket, path).into();
        let req = Request::builder()
            .method(Method::POST)
            .uri(uri)
            .header("content-type", "application/json")
            .body(Full::new(Bytes::from(serde_json::to_vec(&body)?)))?;
        let res = self
            .client
            .request(req)
            .await
            .context("control API unreachable — is pinnacle serve running?")?;
        Ok(res.into_body().collect().await?.to_bytes())
    }

    pub(super) async fn delete(&self, path: &str) -> anyhow::Result<Bytes> {
        let uri: hyper::Uri = Uri::new(&self.socket, path).into();
        let req = Request::builder()
            .method(Method::DELETE)
            .uri(uri)
            .body(Full::new(Bytes::new()))?;
        let res = self
            .client
            .request(req)
            .await
            .context("control API unreachable — is pinnacle serve running?")?;
        Ok(res.into_body().collect().await?.to_bytes())
    }
}
