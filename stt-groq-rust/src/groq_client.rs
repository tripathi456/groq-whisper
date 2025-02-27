use anyhow::{Context, Result};
use reqwest::multipart::{Form, Part};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use crate::tracing::{debug, error, info, instrument};

const GROQ_API_BASE_URL: &str = "https://api.groq.com/openai/v1";

pub struct GroqClient {
    api_key: String,
    client: Client,
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

    #[instrument(skip(self, audio_path), fields(audio_path = %audio_path.display()))]
    pub async fn transcribe_audio(
        &self,
        audio_path: &Path,
        model: &str,
        prompt: Option<&str>,
        language: Option<&str>,
    ) -> Result<String> {
        // Read the audio file asynchronously.
        let mut file = File::open(audio_path).await
            .with_context(|| format!("Failed to open audio file: {}", audio_path.display()))?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).await
            .with_context(|| format!("Failed to read audio file: {}", audio_path.display()))?;
        
        debug!(bytes = buffer.len(), "Read audio file");

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
        let url = format!("{}/audio/transcriptions", GROQ_API_BASE_URL);
        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .multipart(form)
            .send()
            .await
            .with_context(|| format!("Failed to send transcription request to {}", url))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            error!(status = %status, error = %error_text, "API request failed");
            return Err(anyhow::anyhow!("API error: {}", error_text));
        }

        let transcription = response.text().await
            .with_context(|| "Failed to parse transcription response")?;
        info!(chars = transcription.len(), "Received transcription from API");
        Ok(transcription)
    }
}
