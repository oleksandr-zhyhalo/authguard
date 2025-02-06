use serde::{Deserialize, Serialize};
use std::{path::Path, time::{SystemTime, UNIX_EPOCH}};
use crate::{aws::client::AwsCredentialsResponse, error::Error};

#[derive(Serialize, Deserialize)]
struct CachedCredentials {
    credentials: AwsCredentialsResponse,
    expires_at: u64,
}

pub struct CredentialManager;

impl CredentialManager {
    pub fn load(config: &crate::config::Config) -> Option<AwsCredentialsResponse> {
        if !config.cache_enabled || !config.cache_path.exists() {
            return None;
        }

        let data = std::fs::read(&config.cache_path).ok()?;
        let cached: CachedCredentials = serde_json::from_slice(&data).ok()?;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()?
            .as_secs();

        if now + config.cache_threshold_seconds < cached.expires_at {
            Some(cached.credentials)
        } else {
            None
        }
    }

    pub fn store(config: &crate::config::Config, creds: &AwsCredentialsResponse) -> Result<(), Error> {
        if !config.cache_enabled {
            return Ok(());
        }

        let expires_at = chrono::DateTime::parse_from_rfc3339(&creds.credentials.expiration)
            .map_err(|e| Error::CacheError(e.to_string()))?
            .timestamp() as u64;

        let cached = CachedCredentials {
            credentials: creds.clone(),
            expires_at,
        };

        if let Some(parent) = config.cache_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let data = serde_json::to_vec(&cached)?;
        std::fs::write(&config.cache_path, data)?;
        Ok(())
    }
}