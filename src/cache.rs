use crate::aws_iot::AwsCredentialsResponse;
use crate::utils::errors::{Error, Result};
use chrono::{DateTime, Duration, Utc};
use fs2::FileExt;
use std::fs::OpenOptions;
use std::io::Read;
use std::path::{Path, PathBuf};

pub struct CredentialCache {
    path: PathBuf,
}

impl CredentialCache {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }

    pub fn read(&self) -> Result<Option<AwsCredentialsResponse>> {
        if !self.path.exists() {
            return Ok(None);
        }

        let file = OpenOptions::new()
            .read(true)
            .open(&self.path)
            .map_err(|e| {
                Error::Cache(format!(
                    "Failed to open cache file {}: {}",
                    self.path.display(),
                    e
                ))
            })?;

        file.lock_shared().map_err(|e| {
            Error::Cache(format!(
                "Failed to acquire shared lock on cache file: {}",
                e
            ))
        })?;

        let mut data = String::new();
        {
            use std::io::BufReader;
            let mut reader = BufReader::new(&file);
            reader
                .read_to_string(&mut data)
                .map_err(|e| Error::Cache(format!("Failed to read cache file: {}", e)))?;
        }

        file.unlock()
            .map_err(|e| Error::Cache(format!("Failed to release lock on cache file: {}", e)))?;

        let creds = serde_json::from_str(&data).map_err(Error::JsonParse)?;

        Ok(Some(creds))
    }

    pub fn write(&self, creds: &AwsCredentialsResponse) -> Result<()> {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .open(&self.path)
            .map_err(|e| {
                Error::Cache(format!(
                    "Failed to open cache file for writing {}: {}",
                    self.path.display(),
                    e
                ))
            })?;

        file.lock_exclusive().map_err(|e| {
            Error::Cache(format!(
                "Failed to acquire exclusive lock on cache file: {}",
                e
            ))
        })?;

        let data = serde_json::to_string(&creds).map_err(Error::JsonParse)?;

        std::fs::write(&self.path, data).map_err(|e| {
            Error::Cache(format!(
                "Failed to write to cache file {}: {}",
                self.path.display(),
                e
            ))
        })?;

        file.unlock()
            .map_err(|e| Error::Cache(format!("Failed to release lock on cache file: {}", e)))?;

        Ok(())
    }

    pub fn needs_refresh(&self, creds: &AwsCredentialsResponse) -> bool {
        match DateTime::parse_from_rfc3339(&creds.credentials.expiration) {
            Ok(expiration) => {
                let now = Utc::now();
                let refresh_time = expiration - Duration::minutes(10);

                tracing::debug!(
                    "Current time: {} | Expiration: {} | Refresh threshold: {}",
                    now,
                    expiration,
                    refresh_time
                );

                now >= refresh_time
            }
            Err(err) => {
                tracing::warn!(error = ?err, "Invalid expiration format");
                true
            }
        }
    }
}
