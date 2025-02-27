use anyhow::Result;
mod audio;
mod clipboard;
mod groq_client;
mod keyboard;
mod model;
mod notifications;
mod tracing;

use std::sync::Arc;
use std::thread;
use audio::AudioRecorder;
use dotenv::dotenv;
use groq_client::GroqClient;
use keyboard::KeyboardHandler;
use model::ModelSelector;
use notifications::show_notification_default;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

const MIN_RECORDING_DURATION: f64 = 4.0;

struct AppState {
    recorder: Arc<Mutex<AudioRecorder>>,
    recording: Arc<Mutex<bool>>,
    recording_start_time: Arc<Mutex<Option<Instant>>>,
    groq_client: Arc<GroqClient>,
    model_selector: Arc<ModelSelector>,
}

impl AppState {
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

    /// Toggle the recording state.
    async fn toggle_recording(&self) {
        let mut recording = self.recording.lock().await;
        if !*recording {
            // Start recording.
            *recording = true;
            let mut recorder = self.recorder.lock().await;
            if let Err(e) = recorder.start_recording() {
                error!("Failed to start recording: {}", e);
                *recording = false;
                return;
            }
            *self.recording_start_time.lock().await = Some(Instant::now());
            info!("main.rs - Recording started");
            show_notification_default("Recording Started", "Audio recording has started.");
        } else {
            // Stop recording.
            *recording = false;
            info!("Recording stopped");
            let app_state_clone = self.clone();
            tokio::spawn(async move {
                app_state_clone.process_recording();
            });
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
                        error!("Failed to save audio: {}", e);
                    }
                }
            }
        });
    }
}

// Implement Clone for AppState for task spawning.
impl Clone for AppState {
    fn clone(&self) -> Self {
        Self {
            recorder: Arc::clone(&self.recorder),
            recording: Arc::clone(&self.recording),
            recording_start_time: Arc::clone(&self.recording_start_time),
            groq_client: Arc::clone(&self.groq_client),
            model_selector: Arc::clone(&self.model_selector),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    let _guard = tracing::init_tracing();
    info!("Starting Groq Whisper STT application");

    let app_state = Arc::new(AppState::new()?);

    // Capture the runtime handle so the keyboard callback can spawn tasks.
    let rt_handle = tokio::runtime::Handle::current();

    // Create a keyboard handler with a callback to toggle recording.
    let app_state_clone = Arc::clone(&app_state);
    let mut keyboard_handler = KeyboardHandler::new(move || {
        let app_state_clone2 = Arc::clone(&app_state_clone);
        let rt_handle_clone = rt_handle.clone();
        rt_handle_clone.spawn(async move {
            app_state_clone2.toggle_recording().await;
        });
    });
    keyboard_handler.start_listening()?;

    info!("Double-tap the Alt key to toggle recording on/off");
    info!("Press Ctrl+C to exit");

    // Wait for Ctrl+C signal for graceful shutdown.
    tokio::signal::ctrl_c().await?;
    info!("Shutdown signal received, exiting.");
    std::process::exit(0);
}