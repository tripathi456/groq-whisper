//! Audio recording module.
//!
//! This module provides functionality for recording audio from the microphone.
//! When the "audio" feature is enabled, it uses cpal for real audio recording.
//! When the "audio" feature is disabled, it provides a mock implementation.

use anyhow::{Context, Result};
use hound::{WavSpec, WavWriter};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use tempfile::{Builder, NamedTempFile};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tracing::{debug, error, info, warn};

/// Audio configuration constants.
pub const SAMPLE_RATE: u32 = 16000;
pub const CHANNELS: u16 = 1;

// Import cpal and related types only when the "audio" feature is enabled
#[cfg(feature = "audio")]
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
#[cfg(feature = "audio")]
use cpal::{Sample, SampleFormat, SizedSample};
#[cfg(feature = "audio")]
use num_traits::cast::ToPrimitive;

/// AudioRecorder holds the state for audio recording.
pub struct AudioRecorder {
    /// Tokio channel sender for audio samples.
    samples_tx: Option<UnboundedSender<i16>>,
    /// Tokio channel receiver for audio samples.
    samples_rx: Option<UnboundedReceiver<i16>>,
    /// The active audio stream.
    #[allow(dead_code)]
    stream: Option<StreamWrapper>,
    /// Flag to indicate if recording is active.
    recording_flag: Arc<AtomicBool>,
    /// Collected audio samples
    samples: Arc<Mutex<Vec<i16>>>,
}

/// Wrapper for the audio stream
#[cfg(feature = "audio")]
struct StreamWrapper(Arc<Mutex<cpal::Stream>>);

#[cfg(feature = "audio")]
unsafe impl Send for StreamWrapper {}

#[cfg(feature = "audio")]
impl StreamWrapper {
    fn new(stream: cpal::Stream) -> Self {
        Self(Arc::new(Mutex::new(stream)))
    }
}

/// Mock stream wrapper when audio feature is disabled
#[cfg(not(feature = "audio"))]
struct StreamWrapper;

#[cfg(not(feature = "audio"))]
impl StreamWrapper {
    fn new(_: ()) -> Self {
        Self
    }
}

