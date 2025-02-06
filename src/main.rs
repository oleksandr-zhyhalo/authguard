use anyhow::Context;


mod config;
mod aws;
mod error;
mod logging;
mod utils;
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _guard = logging::setup().context("Failed to initialize logging")?;

    let config_path = config::Config::default_path()
        .context("Failed to find default config path")?;

    let config = config::Config::load(&config_path)
        .context("Failed to load configuration")?;

    let aws_client = aws::AwsIotClient::new(
        &config.aws_iot_endpoint,
        &config.role_alias,
        &config.ca_path,
        &config.cert_path,
        &config.key_path
    ).await.context("Failed to create AWS client")?;

    let credentials = aws_client.get_credentials()
        .await.context("Failed to retrieve credentials")?;

    aws::format_credentials(&credentials)
        .context("Failed to format credentials output")?;

    Ok(())
}