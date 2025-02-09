// src/main.rs
use anyhow::{Context, Result};

mod aws_iot;
mod cache;
mod circuit_breaker;
mod config;
mod logging;

#[tokio::main]
async fn main() -> Result<()> {
    // Set up logging (this also creates the necessary log directories)
    let _guard = logging::setup_logging().context("Failed to initialize logging")?;

    // Load the configuration file from the default location.
    let config_path = config::default_config_path().or_else(|e| {
        tracing::error!(error = ?e, "Failed to locate configuration file");
        Err(e)
    })?;
    let config = config::Config::load(&config_path).or_else(|e| {
        tracing::error!(error = ?e, path = ?config_path, "Failed to load configuration");
        Err(e)
    })?;

    // Create the mTLS-enabled HTTP client.
    let client = aws_iot::create_mtls_client(&config)
        .await
        .or_else(|e| {
            tracing::error!(error = ?e, "Failed to create mTLS client");
            Err(e)
        })?;

    // Attempt to read cached credentials.
    // The cache module uses file locks (via fs2) to prevent race conditions.
    let credentials = match cache::read_cached_credentials() {
        Ok(Some(cached)) if !cache::needs_refresh(&cached) => {
            tracing::info!("Using valid cached AWS credentials.");
            cached
        }
        _ => {
            tracing::info!("No valid cache or near expiration; fetching new credentials.");
            let new_creds = aws_iot::get_aws_credentials(&config, &client)
                .await
                .or_else(|e| {
                    tracing::error!(error = ?e, "Failed to retrieve AWS credentials");
                    Err(e)
                })?;
            cache::write_cached_credentials(&new_creds)?;
            new_creds
        }
    };

    // Format and output the credentials in the JSON format expected by the AWS CLI.
    aws_iot::format_credential_output(&credentials)?;

    Ok(())
}
