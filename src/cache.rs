use crate::aws_iot::AwsCredentialsResponse;
use anyhow::{Context, Result};
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
            .with_context(|| format!("Failed to open cache file {}", self.path.display()))?;
        file.lock_shared()
            .with_context(|| "Failed to acquire shared lock on cache file")?;
        let mut data = String::new();
        {
            use std::io::BufReader;
            let mut reader = BufReader::new(&file);
            reader
                .read_to_string(&mut data)
                .with_context(|| "Failed to read cache file")?;
        }
        file.unlock()
            .with_context(|| "Failed to release lock on cache file")?;
        let creds: AwsCredentialsResponse =
            serde_json::from_str(&data).with_context(|| "Failed to parse cached credentials")?;
        Ok(Some(creds))
    }
    pub fn write(&self, creds: &AwsCredentialsResponse) -> Result<()> {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .open(&self.path)
            .with_context(|| format!("Failed to open cache file for writing: {}", self.path.display()))?;
        file.lock_exclusive()
            .with_context(|| "Failed to acquire exclusive lock on cache file")?;

        let data = serde_json::to_string(&creds)
            .with_context(|| "Failed to serialize credentials for caching")?;

        std::fs::write(&self.path, data)
            .with_context(|| format!("Failed to write to cache file: {}", self.path.display()))?;

        file.unlock()
            .with_context(|| "Failed to release lock on cache file")?;
        Ok(())
    }
    pub fn needs_refresh(&self, creds: &AwsCredentialsResponse) -> bool {
        let expiration = match DateTime::parse_from_rfc3339(&creds.credentials.expiration) {
            Ok(dt) => dt.with_timezone(&Utc),
            Err(err) => {
                eprintln!("Warning: Could not parse expiration timestamp: {}", err);
                return true;
            }
        };

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
}