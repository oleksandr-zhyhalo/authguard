use anyhow::Context;
use cache::CredentialManager;


mod config;
mod aws;
mod error;
mod logging;
mod utils;
mod cache;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _guard = logging::setup().context("Failed to initialize logging")?;

    let config_path = config::Config::default_path()
        .context("Failed to find default config path")?;

    let config = config::Config::load(&config_path)
        .context("Failed to load configuration")?;

    // Try to load cached credentials first
    if let Some(cached_credentials) = CredentialManager::load(&config) {
        tracing::info!("Using cached credentials");
        aws::format_credentials(&cached_credentials)
            .context("Failed to format credentials output")?;
        return Ok(());
    }

    // If no valid cached credentials, fetch new ones
    let aws_client = aws::AwsIotClient::new(
        &config.aws_iot_endpoint,
        &config.role_alias,
        &config.ca_path,
        &config.cert_path,
        &config.key_path
    ).await.context("Failed to create AWS client")?;

    let credentials = aws_client.get_credentials()
        .await.context("Failed to retrieve credentials")?;

    // Store the new credentials in cache
    if let Err(e) = CredentialManager::store(&config, &credentials) {
        tracing::warn!("Failed to cache credentials: {}", e);
    }

    aws::format_credentials(&credentials)
        .context("Failed to format credentials output")?;

    Ok(())
}