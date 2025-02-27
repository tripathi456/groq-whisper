//! Notifications module for desktop notifications.
//!
//! This module provides functionality to display desktop notifications.

use notify_rust::{Notification, Timeout};
use std::time::Duration;
use crate::tracing::{debug, error, instrument}; // Add this line to import from our tracing module

/// Show a desktop notification with the given title and message.
///
/// # Arguments
///
/// * `title` - The title of the notification
/// * `message` - The message body of the notification
/// * `timeout_seconds` - How long the notification should be displayed (in seconds)
pub fn show_notification(title: &str, message: &str, timeout_seconds: u32) {
    if let Err(e) = Notification::new()
        .summary(title)
        .body(message)
        .timeout(Timeout::Milliseconds(timeout_seconds * 1000))
        .show()
    {
        error!("Failed to show notification: {}", e);
    }
}

/// Show a desktop notification with the default timeout of 3 seconds.
///
/// # Arguments
///
/// * `title` - The title of the notification
/// * `message` - The message body of the notification
#[instrument]
pub fn show_notification_default(title: &str, message: &str) {
    debug!("Showing notification with default timeout");
    show_notification(title, message, 3);
}