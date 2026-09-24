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
#[serde(default)]
pub struct Config {
    pub listen: String,
    pub upstream: String,
    pub control_socket: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            listen: "0.0.0.0:6188".into(),
            upstream: "127.0.0.1:8080".into(),
            control_socket: "/tmp/pinnacle.sock".into(),
        }
    }
}

impl Config {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = fs::read_to_string(path).map_err(|e| ConfigError {
            message: format!("read {}: {e}", path.display()),
        })?;
        toml::from_str(&raw).map_err(|e| ConfigError {
            message: format!("parse {}: {e}", path.display()),
        })
    }

    pub fn upstream_peer(&self) -> Result<(String, u16), String> {
        parse_host_port(&self.upstream)
    }

    pub fn validate_listen(&self) -> Result<(), String> {
        parse_host_port(&self.listen).map(|_| ())
    }
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
