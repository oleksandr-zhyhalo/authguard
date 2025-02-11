use crate::utils::errors::{Error, Result};
use std::str::FromStr;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct RequestId(Uuid);

impl RequestId {
    pub fn new() -> Self {
        RequestId(Uuid::new_v4())
    }
}

impl std::fmt::Display for RequestId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl FromStr for LogLevel {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "error" => Ok(LogLevel::Error),
            "warn" => Ok(LogLevel::Warn),
            "info" => Ok(LogLevel::Info),
            "debug" => Ok(LogLevel::Debug),
            "trace" => Ok(LogLevel::Trace),
            _ => Err(format!("Invalid log level: {}", s)),
        }
    }
}

impl LogLevel {
    fn to_filter_directive(&self) -> &'static str {
        match self {
            LogLevel::Error => "error",
            LogLevel::Warn => "warn",
            LogLevel::Info => "info",
            LogLevel::Debug => "debug",
            LogLevel::Trace => "trace",
        }
    }
}

pub struct LogConfig {
    pub directory: String,
    pub file_name: String,
    pub level: LogLevel,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            directory: "/var/log/authguard".to_string(),
            file_name: "authguard.log".to_string(),
            level: LogLevel::Info,
        }
    }
}

pub fn setup_logging(config: &LogConfig) -> Result<()> {
    create_log_dir(&config.directory)?;

    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(format!("{}/{}", config.directory, config.file_name))
        .map_err(|e| Error::Logging(format!("Failed to open log file: {}", e)))?;

    let file_layer = fmt::layer()
        .with_writer(log_file)
        .with_ansi(false)
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .with_filter(EnvFilter::new(config.level.to_filter_directive()));

    let stdout_layer = fmt::layer()
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .with_filter(EnvFilter::new(config.level.to_filter_directive()));

    tracing_subscriber::registry()
        .with(file_layer)
        .with(stdout_layer)
        .try_init()
        .map_err(|e| Error::Logging(format!("Failed to initialize logging: {}", e)))?;

    Ok(())
}

fn create_log_dir(path: &str) -> Result<()> {
    std::fs::create_dir_all(path).map_err(Error::Io)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o750);
        std::fs::set_permissions(path, perms).map_err(Error::Io)?;
    }

    Ok(())
}
