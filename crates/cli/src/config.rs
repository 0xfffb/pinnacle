use std::fs;
use std::path::Path;

use serde::Deserialize;

#[derive(Debug)]
pub struct ConfigError {
    pub message: String,
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ConfigError {}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default = "default_listen")]
    pub listen: String,
    #[serde(default = "default_upstream")]
    pub upstream: String,
}

fn default_listen() -> String {
    "0.0.0.0:6188".into()
}

fn default_upstream() -> String {
    "127.0.0.1:8080".into()
}

impl Config {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        let raw = fs::read_to_string(path).map_err(|e| ConfigError {
            message: format!("read {}: {e}", path.display()),
        })?;
        Self::parse(&raw).map_err(|e| ConfigError {
            message: format!("parse {}: {e}", path.display()),
        })
    }

    pub fn parse(raw: &str) -> Result<Self, String> {
        toml::from_str(raw).map_err(|e| e.to_string())
    }

    pub fn upstream_peer(&self) -> Result<(String, u16), String> {
        Self::parse_host_port(&self.upstream)
    }

    pub fn validate_listen(&self) -> Result<(), String> {
        Self::parse_host_port(&self.listen).map(|_| ())
    }

    pub fn parse_host_port(addr: &str) -> Result<(String, u16), String> {
        let (host, port) = addr
            .rsplit_once(':')
            .ok_or_else(|| format!("expected host:port, got {addr}"))?;
        let port: u16 = port
            .parse()
            .map_err(|_| format!("invalid port in {addr}"))?;
        if host.is_empty() {
            return Err("empty host".into());
        }
        Ok((host.to_string(), port))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_listen_and_upstream() {
        let cfg = Config::parse(
            r#"
listen = "0.0.0.0:7000"
upstream = "10.0.0.1:9000"
"#,
        )
        .unwrap();

        assert_eq!(cfg.listen, "0.0.0.0:7000");
        assert_eq!(cfg.upstream_peer().unwrap(), ("10.0.0.1".into(), 9000));
    }

    #[test]
    fn defaults_when_empty() {
        let cfg = Config::parse("").unwrap();
        assert_eq!(cfg.listen, "0.0.0.0:6188");
        assert_eq!(cfg.upstream, "127.0.0.1:8080");
    }
}
