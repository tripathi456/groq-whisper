//! Keyboard handling module.

use std::sync::{Arc, Mutex};
#[cfg(feature = "ui")]
use rdev::{listen, Event, EventType, Key};
use tracing::{error};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use anyhow::Result;

/// KeyboardHandler monitors for double-tap of the Alt key.
pub struct KeyboardHandler {
    on_double_tap: Arc<dyn Fn() + Send + Sync>,
    listener_handle: Option<JoinHandle<()>>,
    running: Arc<Mutex<bool>>,
}

impl KeyboardHandler {
    /// Create a new KeyboardHandler with a callback for double-tap.
    pub fn new<F>(on_double_tap: F) -> Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        Self {
            on_double_tap: Arc::new(on_double_tap),
            listener_handle: None,
            running: Arc::new(Mutex::new(true)),
        }
    }

    /// Start listening for keyboard events.
    pub fn start_listening(&mut self) -> Result<()> {
        let on_double_tap = Arc::clone(&self.on_double_tap);
        let running = Arc::clone(&self.running);

        #[cfg(feature = "ui")]
        let handle = thread::spawn(move || {
            let callback = move |event: Event| {
                // Check shutdown flag.
                if !*running.lock().unwrap() {
                    return;
                }

                // Track Alt key double-tap.
                static mut LAST_ALT_PRESS: Option<Instant> = None;
                if let EventType::KeyPress(Key::Alt) = event.event_type {
                    unsafe {
                        if let Some(last_press) = LAST_ALT_PRESS {
                            if last_press.elapsed() < Duration::from_millis(500) {
                                on_double_tap();
                            }
                            LAST_ALT_PRESS = Some(Instant::now());
                        } else {
                            LAST_ALT_PRESS = Some(Instant::now());
                        }
                    }
                }
            };

            if let Err(error) = listen(callback) {
                error!("Keyboard listener error: {:?}", error);
            }
        });

        #[cfg(not(feature = "ui"))]
        let handle = thread::spawn(move || {
            // Dummy implementation that just logs a message
            error!("Keyboard monitoring is not available (ui feature not enabled)");
            // Keep the thread alive but do nothing
            loop {
                if !*running.lock().unwrap() {
                    break;
                }
                thread::sleep(Duration::from_secs(1));
            }
        });
        
        self.listener_handle = Some(handle);
        Ok(())
    }

    /// Stop listening for keyboard events.
    pub fn stop_listening(&mut self) {
        *self.running.lock().unwrap() = false;
    }
}

impl Drop for KeyboardHandler {
    fn drop(&mut self) {
        self.stop_listening();
        if let Some(handle) = self.listener_handle.take() {
            let _ = handle.join();
        }
    }
}