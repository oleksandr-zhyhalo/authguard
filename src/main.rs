use crate::cache::CredentialCache;
use crate::utils::errors::Result;
use crate::utils::logging::{LogConfig, LogLevel};

mod aws_iot;
mod cache;
mod circuit_breaker;
mod config;
mod utils;

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        tracing::error!(error = ?e, "Application error");
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let config = config::Config::load()?;

    let log_config = LogConfig {
        directory: config.log_dir.to_string_lossy().into_owned(),
        file_name: "authguard.log".to_string(),
        level: LogLevel::Info,
    };

    let _guard = utils::logging::setup_logging(&log_config)?;

    config.validate_paths()?;
    let profile = config.active_profile()?;

    let client = aws_iot::create_mtls_client(profile).await.map_err(|e| {
        tracing::error!(error = ?e, "Failed to create mTLS client");
        e
    })?;

    let cache_path = config.cache_dir.join("creds_cache.json");
    let credentials_cache = CredentialCache::new(cache_path);

    let credentials = match credentials_cache.read()? {
        Some(cached) if !credentials_cache.needs_refresh(&cached) => {
            tracing::info!("Using valid cached AWS credentials");
            cached
        }
        _ => {
            tracing::info!("No valid cache or near expiration; fetching new credentials");
            let new_creds = aws_iot::get_aws_credentials(profile, &config, &client)
                .await
                .map_err(|e| {
                    tracing::error!(error = ?e, "Failed to retrieve AWS credentials");
                    e
                })?;

            credentials_cache.write(&new_creds)?;
            circuit_breaker::record_success(&config.cache_dir);
            new_creds
        }
    };

    aws_iot::format_credential_output(&credentials)?;
    Ok(())
}
