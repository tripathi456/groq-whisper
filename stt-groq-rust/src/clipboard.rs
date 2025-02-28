//! Clipboard operations module.

#[cfg(feature = "ui")]
use arboard::Clipboard;
#[cfg(feature = "ui")]
use enigo::{Enigo, Keyboard, Key, Settings, Direction};
use crate::tracing::{info, instrument};
use anyhow::Result;

/// Trait for clipboard operations.
pub trait ClipboardService {
    /// Copy text to the clipboard.
    fn copy_text(&mut self, text: &str) -> Result<()>;
    /// Simulate a paste operation.
    fn paste(&mut self);
}

/// System clipboard implementation.
#[cfg(feature = "ui")]
pub struct SystemClipboard;

#[cfg(feature = "ui")]
impl ClipboardService for SystemClipboard {
    fn copy_text(&mut self, text: &str) -> Result<()> {
        let mut clipboard = Clipboard::new()?;
        clipboard.set_text(text)?;
        info!("Text copied to clipboard: {}", text);
        Ok(())
    }

    fn paste(&mut self) {
        let mut enigo = Enigo::new(&Settings::default()).unwrap();
        enigo.key(Key::Control, Direction::Press);
        enigo.key(Key::Layout('v'), Direction::Press);
        enigo.key(Key::Layout('v'), Direction::Release);
        enigo.key(Key::Control, Direction::Release);
        info!("Paste operation simulated");
    }
}

/// Dummy implementation for environments without UI support
#[cfg(not(feature = "ui"))]
pub struct SystemClipboard;

#[cfg(not(feature = "ui"))]
impl ClipboardService for SystemClipboard {
    fn copy_text(&mut self, text: &str) -> Result<()> {
        info!("Clipboard copy operation simulated: {}", text);
        Ok(())
    }
    fn paste(&mut self) {
        info!("Clipboard paste operation simulated");
    }
}

/// Copy text to the clipboard and perform a paste operation.
#[instrument(skip(text))]
pub fn copy_and_paste(text: &str) -> Result<()> {
    let mut clipboard = SystemClipboard;
    clipboard.copy_text(text)?;
    clipboard.paste();
    info!("Copy and paste operation completed");
    Ok(())
}