use crate::config::Config;
use reqwest;
use rustls_pemfile::{self};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs;
use chrono::{DateTime, Utc};

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

    pub expiration: String, // You might want to convert this to DateTime later
}

pub async fn build_client(config: &Config) -> Result<reqwest::Client, Box<dyn Error>> {
    let ca_cert = reqwest::Certificate::from_pem(&fs::read(&config.ca_path)?)?;

    // Load client identity (cert + key must be in PKCS8 format)
    let identity = {
        let cert = fs::read(&config.cert_path)?;
        let key = fs::read(&config.key_path)?;
        reqwest::Identity::from_pem(&[cert, key].concat())?
    };

    // Build client with rustls TLS configuration
    let client = reqwest::Client::builder()
        .use_rustls_tls()
        .add_root_certificate(ca_cert)
        .identity(identity)
        .build()?;

    Ok(client)
}

pub async fn get_aws_credentials(
    config: &Config,
    client: &reqwest::Client,
) -> Result<AwsCredentialsResponse, Box<dyn Error>> {
    let credentials_url = format!(
        "https://{}/role-aliases/{}/credentials",
        config.aws_iot_endpoint, config.role_alias
    );

    let response = client.get(&credentials_url).send().await?;

    if !response.status().is_success() {
        return Err(format!("Request failed with status: {}", response.status()).into());
    }

    let response_body = response.text().await?;
    let credentials_response: AwsCredentialsResponse = serde_json::from_str(&response_body)
        .map_err(|e| format!("Failed to parse credentials response: {}", e))?;

    Ok(credentials_response)
}
