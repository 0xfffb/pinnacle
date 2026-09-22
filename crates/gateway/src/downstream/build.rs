use std::collections::HashMap;

use pingora::proxy::Session;
use pinnacle_core::{Transaction, TransactionKind};

use super::body_io::Body;

pub fn transaction(inner: &mut Session) -> Transaction {
    let req = inner.req_header();
    let method = TransactionKind::from(req.method.as_str());

    let mut meta = HashMap::new();
    meta.insert("path".into(), req.uri.path().to_owned());
    meta.insert(
        "ip".into(),
        inner
            .client_addr()
            .and_then(|a| a.as_inet())
            .map(|a| a.ip().to_string())
            .unwrap_or_else(|| "0.0.0.0".into()),
    );

    let mut headers = HashMap::new();
    for (name, value) in req.headers.iter() {
        if let Ok(v) = value.to_str() {
            headers.insert(name.as_str().to_ascii_lowercase(), v.to_owned());
        }
    }
    
    let body = unsafe { Body::new(inner as *mut Session) };
    Transaction::new(meta, headers, method).with_body(body)
}
