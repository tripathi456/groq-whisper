//! Audio recording module.
//!
//! This module provides functionality for recording audio from the microphone.

use anyhow::{Context, Result};
use byteorder::{LittleEndian, WriteBytesExt};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Sample, SampleFormat};
use hound::{WavSpec, WavWriter};
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tempfile::NamedTempFile;

/// Audio configuration constants
pub const SAMPLE_RATE: u32 = 16000;
pub const CHANNELS: u16 = 1;

/// A struct to hold audio recording state
pub struct AudioRecorder {
    /// Buffer to store recorded audio samples
    samples: Arc<Mutex<Vec<i16>>>,
    /// The active audio stream, if recording
    stream: Option<cpal::Stream>,
}

impl AudioRecorder {
    /// Create a new AudioRecorder instance
    pub fn new() -> Self {
        Self {
            samples: Arc::new(Mutex::new(Vec::new())),
            stream: None,
        }
    }

    /// Start recording audio
    pub fn start_recording(&mut self) -> Result<()> {
        // Clear any previous samples
        self.samples.lock().unwrap().clear();

        // Get default host and input device
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .context("Failed to get default input device")?;

        println!("Using input device: {}", device.name()?);

        // Configure the input stream
        let config = cpal::StreamConfig {
            channels: CHANNELS as cpal::ChannelCount,
            sample_rate: cpal::SampleRate(SAMPLE_RATE),
            buffer_size: cpal::BufferSize::Default,
        };

        // Clone the samples Arc for the data callback
        let samples = Arc::clone(&self.samples);

        // Create and start the stream
        let stream = match device.default_input_config()?.sample_format() {
            SampleFormat::I16 => self.build_stream::<i16>(&device, &config, samples)?,
            SampleFormat::U16 => self.build_stream::<u16>(&device, &config, samples)?,
            SampleFormat::F32 => self.build_stream::<f32>(&device, &config, samples)?,
            format => return Err(anyhow::anyhow!("Unsupported sample format: {:?}", format)),
        };

        stream.play()?;
        self.stream = Some(stream);

        Ok(())
    }

    /// Build an audio stream with the appropriate sample type
    fn build_stream<T>(
        &self,
        device: &cpal::Device,
        config: &cpal::StreamConfig,
        samples: Arc<Mutex<Vec<i16>>>,
    ) -> Result<cpal::Stream>
    where
        T: Sample + Send + 'static,
    {
        let err_fn = |err| eprintln!("An error occurred on the audio stream: {}", err);

        let stream = device.build_input_stream(
            config,
            move |data: &[T], _: &cpal::InputCallbackInfo| {
                let mut sample_lock = samples.lock().unwrap();
                for &sample in data {
                    // Convert to i16 and store
                    sample_lock.push(sample.to_i16());
                }
            },
            err_fn,
            None,
        )?;

        Ok(stream)
    }

    /// Stop recording audio
    pub fn stop_recording(&mut self) {
        // Drop the stream to stop recording
        self.stream = None;
    }

    /// Save the recorded audio to a WAV file
    pub fn save_to_wav(&self, path: &Path) -> Result<()> {
        let samples = self.samples.lock().unwrap();

        let spec = WavSpec {
            channels: CHANNELS,
            sample_rate: SAMPLE_RATE,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut writer = WavWriter::create(path, spec)?;

        for &sample in samples.iter() {
            writer.write_sample(sample)?;
        }

        writer.finalize()?;
        Ok(())
    }

    /// Create a temporary WAV file with the recorded audio
    pub fn save_to_temp_wav(&self) -> Result<NamedTempFile> {
        let temp_file = NamedTempFile::new()?;
        self.save_to_wav(temp_file.path())?;
        Ok(temp_file)
    }

    /// Get the duration of the recording in seconds
    pub fn get_duration_seconds(&self) -> f64 {
        let samples = self.samples.lock().unwrap();
        samples.len() as f64 / (SAMPLE_RATE as f64 * CHANNELS as f64)
    }
}

impl Default for AudioRecorder {
    fn default() -> Self {
        Self::new()
    }
}