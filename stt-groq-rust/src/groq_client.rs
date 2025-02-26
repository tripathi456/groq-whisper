//! Groq API client module.
//!
//! This module provides functionality to interact with the Groq API for audio transcription.

use anyhow::{Context, Result};
use reqwest::blocking::multipart::{Form, Part};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs::File;
use std::io::Read;
use std::path::Path;

/// Groq API base URL
const GROQ_API_BASE_URL: &str = "https://api.groq.com/openai/v1";

/// Groq API client for audio transcription
pub struct GroqClient {
    api_key: String,
    client: Client,
}

/// Transcription request parameters
#[derive(Debug, Serialize)]
pub struct TranscriptionRequest {
    pub model: String,
    pub prompt: Option<String>,
    pub response_format: String,
    pub language: Option<String>,
}

/// Transcription response
#[derive(Debug, Deserialize)]
pub struct TranscriptionResponse {
    pub text: String,
}

impl GroqClient {
    /// Create a new GroqClient instance using the API key from environment variables
    pub fn new() -> Result<Self> {
        let api_key = env::var("GROQ_API_KEY")
            .context("GROQ_API_KEY environment variable not set")?;
        
        Ok(Self {
            api_key,
            client: Client::new(),
        })
    }

    /// Create a new GroqClient instance with a provided API key
    pub fn with_api_key(api_key: String) -> Self {
        Self {
            api_key,
            client: Client::new(),
        }
    }

    /// Transcribe an audio file using the Groq Whisper API
    #[instrument(skip(self, audio_path), fields(audio_path = %audio_path.display(), model = %model), err)]
    pub fn transcribe_audio(
        &self,
        audio_path: &Path,
        model: &str,
        prompt: Option<&str>,
        language: Option<&str>,
    ) -> Result<String> {
        // Read the audio file
        let mut file = File::open(audio_path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;
        
        debug!(bytes = buffer.len(), "Read audio file");

        // Create the file part
        let file_name = audio_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("audio.wav");
        
        let file_part = Part::bytes(buffer)
            .file_name(file_name.to_string())
            .mime_str("audio/wav")?;

        // Build the multipart form
        let mut form = Form::new().part("file", file_part);

        // Add other parameters
        form = form.text("model", model.to_string());
        form = form.text("response_format", "text");
        
        if let Some(prompt_text) = prompt {
            form = form.text("prompt", prompt_text.to_string());
            debug!(prompt = %prompt_text, "Added prompt to request");
        }
        
        if let Some(lang) = language {
            form = form.text("language", lang.to_string());
            debug!(language = %lang, "Added language to request");
        }

        info!("Sending transcription request to Groq API");
        
        // Make the API request
        let response = self.client
            .post(&format!("{}/audio/transcriptions", GROQ_API_BASE_URL))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .multipart(form)
            .send()?;

        // Check for errors
        if !response.status().is_success() {
            let error_text = response.text()?;
            error!(status = %response.status(), error = %error_text, "API request failed");
            return Err(anyhow::anyhow!("API error: {}", error_text));
        }

        // Parse the response
        let transcription = response.text()?;
        info!(chars = transcription.len(), "Received transcription from API");
        Ok(transcription)
    }
}