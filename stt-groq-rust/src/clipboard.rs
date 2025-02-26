//! Clipboard operations module.
//!
//! This module provides functionality for copying text to the clipboard and simulating keyboard shortcuts.

use anyhow::Result;
use arboard::Clipboard;
use enigo::{Enigo, Key, KeyboardControllable};
use std::thread;
use std::time::Duration;

/// Copy text to the system clipboard
pub fn copy_to_clipboard(text: &str) -> Result<()> {
    let mut clipboard = Clipboard::new()?;
    clipboard.set_text(text)?;
    Ok(())
}

/// Simulate pressing Ctrl+V to paste from clipboard
pub fn paste_from_clipboard() -> Result<()> {
    let mut enigo = Enigo::new();
    
    // Small delay to ensure the application is ready
    thread::sleep(Duration::from_millis(100));
    
    // Press Ctrl+V
    enigo.key_down(Key::Control);
    enigo.key_click(Key::Layout('v'));
    enigo.key_up(Key::Control);
    
    Ok(())
}

/// Copy text to clipboard and paste it
#[instrument(skip(text), fields(text_len = text.len()))]
pub fn copy_and_paste(text: &str) -> Result<()> {
    debug!("Starting copy and paste operation");
    copy_to_clipboard(text)?;
    paste_from_clipboard()?;
    info!("Copy and paste operation completed");
    Ok(())
}