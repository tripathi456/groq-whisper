use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use rdev::{listen, Event, EventType, Key};
use tracing::{debug, error};

pub const ALT_THRESHOLD: f64 = 0.5;

pub struct KeyboardHandler {
    last_alt_time: Arc<Mutex<Instant>>,
    on_double_tap: Arc<Mutex<Box<dyn Fn() + Send + 'static>>>,
    // Handle for the listener thread.
    listener_handle: Option<thread::JoinHandle<()>>,
    // Shutdown flag.
    running: Arc<Mutex<bool>>,
}

impl KeyboardHandler {
    pub fn new<F>(on_double_tap: F) -> Self
    where
        F: Fn() + Send + 'static,
    {
        Self {
            last_alt_time: Arc::new(Mutex::new(Instant::now() - Duration::from_secs(10))),
            on_double_tap: Arc::new(Mutex::new(Box::new(on_double_tap))),
            listener_handle: None,
            running: Arc::new(Mutex::new(true)),
        }
    }

    /// Start listening for keyboard events using rdev.
    pub fn start_listening(&mut self) -> Result<(), std::io::Error> {
        let last_alt_time = Arc::clone(&self.last_alt_time);
        let on_double_tap = Arc::clone(&self.on_double_tap);
        let running = Arc::clone(&self.running);

        let handle = thread::spawn(move || {
            let callback = move |event: Event| {
                // Check shutdown flag.
                if !*running.lock().unwrap() {
                    return;
                }
                if let EventType::KeyPress(key) = event.event_type {
                    // Check for Alt or AltGr key.
                    if key == Key::Alt || key == Key::AltGr {
                        let mut last_time = last_alt_time.lock().unwrap();
                        let now = Instant::now();
                        let elapsed = now.duration_since(*last_time).as_secs_f64();
                        if elapsed < ALT_THRESHOLD {
                            if let Ok(callback) = on_double_tap.lock() {
                                callback();
                            }
                        }
                        *last_time = now;
                    }
                }
            };

            if let Err(error) = listen(callback) {
                error!("Keyboard listener error: {:?}", error);
            }
        });
        self.listener_handle = Some(handle);
        Ok(())
    }

    /// Stop the keyboard listener.
    pub fn stop_monitoring(&self) {
        let mut running = self.running.lock().unwrap();
        *running = false;
    }
}

impl Drop for KeyboardHandler {
    fn drop(&mut self) {
        self.stop_monitoring();
        if let Some(handle) = self.listener_handle.take() {
            let _ = handle.join();
        }
    }
}
