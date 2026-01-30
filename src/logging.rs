use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

pub fn initialize_logging() -> WorkerGuard {
    // Create 'logs' directory if it doesn't exist
    let _ = std::fs::create_dir_all("logs");
    
    let file_appender = tracing_appender::rolling::daily("logs", "redwood.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .with(fmt::layer().with_writer(non_blocking).with_ansi(false))
        .init();

    tracing::info!("Logging initialized successfully.");
    guard
}