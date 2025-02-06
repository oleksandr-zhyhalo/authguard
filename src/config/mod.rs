mod parser;

use serde::Deserialize;
use std::path::PathBuf;
use anyhow::Context;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub aws_iot_endpoint: String,
    pub role_alias: String,
    pub cert_path: PathBuf,
    pub key_path: PathBuf,
    pub ca_path: PathBuf,
    #[serde(default = "default_cache_enabled")]
    pub cache_enabled: bool,
    #[serde(default = "default_cache_threshold")]
    pub cache_threshold_seconds: u64,
    #[serde(default = "default_cache_path")]
    pub cache_path: PathBuf,
}

fn default_cache_enabled() -> bool { true }
fn default_cache_threshold() -> u64 { 300 } // 5 minutes
fn default_cache_path() -> PathBuf { PathBuf::from("/var/cache/authguard/credentials.json") }

impl Config {
    pub fn load<P: AsRef<std::path::Path>>(path: P) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)
            .context("Failed to read config file")?;
        
        parser::parse_config(&content)
    }

    pub fn default_path() -> anyhow::Result<std::path::PathBuf> {
        Ok(std::path::PathBuf::from("/etc/authguard/authguard.conf"))
    }
}