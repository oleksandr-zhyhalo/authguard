use crate::utils::errors::{Error, Result};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::{non_blocking, rolling};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};
use std::str::FromStr;
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
    pub json_output: bool,
    pub retention_days: u32,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            directory: "/var/log/authguard".to_string(),
            file_name: "authguard.log".to_string(),
            level: LogLevel::Info,
            json_output: true,
            retention_days: 7,
        }
    }
}

pub fn setup_logging(config: &LogConfig) -> Result<WorkerGuard> {
    create_log_dir(&config.directory)?;

    let file_appender = rolling::RollingFileAppender::builder()
        .rotation(rolling::Rotation::DAILY)
        .filename_prefix(&config.file_name)
        .filename_suffix("log")
        .build(&config.directory)
        .map_err(|e| Error::Logging(format!("Failed to create file appender: {}", e)))?;

    let (non_blocking_writer, guard) = non_blocking(file_appender);

    let file_layer = if config.json_output {
        fmt::layer()
            .json()
            .with_writer(non_blocking_writer)
            .with_target(true)
            .with_thread_ids(true)
            .with_thread_names(true)
            .with_file(true)
            .with_line_number(true)
            .with_span_events(fmt::format::FmtSpan::NEW | fmt::format::FmtSpan::CLOSE)
            .with_timer(fmt::time::time())
            .with_filter(EnvFilter::new(config.level.to_filter_directive()))
            .boxed()
    } else {
        fmt::layer()
            .with_writer(non_blocking_writer)
            .with_target(true)
            .with_thread_ids(true)
            .with_thread_names(true)
            .with_file(true)
            .with_line_number(true)
            .with_span_events(fmt::format::FmtSpan::NEW | fmt::format::FmtSpan::CLOSE)
            .with_timer(fmt::time::time())
            .with_filter(EnvFilter::new(config.level.to_filter_directive()))
            .boxed()
    };

    let stdout_layer = fmt::layer()
        .with_writer(std::io::stderr)
        .with_ansi(true)
        .with_target(true)
        .with_thread_ids(true)
        .with_filter(EnvFilter::new(config.level.to_filter_directive()))
        .boxed();

    tracing_subscriber::registry()
        .with(file_layer)
        .with(stdout_layer)
        .try_init()
        .map_err(|e| Error::Logging(format!("Failed to initialize logging subsystem: {}", e)))?;

    clean_old_logs(&config.directory, config.retention_days)?;

    Ok(guard)
}

fn create_log_dir(path: &str) -> Result<()> {
    std::fs::create_dir_all(path)
        .map_err(Error::Io)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o750);
        std::fs::set_permissions(path, perms)
            .map_err(Error::Io)?;
    }

    Ok(())
}

fn clean_old_logs(log_dir: &str, retention_days: u32) -> Result<()> {
    use chrono::{DateTime, Utc};
    use std::time::SystemTime;

    let retention_duration = chrono::Duration::days(retention_days as i64);
    let cutoff = Utc::now() - retention_duration;

    for entry in std::fs::read_dir(log_dir).map_err(Error::Io)? {
        let entry = entry.map_err(Error::Io)?;
        let metadata = entry.metadata().map_err(Error::Io)?;

        if metadata.is_file() {
            let modified: DateTime<Utc> = metadata
                .modified()
                .map_err(Error::Io)?
                .into();

            if modified < cutoff {
                std::fs::remove_file(entry.path()).map_err(Error::Io)?;
                tracing::debug!(path = ?entry.path(), "Removed old log file");
            }
        }
    }

    Ok(())
}