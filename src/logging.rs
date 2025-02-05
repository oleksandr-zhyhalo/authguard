use anyhow::{Context, Result};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::{non_blocking, rolling};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};

pub fn setup_logging() -> Result<WorkerGuard> {
    let log_dir = "/var/log/authguard";
    create_log_dir(log_dir)?;

    let file_appender =
        rolling::RollingFileAppender::new(rolling::Rotation::DAILY, log_dir, "authguard.log");

    let (non_blocking_writer, guard) = non_blocking(file_appender);

    let file_layer = fmt::layer()
        .json()
        .with_writer(non_blocking_writer)
        .with_filter(EnvFilter::new("info"));

    let stdout_layer = fmt::layer()
        .with_writer(std::io::stderr)
        .with_ansi(true)
        .with_timer(fmt::time::time())
        .with_filter(EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(file_layer)
        .with(stdout_layer)
        .try_init()
        .context("Failed to initialize logging subsystem")?;

    Ok(guard)
}

fn create_log_dir(path: &str) -> Result<()> {
    std::fs::create_dir_all(path).context(format!("Failed to create log directory: {}", path))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o750);
        std::fs::set_permissions(path, perms)
            .context(format!("Failed to set permissions for {}", path))?;
    }

    Ok(())
}