impl AudioRecorder {
    /// Create a new AudioRecorder instance.
    pub fn new() -> Self {
        Self {
            samples_tx: None,
            samples_rx: None,
            stream: None,
            recording_flag: Arc::new(AtomicBool::new(false)),
            samples: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Start recording audio.
    pub fn start_recording(&mut self) -> Result<()> {
        // Create an unbounded channel so that high-rate audio samples aren't dropped.
        let (tx, rx) = unbounded_channel();
        self.samples_tx = Some(tx);
        self.samples_rx = Some(rx);
        
        // Set the recording flag to true.
        self.recording_flag.store(true, Ordering::Relaxed);
        
        // Clear any previous samples
        {
            let mut samples = self.samples.lock().unwrap();
            samples.clear();
        }

        // Real audio recording implementation when the "audio" feature is enabled
        #[cfg(feature = "audio")]
        {
            let host = cpal::default_host();
            let device = host
                .default_input_device()
                .context("Failed to get default input device")?;
            info!("Using input device: {}", device.name()?);

            let config = cpal::StreamConfig {
                channels: CHANNELS as cpal::ChannelCount,
                sample_rate: cpal::SampleRate(SAMPLE_RATE),
                buffer_size: cpal::BufferSize::Default,
            };

            // Clone the sender and the recording flag to move into the audio callback.
            let tx_clone = self.samples_tx.as_ref().unwrap().clone();
            let recording_flag_clone = self.recording_flag.clone();

            // Build the input stream for the device's sample format.
            let stream = match device.default_input_config()?.sample_format() {
                SampleFormat::I16 => {
                    Self::build_stream::<i16>(&device, &config, tx_clone, recording_flag_clone)?
                }
                SampleFormat::U16 => {
                    Self::build_stream::<u16>(&device, &config, tx_clone, recording_flag_clone)?
                }
                SampleFormat::F32 => {
                    Self::build_stream::<f32>(&device, &config, tx_clone, recording_flag_clone)?
                }
                format => return Err(anyhow::anyhow!("Unsupported sample format: {:?}", format)),
            };

            stream.play()?;
            self.stream = Some(StreamWrapper::new(stream));
        }
        
        // Mock implementation when the "audio" feature is disabled
        #[cfg(not(feature = "audio"))]
        {
            info!("Audio recording not available (audio feature not enabled)");
            self.stream = Some(StreamWrapper::new(()));
            
            // Generate some mock audio data
            if let Some(tx) = &self.samples_tx {
                for i in 0..SAMPLE_RATE {
                    // Simple sine wave
                    let t = i as f32 / SAMPLE_RATE as f32;
                    let sample = (t * 440.0 * 2.0 * std::f32::consts::PI).sin() * 10000.0;
                    let _ = tx.send(sample as i16);
                }
            }
        }
        
        info!("Recording started");
        Ok(())
    }

    /// Build the CPAL input stream for a given sample type.
    #[cfg(feature = "audio")]
    fn build_stream<T>(
        device: &cpal::Device,
        config: &cpal::StreamConfig,
        tx: UnboundedSender<i16>,
        recording_flag: Arc<AtomicBool>,
    ) -> Result<cpal::Stream>
    where
        T: Sample + SizedSample + Send + 'static + ToPrimitive,
    {
        let err_fn = |err| error!("An error occurred on the audio stream: {}", err);

        let stream = device.build_input_stream(
            config,
            move |data: &[T], _: &cpal::InputCallbackInfo| {
                // Check if recording is still active.
                if !recording_flag.load(Ordering::Relaxed) {
                    return;
                }
                // Convert and send each sample over the channel.
                for &sample in data {
                    if let Some(value) = sample.to_i16() {
                        if let Err(e) = tx.send(value) {
                            error!("Failed to send sample: {}", e);
                        }
                    }
                }
                debug!("Sent {} samples", data.len());
            },
            err_fn,
            None,
        )?;
        Ok(stream)
    }

    /// Stop recording audio.
    pub fn stop_recording(&mut self) {
        debug!("Stopping audio recording");
        
        // Set the recording flag to false to stop any new samples from being sent
        self.recording_flag.store(false, Ordering::Relaxed);
        
        // Check if we have an active stream
        if let Some(stream) = self.stream.take() {
            // Explicitly drop the stream to stop recording
            drop(stream);
            debug!("Audio stream stopped");
        } else {
            debug!("No active audio stream to stop");
        }
        
        // Drain any remaining samples from the channel
        if let Some(mut rx) = self.samples_rx.take() {
            let mut samples = self.samples.lock().unwrap();
            while let Ok(sample) = rx.try_recv() {
                samples.push(sample);
            }
        }
        
        // Log the number of samples collected
        let sample_count = {
            let samples = self.samples.lock().unwrap();
            samples.len()
        };
        
        debug!(sample_count, "Recording stopped with samples");
    }

    /// Save the recorded audio to a WAV file
    pub fn save_to_wav(&self, path: &Path) -> Result<()> {
        let samples = self.samples.lock().unwrap();
        
        if samples.is_empty() {
            warn!("No audio samples to save!");
            return Err(anyhow::anyhow!("No audio samples to save"));
        }
        
        let spec = WavSpec {
            channels: CHANNELS,
            sample_rate: SAMPLE_RATE,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut writer = WavWriter::create(path, spec)
            .with_context(|| format!("Failed to create WAV writer for {:?}", path))?;
        for &sample in samples.iter() {
            writer.write_sample(sample)?;
        }

        // Explicitly flush and finalize the writer
        writer.flush()?;
        writer.finalize()?;
        
        debug!(
            path = %path.display(),
            sample_count = samples.len(),
            "Saved WAV file successfully"
        );
        
        Ok(())
    }

    /// Create a temporary WAV file with the recorded audio
    pub fn save_to_temp_wav(&self) -> Result<NamedTempFile> {
        let temp_file = Builder::new().suffix(".wav").tempfile()?;
        
        // Log the number of samples before saving
        let sample_count = {
            let samples = self.samples.lock().unwrap();
            debug!(sample_count = samples.len(), "Number of audio samples to save");
            samples.len()
        };
        
        self.save_to_wav(temp_file.path())?;
        
        // Log the file size after saving
        if let Ok(metadata) = std::fs::metadata(temp_file.path()) {
            let file_size = metadata.len();
            
            // Calculate expected file size (header + data)
            let expected_size = 44 + (sample_count * 2); // 44 bytes for WAV header, 2 bytes per sample
            
            debug!(
                file_size,
                expected_size,
                "Saved WAV file details"
            );
            
            if file_size < 100 {
                warn!(file_size, "WAV file is suspiciously small!");
            }
        }
        
        Ok(temp_file)
    }
}

impl Default for AudioRecorder {
    fn default() -> Self {
        Self::new()
    }
}
