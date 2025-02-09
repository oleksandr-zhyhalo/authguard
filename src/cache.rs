#![feature(file_lock)]
use crate::aws_iot::AwsCredentialsResponse;
use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::fs::OpenOptions;
use std::io::Read;
use std::path::PathBuf;

const CACHE_FILE: &str = "/var/cache/authguard/creds_cache.json";
pub fn read_cached_credentials() -> Result<Option<AwsCredentialsResponse>> {
    let path = PathBuf::from(CACHE_FILE);
    if path.exists() {
        return Ok(None);
    };
    let file = OpenOptions::new()
        .read(true)
        .open(&path)
        .with_context(|| format!("Failed to open cache file {}", path.display()))?;
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

pub fn write_cached_credentials(creds: &AwsCredentialsResponse) -> Result<()> {
    let path = PathBuf::from(CACHE_FILE);
    // Open (or create) the file for writing and acquire an exclusive lock
    let file = OpenOptions::new().write(true).create(true).open(&path)
        .with_context(|| format!("Failed to open cache file for writing: {}", path.display()))?;
    file.lock_exclusive()
        .with_context(|| "Failed to acquire exclusive lock on cache file")?;

    let data = serde_json::to_string(creds)
        .with_context(|| "Failed to serialize credentials for caching")?;

    std::fs::write(&path, data)
        .with_context(|| format!("Failed to write to cache file: {}", path.display()))?;

    file.unlock()
        .with_context(|| "Failed to release lock on cache file")?;
    Ok(())
}