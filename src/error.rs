use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    #[error("AWS IoT error: {0}")]
    AwsIot(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Security error: {0}")]
    Security(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("HTTP client error: {0}")]
    HttpClient(#[from] reqwest::Error),
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("File not found: {0}")]
    FileNotFound(PathBuf),

    #[error("Invalid configuration format: {0}")]
    InvalidFormat(String),
}