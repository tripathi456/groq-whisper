//! Clipboard operations module.
//!
//! This module provides functionality for copying text to the clipboard and simulating keyboard shortcuts.

use anyhow::Result;
use arboard::Clipboard;
use enigo::{Enigo, Keyboard, Key, Settings, Direction};
use crate::tracing::{debug, info, instrument}; // Use our local tracing module

/// Copy text to the system clipboard
pub fn copy_to_clipboard(text: &str) -> Result<()> {
    let mut clipboard = Clipboard::new()?;
    clipboard.set_text(text)?;
    Ok(())
}

/// Simulate system paste operation
pub fn system_paste() {
    let settings = Settings::default();
    let mut enigo = Enigo::new(&settings).unwrap();
    
    enigo.key(Key::Control, Direction::Press);
    enigo.key(Key::Unicode('v'), Direction::Click);
    enigo.key(Key::Control, Direction::Release);
}

/// Copy text to clipboard and paste it
#[instrument(skip(text), fields(text_len = text.len()))]
pub fn copy_and_paste(text: &str) -> Result<()> {
    debug!("Starting copy and paste operation");
    copy_to_clipboard(text)?;
    system_paste();
    info!("Copy and paste operation completed");
    Ok(())
}