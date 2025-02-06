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

#[instrument(skip(config))]
pub async fn create_mtls_client(config: &super::config::Config) -> Result<Client> {
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

#[instrument(skip(config, client))]
pub async fn get_aws_credentials(
    config: &super::config::Config,
    client: &Client,
) -> Result<AwsCredentialsResponse> {
    let url = format!(
        "https://{}/role-aliases/{}/credentials",
        config.aws_iot_endpoint, config.role_alias
    );

    let response = match client.get(&url).send().await {
        Ok(resp) => resp,
        Err(e) => {
            tracing::error!(error = ?e, url = %url, "Failed to send credentials request");
            return Err(e.into());
        }
    };

    if !response.status().is_success() {
        let error_msg = format!("Credentials request failed with status: {}", response.status());
        tracing::error!(status = %response.status(), url = %url, "Request failed");
        anyhow::bail!(error_msg);
    }

    let body = match response.text().await {
        Ok(text) => text,
        Err(e) => {
            tracing::error!(error = ?e, "Failed to read response body");
            return Err(e.into());
        }
    };

    let credentials = match serde_json::from_str(&body) {
        Ok(creds) => creds,
        Err(e) => {
            tracing::error!(error = ?e, "Failed to parse credentials response");
            return Err(e.into());
        }
    };

    tracing::info!("Successfully retrieved AWS credentials");
    Ok(credentials)
}

pub fn format_credential_output(creds: &AwsCredentialsResponse) -> Result<()> {
    let output = serde_json::json!({
        "Version": 1,
        "AccessKeyId": creds.credentials.access_key_id,
        "SecretAccessKey": creds.credentials.secret_access_key,
        "SessionToken": creds.credentials.session_token,
        "Expiration": creds.credentials.expiration
    });

    // Only print to stdout, not to logs
    println!("{}", serde_json::to_string(&output)?);
    tracing::info!("Successfully formatted credentials for output");
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