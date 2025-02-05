use anyhow::{Context, Result};
use reqwest::{Certificate, Identity, Client};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};
use tracing::instrument;

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

#[instrument]
pub async fn create_mtls_client(config: &super::config::Config) -> Result<Client> {
    // validate_key_permissions(&config.key_path)
    //     .context("Invalid private key permissions")?;

    let ca_cert = load_pem(&config.ca_path)
        .context("Failed to load CA certificate")?;

    let client_cert = load_pem(&config.cert_path)
        .context("Failed to load client certificate")?;

    let client_key = load_pem(&config.key_path)
        .context("Failed to load client private key")?;

    let identity = Identity::from_pem(&[client_cert, client_key].concat())
        .context("Failed to create client identity")?;

    let ca_cert = Certificate::from_pem(&ca_cert)
        .context("Failed to parse CA certificate")?;

    Client::builder()
        .use_rustls_tls()
        .add_root_certificate(ca_cert)
        .identity(identity)
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .context("Failed to build HTTP client")
}

#[instrument(skip(client))]
pub async fn get_aws_credentials(
    config: &super::config::Config,
    client: &Client,
) -> Result<AwsCredentialsResponse> {
    let url = format!(
        "https://{}/role-aliases/{}/credentials",
        config.aws_iot_endpoint, config.role_alias
    );

    let response = client.get(&url)
        .send()
        .await
        .context("Failed to send credentials request")?;

    if !response.status().is_success() {
        anyhow::bail!(
            "Credentials request failed with status: {}",
            response.status()
        );
    }

    let body = response.text()
        .await
        .context("Failed to read response body")?;

    serde_json::from_str(&body)
        .context("Failed to parse credentials response")
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
    Ok(())
}

fn load_pem(path: &Path) -> Result<Vec<u8>> {
    fs::read(path)
        .context(format!("Failed to read PEM file: {}", path.display()))
}

fn validate_key_permissions(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = fs::metadata(path)
            .context(format!("Failed to get metadata for {}", path.display()))?;

        let mode = metadata.permissions().mode();
        if mode & 0o7777 != 0o600 {
            anyhow::bail!(
                "Insecure permissions for {}: expected 0600, got {:o}",
                path.display(),
                mode & 0o7777
            );
        }
    }
    Ok(())
}