use std::sync::Arc;

use rand::Rng;
use pinnacle_store::Store;

pub struct CookieEndpoint {
    pub path: String,
    pub cookie_cid: String,
    pub cookie_pass: String,
}

impl CookieEndpoint {

    pub fn random_path() -> String {
        const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
        let mut rng = rand::thread_rng();
        let suffix: String = (0..31)
            .map(|_| {
                let i = rng.gen_range(0..CHARSET.len());
                CHARSET[i] as char
            })
            .collect();
        format!("/{suffix}")
    }
}

#[derive(Clone)]
pub struct TurnstileState {
    pub store: Arc<dyn Store>,
    pub endpoints: Arc<CookieEndpoint>,
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_random_path() {
        let path = CookieEndpoint::random_path();
        assert!(path.starts_with("/"));
        assert!(path.len() == 32);
    }
}