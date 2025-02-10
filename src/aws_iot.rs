use reqwest::{Certificate, Identity, Client};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};
use std::time::Duration;
use tracing::instrument;
use crate::circuit_breaker;
use crate::config::EnvironmentProfile;
use crate::utils::errors::{Error, Result};

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
    tracing::debug!(
        cert_path = ?config.cert_path,
        ca_path = ?config.ca_path,
        key_path = ?config.key_path,
        "Creating mTLS client"
    );

    let ca_cert = load_pem(&config.ca_path)
        .map_err(|e| Error::LoadCaCert {
            path: config.ca_path.clone(),
            source: e,
        })?;

    let client_cert = load_pem(&config.cert_path)
        .map_err(|e| Error::LoadClientCert {
            path: config.cert_path.clone(),
            source: e,
        })?;

    let client_key = load_pem(&config.key_path)
        .map_err(|e| Error::LoadPrivateKey {
            path: config.key_path.clone(),
            source: e,
        })?;

    let identity = Identity::from_pem(&[client_cert, client_key].concat())
        .map_err(Error::HttpClient)?;

    let ca_cert = Certificate::from_pem(&ca_cert)
        .map_err(Error::HttpClient)?;

    tracing::debug!("Successfully loaded all certificates");

    Client::builder()
        .use_rustls_tls()
        .add_root_certificate(ca_cert)
        .identity(identity)
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(Error::HttpClient)
}

#[instrument(skip(client), fields(endpoint = %env_profile.aws_iot_endpoint))]
pub async fn get_aws_credentials(
    env_profile: &EnvironmentProfile,
    app_config: &super::config::Config,
    client: &Client,
) -> Result<AwsCredentialsResponse> {
    let url = format!(
        "https://{}/role-aliases/{}/credentials",
        env_profile.aws_iot_endpoint, env_profile.role_alias
    );

    if circuit_breaker::is_open(&app_config.cache_dir, app_config.circuit_breaker_threshold, app_config.cool_down_seconds) {
        tracing::warn!("Circuit breaker is open, skipping credentials request");
        return Err(Error::CircuitBreakerOpen);
    }

    let max_attempts = 3;
    let mut attempts = 0;
    let mut delay = Duration::from_secs(1);

    while attempts < max_attempts {
        attempts += 1;
        tracing::debug!(attempt = attempts, "Attempting to fetch AWS credentials");

        match client.get(&url).send().await {
            Ok(response) => {
                let status = response.status();
                tracing::debug!(status = %status, "Received response");

                if status.is_success() {
                    match response.json::<AwsCredentialsResponse>().await {
                        Ok(creds) => {
                            tracing::info!("Successfully retrieved AWS credentials");
                            return Ok(creds);
                        }
                        Err(e) => {
                            tracing::error!(error = ?e, "Failed to parse credentials response");
                            circuit_breaker::record_failure(&app_config.cache_dir)?;
                            // Use HttpClient error instead of trying to convert to JsonParse
                            return Err(Error::HttpClient(e));
                        }
                    }
                }

                circuit_breaker::record_failure(&app_config.cache_dir)?;
                if attempts == max_attempts {
                    return Err(Error::CredentialsRequest {
                        url: url.clone(),
                        status,
                    });
                }
            }
            Err(e) => {
                tracing::error!(error = ?e, attempt = attempts, "Request failed");
                circuit_breaker::record_failure(&app_config.cache_dir)?;

                if attempts == max_attempts {
                    return Err(Error::HttpClient(e));
                }
            }
        }

        tracing::debug!(delay_ms = ?delay.as_millis(), "Retrying request after delay");
        tokio::time::sleep(delay).await;
        delay *= 2;
    }

    unreachable!("Loop should return before this point");
}

#[instrument(skip(creds))]
pub fn format_credential_output(creds: &AwsCredentialsResponse) -> Result<()> {
    let output = serde_json::json!({
        "Version": 1,
        "AccessKeyId": creds.credentials.access_key_id,
        "SecretAccessKey": creds.credentials.secret_access_key,
        "SessionToken": creds.credentials.session_token,
        "Expiration": creds.credentials.expiration
    });

    serde_json::to_string(&output)
        .map_err(Error::JsonParse)
        .and_then(|json| {
            println!("{}", json);
            tracing::info!("Successfully formatted credentials for output");
            Ok(())
        })
}

fn load_pem(path: &Path) -> std::io::Result<Vec<u8>> {
    fs::read(path)
}