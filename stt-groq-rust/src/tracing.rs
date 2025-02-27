//! Tracing module for structured logging.
//!
//! This module provides functionality to set up JSON-formatted logging using the tracing framework.

// Re-export tracing macros and types
pub use tracing::{debug, error, info, warn, instrument, info_span};

use std::io;
use std::path::PathBuf;
use time::macros::format_description;
use time::OffsetDateTime;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling;
use tracing_subscriber::{
    fmt::{format::FmtSpan},
    EnvFilter,
    layer::SubscriberExt,
    Registry,
};

/// Initialize tracing with JSON formatting
///
/// This function sets up tracing with JSON-formatted logs directed to stdout and a log file.
/// It also configures the log level based on the RUST_LOG environment variable,
/// defaulting to "info" if not set.
///
/// Returns a guard that must be kept alive for the duration of the program
/// to ensure logs are flushed properly.
pub fn init_tracing() -> (WorkerGuard, WorkerGuard) {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }

    let log_dir = PathBuf::from("logs");
    std::fs::create_dir_all(&log_dir).expect("Failed to create logs directory");

    // File appender
    let file_appender = tracing_appender::rolling::daily(log_dir, "app.log");
    let (non_blocking_file, guard_file) = tracing_appender::non_blocking(file_appender);
    
    // Console appender
    let (non_blocking_stdout, guard_stdout) = tracing_appender::non_blocking(io::stdout());

    // Create layers
    let fmt_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_span_events(FmtSpan::CLOSE)
        .with_writer(non_blocking_stdout);

    let file_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_span_events(FmtSpan::CLOSE)
        .with_ansi(false)
        .with_writer(non_blocking_file);

    let subscriber = Registry::default()
        .with(EnvFilter::from_default_env())
        .with(fmt_layer)
        .with(file_layer);

    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set global subscriber");

    (guard_file, guard_stdout)
}