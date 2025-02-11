use crate::circuit_breaker;
use crate::config::EnvironmentProfile;
use crate::utils::errors::{Error, Result};
use reqwest::{Certificate, Client, Identity};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use std::{fs, path::Path};
use tracing::instrument;
// Import the Result type alias

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
    let ca_cert = load_pem(&config.ca_path).map_err(|e| Error::LoadCaCert {
        path: config.ca_path.clone(),
        source: e,
    })?;

    let client_cert = load_pem(&config.cert_path).map_err(|e| Error::LoadClientCert {
        path: config.cert_path.clone(),
        source: e,
    })?;

    let client_key = load_pem(&config.key_path).map_err(|e| Error::LoadPrivateKey {
        path: config.key_path.clone(),
        source: e,
    })?;

    let identity = Identity::from_pem(&[client_cert, client_key].concat())
        .map_err(|e| Error::HttpClient(e))?;

    let ca_cert = Certificate::from_pem(&ca_cert).map_err(|e| Error::HttpClient(e))?;

    Client::builder()
        .use_rustls_tls()
        .add_root_certificate(ca_cert)
        .identity(identity)
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(Error::HttpClient)
}

pub async fn get_aws_credentials(
    env_profile: &EnvironmentProfile,
    app_config: &super::config::Config,
    client: &Client,
) -> Result<AwsCredentialsResponse> {
    let url = format!(
        "https://{}/role-aliases/{}/credentials",
        env_profile.aws_iot_endpoint, env_profile.role_alias
    );

    if circuit_breaker::is_open(
        &app_config.cache_dir,
        app_config.circuit_breaker_threshold,
        app_config.cool_down_seconds,
    ) {
        return Err(Error::CircuitBreakerOpen);
    }

    let max_attempts = 3;
    let mut attempts = 0;
    let mut delay = Duration::from_secs(1);

    loop {
        attempts += 1;
        match client.get(&url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    return response
                        .json::<AwsCredentialsResponse>()
                        .await
                        .map_err(Error::HttpClient);
                }

                circuit_breaker::record_failure(&app_config.cache_dir);
                return Err(Error::CredentialsRequest {
                    url: url.clone(),
                    status: response.status(),
                });
            }
            Err(e) => {
                circuit_breaker::record_failure(&app_config.cache_dir);
                tracing::error!(error = ?e, url = %url, "Failed to send credentials request");

                if attempts >= max_attempts {
                    return Err(Error::HttpClient(e));
                }

                tracing::info!("Retrying AWS credentials request in {:?}", delay);
                tokio::time::sleep(delay).await;
                delay *= 2;
            }
        }
    }
}

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
