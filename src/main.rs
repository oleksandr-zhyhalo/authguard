use anyhow::{Context, Result};

mod config;
mod aws_iot;
mod logging;

#[tokio::main]
async fn main() -> Result<()> {
    let _guard = logging::setup_logging().context("Failed to initialize logging")?;

    let config_path = config::default_config_path()
        .context("Failed to locate configuration file")?;

    let config = config::Config::load(&config_path)
        .context("Failed to load configuration")?;

    tracing::info!(path = ?config_path, "Loaded configuration");

    let client = aws_iot::create_mtls_client(&config)
        .await
        .context("Failed to create mTLS client")?;

    let credentials = aws_iot::get_aws_credentials(&config, &client)
        .await
        .context("Failed to retrieve AWS credentials")?;

    aws_iot::format_credential_output(&credentials)
        .context("Failed to format credentials output")?;

    Ok(())
}