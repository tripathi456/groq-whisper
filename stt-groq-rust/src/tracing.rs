//! Tracing module for structured logging.
//!
//! This module provides functionality to set up JSON-formatted logging using the tracing framework.

use std::io;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    EnvFilter,
};

/// Initialize tracing with JSON formatting
///
/// This function sets up tracing with JSON-formatted logs directed to stdout.
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

    // Create a non-blocking writer for stdout
    let (non_blocking, guard) = tracing_appender::non_blocking(io::stdout());

    // Initialize the tracing subscriber with JSON formatting
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(non_blocking)
        .with_span_events(FmtSpan::CLOSE)
        .json()
        .init();

    // Return the guard to ensure logs are flushed
    guard
}