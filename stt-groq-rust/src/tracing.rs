//! Tracing module for structured logging.
//!
//! This module provides functionality to set up JSON-formatted logging using the tracing framework.

// Re-export tracing macros and types
pub use tracing::{debug, error, info, warn, instrument, info_span};

use std::io;
use std::path::PathBuf;
use time::format_description;
use time::OffsetDateTime;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling;
use tracing_subscriber::{
    fmt::{format::FmtSpan},
    EnvFilter,
};

/// Initialize tracing with JSON formatting
///
/// This function sets up tracing with JSON-formatted logs directed to stdout and a log file.
/// It also configures the log level based on the RUST_LOG environment variable,
/// defaulting to "info" if not set.
///
/// Returns a guard that must be kept alive for the duration of the program
/// to ensure logs are flushed properly.
pub fn init_tracing() -> WorkerGuard {
    // Set default log level to info if RUST_LOG is not set
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }

    let log_dir = PathBuf::from("logs");
    std::fs::create_dir_all(&log_dir).expect("Failed to create logs directory");

    let file_name = format!(
        "app_{}.log",
        OffsetDateTime::now_utc()
            .format(&format_description!("[year]-[month]-[day]_[hour]-[minute]-[second]"))
            .unwrap()
    );
    let file_appender = rolling::never(&log_dir, file_name);
    let (non_blocking_file, guard_file) = tracing_appender::non_blocking(file_appender);

    let (non_blocking_stdout, _) = tracing_appender::non_blocking(io::stdout());

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(non_blocking_stdout)
        .with_span_events(FmtSpan::CLOSE)
        .json()
        .init();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(non_blocking_file)
        .with_span_events(FmtSpan::CLOSE)
        .json()
        .init();

    guard_file
}