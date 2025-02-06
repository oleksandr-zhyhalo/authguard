mod parser;

use std::path::{Path, PathBuf};
use crate::error::{Error, ConfigError};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub aws_iot_endpoint: String,
    pub role_alias: String,
    pub cert_path: PathBuf,
    pub key_path: PathBuf,
    pub ca_path: PathBuf,
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        let content = std::fs::read_to_string(&path)
            .map_err(|e| ConfigError::FileNotFound(path.as_ref().to_path_buf()))?;

        parser::parse(&content)
    }

    pub fn default_path() -> Result<PathBuf, Error> {
        let paths = ["/etc/authguard/authguard.conf"];

        paths.iter()
            .map(PathBuf::from)
            .find(|p| p.exists())
            .ok_or_else(|| ConfigError::FileNotFound(PathBuf::from("/etc/authguard/authguard.conf")).into())
    }
}