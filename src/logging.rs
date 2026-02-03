//! Logging setup for the Redwood flight tracker.
//!
//! This module configures the [`tracing`] subscriber to write logs to a
//! daily-rotating file under the `logs/` directory. The log level can be
//! overridden via the `RUST_LOG` environment variable (default: `INFO`).

use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Initializes global logging to a file and returns a guard that must be held.
///
/// Creates a `logs/` directory if it does not exist, then sets up a
/// non-blocking file appender with daily rotation (`logs/redwood.log` and
/// date-suffixed archives). The tracing subscriber is configured to use
/// [`EnvFilter`] from `RUST_LOG`, with a default directive of `INFO` if unset.
/// ANSI escape codes are disabled for file output.
///
/// # Returns
///
/// A [`WorkerGuard`] that must be kept alive for the process lifetime. Dropping
/// it flushes and shuts down the log worker; the caller (e.g. `main`) should
/// store it (e.g. `let _log_guard = logging::initialize_logging();`) so that
/// logs are written until exit.
///
/// # Panics
///
/// Does not panic. Directory creation failures are ignored; logging will still
/// be initialized (writes may fail later if the path is invalid).
pub fn initialize_logging() -> WorkerGuard {
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
