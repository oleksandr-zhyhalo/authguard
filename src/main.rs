use anyhow::{Context, Result};
use crate::cache::CredentialCache;

mod aws_iot;
mod cache;
mod circuit_breaker;
mod config;
mod logging;

#[tokio::main]
async fn main() -> Result<()> {
    let _guard = logging::setup_logging().context("Failed to initialize logging")?;

    let config = config::Config::load()?;
    config.validate_paths()?;
    let profile = config.active_profile()?;

    let client = aws_iot::create_mtls_client(profile)
        .await
        .or_else(|e| {
            tracing::error!(error = ?e, "Failed to create mTLS client");
            Err(e)
        })?;

    let cache_path = config.cache_dir.join("creds_cache.json");
    let credentials_cache : CredentialCache = CredentialCache::new(cache_path);
    let credentials = match credentials_cache.read() {
        Ok(Some(cached)) if !credentials_cache.needs_refresh(&cached) => {
            tracing::info!("Using valid cached AWS credentials.");
            cached
        }
        _ => {
            tracing::info!("No valid cache or near expiration; fetching new credentials.");
            let new_creds = aws_iot::get_aws_credentials(profile,&config, &client)
                .await
                .or_else(|e| {
                    tracing::error!(error = ?e, "Failed to retrieve AWS credentials");
                    Err(e)
                })?;
            credentials_cache.write(&new_creds)?;
            new_creds
        }
    };

    aws_iot::format_credential_output(&credentials)?;
    Ok(())
}
