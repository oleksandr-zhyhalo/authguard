use serde::Deserialize;
use std::path::{Path, PathBuf};
use crate::utils::errors::{Error, ConfigError, Result};

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default = "default_cache_dir")]
    pub cache_dir: PathBuf,

    #[serde(default = "default_log_dir")]
    pub log_dir: PathBuf,

    #[serde(default = "default_circuit_breaker_threshold")]
    pub circuit_breaker_threshold: u32,

    #[serde(default = "default_cool_down_seconds")]
    pub cool_down_seconds: u64,

    #[serde(rename = "environment")]
    pub env_config: EnvironmentConfig,
}

#[derive(Debug, Deserialize)]
pub struct EnvironmentConfig {
    pub current: String,
    #[serde(flatten)]
    pub profiles: std::collections::HashMap<String, EnvironmentProfile>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct EnvironmentProfile {
    pub aws_iot_endpoint: String,
    pub role_alias: String,
    pub cert_path: PathBuf,
    pub key_path: PathBuf,
    pub ca_path: PathBuf,
}

fn default_cache_dir() -> PathBuf {
    PathBuf::from("/var/cache/authguard")
}

fn default_log_dir() -> PathBuf {
    PathBuf::from("/var/log/authguard")
}

fn default_circuit_breaker_threshold() -> u32 {
    3
}

fn default_cool_down_seconds() -> u64 {
    60
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = Self::find_config_file()?;

        let settings = config::Config::builder()
            .add_source(config::File::from(config_path))
            .build()
            .map_err(|e| Error::Config(ConfigError::LoadError(e.to_string())))?;

        settings
            .try_deserialize()
            .map_err(|e| Error::Config(ConfigError::LoadError(e.to_string())))
    }

    fn find_config_file() -> Result<PathBuf> {
        let paths = [
            "/etc/authguard/authguard.toml",
            "./authguard.toml"
        ];

        for path in &paths {
            let path = PathBuf::from(path);
            if path.exists() {
                return Ok(path);
            }
        }

        Err(Error::Config(ConfigError::LoadError(
            "No configuration file found in default locations".to_string()
        )))
    }

    pub fn active_profile(&self) -> Result<&EnvironmentProfile> {
        self.env_config.profiles
            .get(&self.env_config.current)
            .ok_or_else(|| Error::Config(ConfigError::MissingEnvironment(
                self.env_config.current.clone()
            )))
    }

    pub fn validate_paths(&self) -> Result<()> {
        let profile = self.active_profile()?;

        let validate = |path: &Path, desc: &str| {
            if !path.exists() {
                Err(Error::Config(ConfigError::FileNotFound {
                    file: path.to_path_buf(),
                    description: desc.to_string(),
                }))
            } else {
                Ok(())
            }
        };

        validate(&profile.cert_path, "Client certificate")?;
        validate(&profile.key_path, "Private key")?;
        validate(&profile.ca_path, "CA certificate")?;

        Ok(())
    }
}