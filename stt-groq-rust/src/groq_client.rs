use anyhow::{Context, Result};
use reqwest::{Client};
use reqwest::blocking;
use reqwest::blocking::multipart::{Form, Part};
use serde::{Deserialize, Serialize};
use std::env;
use std::path::Path;
use std::fs::File;
use std::io::Read;
use crate::tracing::{debug, error, info, instrument};

const GROQ_API_BASE_URL: &str = "https://api.groq.com/openai/v1";

pub struct GroqClient {
    api_key: String,
    client: Client, // retained for potential async usage
}

#[derive(Debug, Serialize)]
pub struct TranscriptionRequest {
    pub model: String,
    pub prompt: Option<String>,
    pub response_format: String,
    pub language: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TranscriptionResponse {
    pub text: String,
}

impl GroqClient {
    pub fn new() -> Result<Self> {
        let api_key = env::var("GROQ_API_KEY")
            .context("GROQ_API_KEY environment variable not set")?;
        Ok(Self {
            api_key,
            client: Client::new(),
        })
    }
    
    /// Synchronous transcription method.
    #[instrument(skip(self, audio_path), fields(audio_path = %audio_path.display()))]
    pub fn transcribe_audio_sync(
        &self,
        audio_path: &Path,
        model: &str,
        prompt: Option<&str>,
        language: Option<&str>,
    ) -> Result<String> {
        // Check if the file exists
        if !audio_path.exists() {
            return Err(anyhow::anyhow!("Audio file does not exist: {}", audio_path.display()));
        }
        
        // Get file metadata to log the size
        let metadata = std::fs::metadata(audio_path)?;
        println!("Audio file size: {} bytes", metadata.len());
        
        // Read the audio file
        let mut file = File::open(audio_path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .with_context(|| format!("Failed to read audio file: {}", audio_path.display()))?;
        
        debug!(bytes = buffer.len(), "Read audio file");
        println!("Audio buffer size: {} bytes", buffer.len());

        // Create the multipart file part.
        let file_name = audio_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("audio.wav");
        let file_part = Part::bytes(buffer)
            .file_name(file_name.to_string())
            .mime_str("audio/wav")?;
        
        let mut form = Form::new().part("file", file_part);
        form = form.text("model", model.to_string());
        form = form.text("response_format", "text".to_string());
        if let Some(prompt_text) = prompt {
            form = form.text("prompt", prompt_text.to_string());
            debug!(prompt = %prompt_text, "Added prompt to sync request");
        }
        if let Some(lang) = language {
            form = form.text("language", lang.to_string());
            debug!(language = %lang, "Added language to sync request");
        }
        
        info!("Sending synchronous transcription request to Groq API");
        let url = format!("{}/audio/transcriptions", GROQ_API_BASE_URL);
        let blocking_client = blocking::Client::new();
        let response = blocking_client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .multipart(form)
            .send()
            .with_context(|| format!("Failed to send synchronous transcription request to {}", url))?;
        
        debug!(status = ?response.status(), "Received response from synchronous transcription request");
        
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().unwrap_or_default();
            error!(status = %status, error = %error_text, "Synchronous API request failed");
            return Err(anyhow::anyhow!("API error: {}", error_text));
        }
        
        let transcription = response.text()
            .with_context(|| "Failed to parse synchronous transcription response")?;
        
        // Log the full transcription for debugging.
        info!("Received synchronous transcription ({} chars): {}", transcription.len(), transcription);
        Ok(transcription)
    }
}
