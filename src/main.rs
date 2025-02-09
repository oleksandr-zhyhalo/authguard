use anyhow::{Context, Result};

mod config;
mod aws_iot;
mod logging;
mod cache;

#[tokio::main]
async fn main() -> Result<()> {
    let _guard = logging::setup_logging().context("Failed to initialize logging")?;

    let config_path = match config::default_config_path() {
        Ok(path) => path,
        Err(e) => {
            tracing::error!(error = ?e, "Failed to locate configuration file");
            return Err(e);
        }
    };

    let config = match config::Config::load(&config_path) {
        Ok(cfg) => {
            tracing::info!(path = ?config_path, "Loaded configuration");
            cfg
        }
        Err(e) => {
            tracing::error!(error = ?e, path = ?config_path, "Failed to load configuration");
            return Err(e);
        }
    };

    let client = match aws_iot::create_mtls_client(&config).await {
        Ok(client) => client,
        Err(e) => {
            tracing::error!(error = ?e, "Failed to create mTLS client");
            return Err(e);
        }
    };

    let credentials = match aws_iot::get_aws_credentials(&config, &client).await {
        Ok(creds) => creds,
        Err(e) => {
            tracing::error!(
                error = ?e,
                endpoint = %config.aws_iot_endpoint,
                role_alias = %config.role_alias,
                "Failed to retrieve AWS credentials"
            );
            return Err(e);
        }
    };

    if let Err(e) = aws_iot::format_credential_output(&credentials) {
        tracing::error!(error = ?e, "Failed to format credentials output");
        return Err(e);
    }

    Ok(())
}