use std::path::Path;
use reqwest::{Certificate, Client, Identity};
use serde::{Deserialize, Serialize};
use crate::error::Error;
use crate::utils::filesystem;

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

pub struct AwsIotClient {
    client: Client,
    endpoint: String,
    role_alias: String,
}

impl AwsIotClient {
    pub async fn new(
        endpoint: &str,
        role_alias: &str,
        ca_path: &Path,
        cert_path: &Path,
        key_path: &Path,
    ) -> Result<Self, Error> {
        let client = create_mtls_client(ca_path, cert_path, key_path).await?;
        Ok(Self {
            client,
            endpoint: endpoint.to_string(),
            role_alias: role_alias.to_string(),
        })
    }

    pub async fn get_credentials(&self) -> Result<AwsCredentialsResponse, Error> {
        let url = format!(
            "https://{}/role-aliases/{}/credentials",
            self.endpoint, self.role_alias
        );

        let response = self.client.get(&url)
            .send()
            .await
            .map_err(|e| Error::AwsIot(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(Error::AwsIot(format!("HTTP error: {}", response.status())));
        }

        let body = response.text()
            .await
            .map_err(|e| Error::AwsIot(format!("Failed to read response: {}", e)))?;

        serde_json::from_str(&body)
            .map_err(|e| Error::AwsIot(format!("Failed to parse credentials: {}", e)))
    }
}

pub fn format_credentials(creds: &AwsCredentialsResponse) -> Result<(), Error> {
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

async fn create_mtls_client(
    ca_path: &Path,
    cert_path: &Path,
    key_path: &Path,
) -> Result<Client, Error> {
    // filesystem::validate_file_permissions(key_path)?;

    let ca_cert = filesystem::load_pem(ca_path)?;
    let client_cert = filesystem::load_pem(cert_path)?;
    let client_key = filesystem::load_pem(key_path)?;

    let identity = Identity::from_pem(&[client_cert, client_key].concat())?;
    let ca_cert = Certificate::from_pem(&ca_cert)?;

    Client::builder()
        .use_rustls_tls()
        .add_root_certificate(ca_cert)
        .identity(identity)
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(Into::into)
}