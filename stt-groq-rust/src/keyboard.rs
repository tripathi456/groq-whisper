//! Keyboard handling module.
//!
//! This module provides functionality for detecting keyboard events.

use device_query::{DeviceState, DeviceQuery, Keycode as Key};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// Threshold for double-tap detection (in seconds)
pub const ALT_THRESHOLD: f64 = 0.5;

/// Keyboard event handler for detecting Alt key double-taps
pub struct KeyboardHandler {
    /// Thread handle for querying keyboard
    device_query_thread: Option<thread::JoinHandle<()>>,
    /// Last time the Alt key was pressed
    last_alt_time: Arc<Mutex<Instant>>,
    /// Flag to control the background thread
    running: Arc<Mutex<bool>>,
    /// Callback function to execute on double-tap
    on_double_tap: Arc<Mutex<Box<dyn Fn() + Send + 'static>>>,
}

impl std::fmt::Debug for KeyboardHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KeyboardHandler")
            .field("device_query_thread", &self.device_query_thread)
            .field("last_alt_time", &self.last_alt_time)
            .field("running", &self.running)
            .field("on_double_tap", &"Box<dyn Fn() + Send + 'static>")
            .finish()
    }
}

impl KeyboardHandler {
    /// Create a new KeyboardHandler instance
    pub fn new<F>(on_double_tap: F) -> Self
    where
        F: Fn() + Send + 'static,
    {
        Self {
            device_query_thread: None,
            last_alt_time: Arc::new(Mutex::new(Instant::now() - Duration::from_secs(10))),
            running: Arc::new(Mutex::new(true)),
            on_double_tap: Arc::new(Mutex::new(Box::new(on_double_tap))),
        }
    }

    /// Start listening for keyboard events in a background thread
    pub fn start_listening(&mut self) -> Result<(), anyhow::Error> {
        let last_alt_time = Arc::clone(&self.last_alt_time);
        let running = Arc::clone(&self.running);
        let on_double_tap = Arc::clone(&self.on_double_tap);

        let handle = thread::spawn(move || {
            // Create DeviceState inside the thread to avoid Send/Sync issues
            let device_state = DeviceState::new();
            
            while *running.lock().unwrap() {
                let keys = device_state.get_keys();
                let alt_pressed = keys.contains(&Key::LAlt) || keys.contains(&Key::RAlt);

                if alt_pressed {
                    let mut last_time = last_alt_time.lock().unwrap();
                    let now = Instant::now();
                    let elapsed = now.duration_since(*last_time).as_secs_f64();

                    if elapsed < ALT_THRESHOLD {
                        // Double-tap detected
                        if let Ok(callback) = on_double_tap.lock() {
                            callback();
                        }
                    }

                    *last_time = now;
                }

                thread::sleep(Duration::from_millis(50));
            }
        });

        self.device_query_thread = Some(handle);
        Ok(())
    }

    /// Stop monitoring keyboard events
    pub fn stop_monitoring(&self) {
        let mut running = self.running.lock().unwrap();
        *running = false;
    }
}

impl Drop for KeyboardHandler {
    fn drop(&mut self) {
        self.stop_monitoring();
    }
}