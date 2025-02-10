use anyhow::{Context, Result};
use reqwest::{Certificate, Identity, Client};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};
use std::time::Duration;
use tracing::instrument;
use crate::circuit_breaker;
use crate::config::EnvironmentProfile;

#[derive(Debug, Deserialize, Serialize)]
pub struct AwsCredentialsResponse {
    pub credentials: AwsCredentials,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AwsCredentials {
    #[serde(rename = "accessKeyId")]
    pub access_key_id: String,
    #[serde(rename = "secretAccessKey")]
    pub secret_access_key: String,
    #[serde(rename = "sessionToken")]
    pub session_token: String,
    pub expiration: String,
}

#[instrument(skip(config))]
pub async fn create_mtls_client(config: &EnvironmentProfile) -> Result<Client> {
    let ca_cert = match load_pem(&config.ca_path) {
        Ok(cert) => cert,
        Err(e) => {
            tracing::error!(error = ?e, path = ?config.ca_path, "Failed to load CA certificate");
            return Err(e);
        }
    };

    let client_cert = match load_pem(&config.cert_path) {
        Ok(cert) => cert,
        Err(e) => {
            tracing::error!(error = ?e, path = ?config.cert_path, "Failed to load client certificate");
            return Err(e);
        }
    };

    let client_key = match load_pem(&config.key_path) {
        Ok(key) => key,
        Err(e) => {
            tracing::error!(error = ?e, path = ?config.key_path, "Failed to load client private key");
            return Err(e);
        }
    };

    let identity = match Identity::from_pem(&[client_cert, client_key].concat()) {
        Ok(id) => id,
        Err(e) => {
            tracing::error!(error = ?e, "Failed to create client identity");
            return Err(e.into());
        }
    };

    let ca_cert = match Certificate::from_pem(&ca_cert) {
        Ok(cert) => cert,
        Err(e) => {
            tracing::error!(error = ?e, "Failed to parse CA certificate");
            return Err(e.into());
        }
    };
    match Client::builder()
        .use_rustls_tls()
        .add_root_certificate(ca_cert)
        .identity(identity)
        .timeout(std::time::Duration::from_secs(15))
        .build()
    {
        Ok(client) => Ok(client),
        Err(e) => {
            tracing::error!(error = ?e, "Failed to build HTTP client");
            Err(e.into())
        }
    }
}

pub async fn get_aws_credentials(
    env_profile: &super::config::EnvironmentProfile,
    app_config: &super::config::Config,
    client: &Client,
) -> Result<AwsCredentialsResponse> {
    let url = format!(
        "https://{}/role-aliases/{}/credentials",
        env_profile.aws_iot_endpoint, env_profile.role_alias
    );

    if circuit_breaker::is_open(&app_config.cache_dir, app_config.circuit_breaker_threshold, app_config.cool_down_seconds) {
        anyhow::bail!("Circuit breaker is open; skipping AWS credentials call");
    }

    let max_attempts = 3;
    let mut attempts = 0;
    let mut delay = Duration::from_secs(1);
    let mut last_err = None;

    loop {
        attempts += 1;
        let response = client.get(&url).send().await;
        match response {
            Ok(resp) => {
                if resp.status().is_success() {
                    let body = resp.text().await
                        .with_context(|| "Failed to read response body")?;
                    let credentials: AwsCredentialsResponse = serde_json::from_str(&body)
                        .with_context(|| "Failed to parse credentials response")?;
                    circuit_breaker::record_success(&app_config.cache_dir);
                    tracing::info!("Successfully retrieved AWS credentials");
                    return Ok(credentials);
                } else {
                    let status = resp.status();
                    last_err = Some(anyhow::anyhow!("Credentials request failed with status: {}", status));
                    tracing::error!(status = %status, url = %url, "Request failed");
                }
            }
            Err(e) => {
                last_err = Some(e.into());
                tracing::error!(error = ?last_err, url = %url, "Failed to send credentials request");
            }
        }

        circuit_breaker::record_failure(&app_config.cache_dir);

        if attempts >= max_attempts {
            break;
        }
        tracing::info!("Retrying AWS credentials request in {:?}", delay);
        tokio::time::sleep(delay).await;
        delay *= 2;
    }

    Err(last_err.unwrap_or_else(|| anyhow::anyhow!("Unknown error retrieving AWS credentials")))
}

pub fn format_credential_output(creds: &AwsCredentialsResponse) -> Result<()> {
    let output = serde_json::json!({
        "Version": 1,
        "AccessKeyId": creds.credentials.access_key_id,
        "SecretAccessKey": creds.credentials.secret_access_key,
        "SessionToken": creds.credentials.session_token,
        "Expiration": creds.credentials.expiration
    });

    println!("{}", serde_json::to_string(&output)?);
    tracing::info!("Successfully formatted credentials for output");
    Ok(())
}

fn load_pem(path: &Path) -> Result<Vec<u8>> {
    fs::read(path)
        .context(format!("Failed to read PEM file: {}", path.display()))
}