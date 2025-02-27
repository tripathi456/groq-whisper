//! Speech-to-Text application using Groq's Whisper API.
//!
//! This application records audio when triggered by a double-tap of the Alt key,
//! transcribes the audio using Groq's Whisper API, and pastes the transcription
//! to the clipboard.

mod audio;
mod clipboard;
mod groq_client;
mod keyboard;
mod model;
mod notifications;

use anyhow::{Context, Result};
use audio::AudioRecorder;
use dotenv::dotenv;
use groq_client::GroqClient;
use keyboard::KeyboardHandler;
use model::ModelSelector;
use notifications::show_notification_default;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// Minimum recording duration in seconds
const MIN_RECORDING_DURATION: f64 = 5.0;

/// Application state
struct AppState {
    /// Audio recorder
    recorder: Arc<Mutex<AudioRecorder>>,
    /// Flag indicating if recording is in progress
    recording: Arc<Mutex<bool>>,
    /// Timestamp when recording started
    recording_start_time: Arc<Mutex<Option<Instant>>>,
    /// Groq API client
    groq_client: Arc<GroqClient>,
    /// Model selector
    model_selector: Arc<ModelSelector>,
}

impl AppState {
    /// Create a new AppState instance
    fn new() -> Result<Self> {
        let groq_client = Arc::new(GroqClient::new()?);
        
        Ok(Self {
            recorder: Arc::new(Mutex::new(AudioRecorder::new())),
            recording: Arc::new(Mutex::new(false)),
            recording_start_time: Arc::new(Mutex::new(None)),
            groq_client,
            model_selector: Arc::new(ModelSelector::new()),
        })
    }

    /// Toggle recording state
    fn toggle_recording(&self) {
        let mut recording = self.recording.lock().unwrap();
        
        if !*recording {
            // Start recording
            *recording = true;
            let mut recorder = self.recorder.lock().unwrap();
            if let Err(e) = recorder.start_recording() {
                eprintln!("Failed to start recording: {}", e);
                *recording = false;
                return;
            }
            
            *self.recording_start_time.lock().unwrap() = Some(Instant::now());
            println!("Recording started.");
            show_notification_default("Recording Started", "Audio recording has started.");
        } else {
            // Stop recording
            *recording = false;
            println!("Recording stopped.");
            
            // Process the recording in a separate thread
            self.process_recording();
        }
    }

    /// Process the recording (stop recording, transcribe, and paste)
    fn process_recording(&self) {
        let recorder_clone = Arc::clone(&self.recorder);
        let recording_start_time_clone = Arc::clone(&self.recording_start_time);
        let groq_client_clone = Arc::clone(&self.groq_client);
        let model_selector_clone = Arc::clone(&self.model_selector);
        
        thread::spawn(move || {
            // Stop the recording
            let mut recorder = recorder_clone.lock().unwrap();
            recorder.stop_recording();
            
            // Add a short delay to ensure all audio data is processed
            drop(recorder);  // Release the lock before sleeping
            thread::sleep(Duration::from_millis(500));
            let mut recorder = recorder_clone.lock().unwrap();
            
            // Check recording duration
            let start_time = *recording_start_time_clone.lock().unwrap();
            if let Some(start) = start_time {
                let duration = start.elapsed().as_secs_f64();
                
                if duration < MIN_RECORDING_DURATION {
                    println!("Recording duration was less than {} seconds. Skipping transcription.", MIN_RECORDING_DURATION);
                    return;
                }
                
                // Save the recording to a temporary file
                match recorder.save_to_temp_wav() {
                    Ok(temp_file) => {
                        println!("Transcribing...");
                        
                        // Select the next model
                        let model = model_selector_clone.get_next_model();
                        println!("Using model: {}", model);
                        
                        // Transcribe the audio
                        match groq_client_clone.transcribe_audio(
                            temp_file.path(),
                            &model,
                            Some("The audio is by a programmer discussing programming issues"),
                            Some("en"),
                        ) {
                            Ok(transcription) => {
                                println!("\nTranscription:");
                                println!("{}", transcription);
                                
                                // Copy to clipboard and paste
                                if let Err(e) = clipboard::copy_and_paste(&transcription) {
                                    eprintln!("Failed to copy/paste transcription: {}", e);
                                } else {
                                    println!("Transcription copied to clipboard.");
                                }
                            }
                            Err(e) => {
                                eprintln!("Transcription failed: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to save audio: {}", e);
                    }
                }
            }
        });
    }
}

fn main() -> Result<()> {
    // Load environment variables from .env file if present
    dotenv().ok();
    
    // Create application state
    let app_state = Arc::new(AppState::new()?);
    
    // Create keyboard handler with callback to toggle recording
    let app_state_clone = Arc::clone(&app_state);
    let keyboard_handler = KeyboardHandler::new(move || {
        app_state_clone.toggle_recording();
    });
    
    // Start keyboard monitoring
    let _keyboard_thread = keyboard_handler.start_monitoring();
    
    info!("Double-tap the Alt key (press Alt twice quickly) to toggle recording on/off");
    info!("Press Ctrl+C to exit");
    
    // Keep the main thread running
    loop {
        thread::sleep(Duration::from_secs(1));
    }
}