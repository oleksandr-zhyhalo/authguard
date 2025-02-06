use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub aws_iot_endpoint: String,
    pub role_alias: String,
    pub cert_path: PathBuf,
    pub key_path: PathBuf,
    pub ca_path: PathBuf,
}

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Missing required configuration field: {0}")]
    MissingField(String),
    #[error("File does not exist: {0}")]
    FileNotFound(String),
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(&path).context(format!(
            "Failed to read config file: {}",
            path.as_ref().display()
        ))?;

        let mut config = config_parser::parse_config(&content)?;

        validate_file_exists(&config.cert_path, "Client certificate")?;
        validate_file_exists(&config.key_path, "Private key")?;
        validate_file_exists(&config.ca_path, "CA certificate")?;

        Ok(config)
    }
}
pub fn default_config_path() -> Result<PathBuf> {
    let paths = ["/etc/authguard/authguard.conf"];

    for path in &paths {
        let path = PathBuf::from(path);
        if path.exists() {
            return Ok(path);
        }
    }

    anyhow::bail!(
        "No configuration file found in default locations: {:?}",
        paths
    )
}

fn validate_file_exists(path: &Path, description: &str) -> Result<()> {
    if !path.exists() {
        tracing::error!(
            path = ?path,
            description = description,
            "Required file not found"
        );
        anyhow::bail!("{} not found at {}", description, path.display());
    }
    Ok(())
}

mod config_parser {
    use super::*;

    pub fn parse_config(content: &str) -> Result<Config> {
        let mut config = Config {
            aws_iot_endpoint: String::new(),
            role_alias: String::new(),
            cert_path: PathBuf::new(),
            key_path: PathBuf::new(),
            ca_path: PathBuf::new(),
        };

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let mut parts = line.splitn(2, '=');
            let key = parts.next().context("Invalid config line")?.trim();
            let value = parts.next().context("Missing config value")?.trim();

            match key {
                "aws_iot_endpoint" => config.aws_iot_endpoint = value.to_string(),
                "role_alias" => config.role_alias = value.to_string(),
                "cert_path" => config.cert_path = PathBuf::from(value),
                "key_path" => config.key_path = PathBuf::from(value),
                "ca_path" => config.ca_path = PathBuf::from(value),
                _ => tracing::warn!("Unknown configuration key: {}", key),
            }
        }

        validate_required_fields(&config)?;
        Ok(config)
    }

    fn validate_required_fields(config: &Config) -> Result<()> {
        let required = [
            ("aws_iot_endpoint", &config.aws_iot_endpoint),
            ("role_alias", &config.role_alias),
        ];

        for (name, value) in required {
            if value.is_empty() {
                tracing::error!(field = name, "Missing required configuration field");
                anyhow::bail!(ConfigError::MissingField(name.to_string()));
            }
        }

        Ok(())
    }
}
