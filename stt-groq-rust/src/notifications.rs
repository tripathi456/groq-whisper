//! Notifications module for desktop notifications.
//!
//! This module provides functionality to display desktop notifications.

#[cfg(feature = "ui")]
use notify_rust::{Notification, Timeout};
use crate::tracing::{debug, error, instrument}; 

/// Show a desktop notification with the given title and message.
///
/// # Arguments
///
/// * `title` - The title of the notification
/// * `message` - The message body of the notification
/// * `timeout_seconds` - How long the notification should be displayed (in seconds)
#[cfg(feature = "ui")]
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

#[cfg(not(feature = "ui"))]
pub fn show_notification(title: &str, message: &str, timeout_seconds: u32) {
    debug!("Notification (simulated): {} - {} (timeout: {}s)", title, message, timeout_seconds);
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