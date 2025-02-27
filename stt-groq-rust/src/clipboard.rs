use anyhow::Result;
use arboard::Clipboard;
use enigo::{Enigo, Keyboard, Key, Settings, Direction};
use crate::tracing::{debug, info, instrument};

/// A trait to abstract clipboard operations.
pub trait ClipboardService: Send + Sync {
    fn copy_text(&mut self, text: &str) -> Result<()>;
    fn paste(&mut self);
}

/// The production implementation using arboard and enigo.
pub struct SystemClipboard;

impl ClipboardService for SystemClipboard {
    fn copy_text(&mut self, text: &str) -> Result<()> {
        let mut clipboard = Clipboard::new()?;
        clipboard.set_text(text)?;
        Ok(())
    }
    fn paste(&mut self) {
        let settings = Settings::default();
        let mut enigo = Enigo::new(&settings).unwrap();
        let _ = enigo.key(Key::Control, Direction::Press);
        let _ = enigo.key(Key::Unicode('v'), Direction::Click);
        let _ = enigo.key(Key::Control, Direction::Release);
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
