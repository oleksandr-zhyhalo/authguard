use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};

pub fn setup() -> Result<WorkerGuard, std::io::Error> {
    let log_dir = "/var/log/authguard";
    std::fs::create_dir_all(log_dir)?;

    let file_appender = tracing_appender::rolling::daily(log_dir, "authguard.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let file_layer = fmt::layer()
        .json()
        .with_writer(non_blocking)
        .with_target(true)
        .with_filter(EnvFilter::new("info"));

    let stdout_layer = fmt::layer()
        .with_writer(std::io::stderr)
        .with_filter(EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(file_layer)
        .with(stdout_layer)
        .init();

    Ok(guard)
}