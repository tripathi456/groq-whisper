//! Keyboard handling module.
//!
//! This module provides functionality for detecting keyboard events.

#[cfg(feature = "keyboard")]
use device_query::{DeviceQuery, DeviceState, Keycode};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use crate::tracing::{info, instrument};

/// Threshold for double-tap detection (in seconds)
pub const ALT_THRESHOLD: f64 = 0.5;

/// Keyboard event handler for detecting Alt key double-taps
pub struct KeyboardHandler {
    /// Device state for querying keyboard
    #[cfg(feature = "keyboard")]
    device_state: DeviceState,
    /// Last time the Alt key was pressed
    last_alt_time: Arc<Mutex<Instant>>,
    /// Flag to control the background thread
    running: Arc<Mutex<bool>>,
    /// Callback function to execute on double-tap
    on_double_tap: Arc<Mutex<Box<dyn Fn() + Send + 'static>>>,
}

impl KeyboardHandler {
    /// Create a new KeyboardHandler instance
    pub fn new<F>(on_double_tap: F) -> Self
    where
        F: Fn() + Send + 'static,
    {
        Self {
            #[cfg(feature = "keyboard")]
            device_state: DeviceState::new(),
            last_alt_time: Arc::new(Mutex::new(Instant::now() - Duration::from_secs(10))),
            running: Arc::new(Mutex::new(true)),
            on_double_tap: Arc::new(Mutex::new(Box::new(on_double_tap))),
        }
    }

    /// Start monitoring keyboard events in a background thread
    pub fn start_monitoring(&self) -> thread::JoinHandle<()> {
        #[cfg(feature = "keyboard")]
        {
            let device_state = DeviceState::new();
            let last_alt_time = Arc::clone(&self.last_alt_time);
            let running = Arc::clone(&self.running);
            let on_double_tap = Arc::clone(&self.on_double_tap);

            thread::spawn(move || {
                let mut was_alt_pressed = false;

                while *running.lock().unwrap() {
                    let keys = device_state.get_keys();
                    let alt_pressed = keys.contains(&Keycode::LAlt) || keys.contains(&Keycode::RAlt);

                    // Detect Alt key press (transition from not pressed to pressed)
                    if alt_pressed && !was_alt_pressed {
                        let mut last_time = last_alt_time.lock().unwrap();
                        let current_time = Instant::now();
                        let elapsed = current_time.duration_since(*last_time).as_secs_f64();

                        // Check if this is a double-tap
                        if elapsed < ALT_THRESHOLD {
                            // Execute the callback
                            let callback = on_double_tap.lock().unwrap();
                            callback();
                            // Reset the timer
                            *last_time = current_time - Duration::from_secs(10);
                        } else {
                            // Update the last press time
                            *last_time = current_time;
                        }
                    }

                    was_alt_pressed = alt_pressed;
                    thread::sleep(Duration::from_millis(10));
                }
            })
        }

        #[cfg(not(feature = "keyboard"))]
        {
            let running = Arc::clone(&self.running);
            
            thread::spawn(move || {
                info!("Keyboard monitoring not available (compiled without keyboard support)");
                
                // Keep the thread alive until stopped
                while *running.lock().unwrap() {
                    thread::sleep(Duration::from_secs(1));
                }
            })
        }
    }

    /// Stop monitoring keyboard events
    #[instrument(skip(self))]
    pub fn stop_monitoring(&self) {
        let mut running = self.running.lock().unwrap();
        *running = false;
        info!("Stopping keyboard monitoring");
    }
}

impl Drop for KeyboardHandler {
    fn drop(&mut self) {
        self.stop_monitoring();
    }
}