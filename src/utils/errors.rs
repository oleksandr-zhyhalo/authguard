use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    #[error("Failed to load CA certificate from {path}: {source}")]
    LoadCaCert {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("Failed to load client certificate from {path}: {source}")]
    LoadClientCert {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("Failed to load private key from {path}: {source}")]
    LoadPrivateKey {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("HTTP client error: {0}")]
    HttpClient(#[from] reqwest::Error),

    #[error("Credentials request to {url} failed with status {status}")]
    CredentialsRequest {
        url: String,
        status: reqwest::StatusCode,
    },

    #[error("JSON parsing error: {0}")]
    JsonParse(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Cache error: {0}")]
    Cache(String),

    #[error("Circuit breaker open")]
    CircuitBreakerOpen,

    #[error("Invalid expiration format: {0}")]
    InvalidExpiration(String),

    #[error("Logging setup error: {0}")]
    Logging(String),
}

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Missing environment configuration: {0}")]
    MissingEnvironment(String),

    #[error("File not found: {file} ({description})")]
    FileNotFound {
        file: PathBuf,
        description: String,
    },

    #[error("Configuration load error: {0}")]
    LoadError(String),
}

// Result type alias for cleaner signatures
pub type Result<T> = std::result::Result<T, Error>;