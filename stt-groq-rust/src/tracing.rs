//! Tracing module for structured logging.
pub use tracing::{debug, error, info, warn, instrument, info_span};

use std::io;
use std::path::PathBuf;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling;
use tracing_subscriber::fmt::format;
use tracing_subscriber::{
    fmt::format::FmtSpan,
    EnvFilter,
    layer::SubscriberExt,
    Registry,
};

pub fn init_tracing() -> (WorkerGuard, WorkerGuard) {
    // Set log level from env or default to info.
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }

    let log_dir = PathBuf::from("logs");
    std::fs::create_dir_all(&log_dir).expect("Failed to create logs directory");

    // File appender (JSON format, no ANSI).
    let file_appender = rolling::daily(log_dir, "app.log");
    let (non_blocking_file, guard_file) = tracing_appender::non_blocking(file_appender);

    // Console appender (with ANSI color coding).
    let (non_blocking_stdout, guard_stdout) = tracing_appender::non_blocking(io::stdout());

    // Create a custom formatter for console output.
    let console_formatter = format()
        .with_level(true)
        .with_target(false)
        .with_thread_names(true)
        .with_timer(tracing_subscriber::fmt::time::ChronoLocal::rfc_3339())
        .with_ansi(true);

    let fmt_layer_console = tracing_subscriber::fmt::layer()
        .event_format(console_formatter)
        .with_writer(non_blocking_stdout);

    // File layer uses JSON formatting (no colors).
    let fmt_layer_file = tracing_subscriber::fmt::layer()
        .json()
        .with_span_events(FmtSpan::CLOSE)
        .with_ansi(false)
        .with_writer(non_blocking_file);

    let subscriber = Registry::default()
        .with(EnvFilter::from_default_env())
        .with(fmt_layer_console)
        .with(fmt_layer_file);

    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set global subscriber");

    (guard_file, guard_stdout)
}
