//! Audio recording module.
//!
//! This module provides asynchronous functionality for recording audio from the microphone.
//! Recorded samples are sent over a Tokio unbounded channel and later drained to write to a WAV file.

use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Sample, SampleFormat, SizedSample};
use hound::{WavSpec, WavWriter};
use num_traits::cast::ToPrimitive;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use std::thread;
use tempfile::{Builder, NamedTempFile};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tracing::{debug, error, info};

/// Audio configuration constants.
pub const SAMPLE_RATE: u32 = 16000;
pub const CHANNELS: u16 = 1;

/// AudioRecorder holds the state for asynchronous audio recording.
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
}

/// Wrapper to allow cpal::Stream to be Send.
struct StreamWrapper(Arc<Mutex<cpal::Stream>>);

unsafe impl Send for StreamWrapper {}

impl StreamWrapper {
    fn new(stream: cpal::Stream) -> Self {
        Self(Arc::new(Mutex::new(stream)))
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
        }
    }

    /// Start recording audio.
    ///
    /// This method creates an unbounded channel to transfer samples from the CPAL audio callback.
    /// It selects the default input device and starts a CPAL stream that sends converted samples
    /// over the channel.
    pub fn start_recording(&mut self) -> Result<()> {
        // Create an unbounded channel so that high-rate audio samples aren’t dropped.
        let (tx, rx) = unbounded_channel();
        self.samples_tx = Some(tx);
        self.samples_rx = Some(rx);
        // Set the recording flag to true.
        self.recording_flag.store(true, Ordering::Relaxed);

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
        info!("audio.rs - Recording started.");
        Ok(())
    }

    /// Build the CPAL input stream for a given sample type.
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
    ///
    /// Pauses the stream, sets the recording flag to false, and waits briefly to
    /// allow any in-flight callbacks to finish before dropping the sender.
    pub fn stop_recording(&mut self) {
        // Set the flag so that the callback will early-exit.
        self.recording_flag.store(false, Ordering::Relaxed);

        // Pause the stream to stop further callbacks.
        if let Some(ref stream_wrapper) = self.stream {
            let stream_lock = stream_wrapper.0.lock().unwrap();
            if let Err(e) = stream_lock.pause() {
                error!("Failed to pause audio stream: {}", e);
            } else {
                info!("Audio stream paused successfully.");
            }
        }
        // Wait a short moment (e.g., 200ms) for any in-flight callbacks to complete.
        thread::sleep(Duration::from_millis(200));
        // Drop the sender so that the channel will eventually close.
        self.samples_tx = None;
        // Drop the stream handle.
        self.stream = None;
        info!("Recording stopped.");
    }

    /// Asynchronously drain the samples and save the audio as a WAV file.
    pub async fn save_to_wav(&mut self, path: &Path) -> Result<()> {
        let mut samples = Vec::new();
        if let Some(rx) = self.samples_rx.as_mut() {
            // Drain the channel until it is closed.
            while let Some(sample) = rx.recv().await {
                samples.push(sample);
            }
        }
        debug!("Collected {} samples for WAV file", samples.len());

        let spec = WavSpec {
            channels: CHANNELS,
            sample_rate: SAMPLE_RATE,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut writer = WavWriter::create(path, spec)
            .with_context(|| format!("Failed to create WAV writer for {:?}", path))?;
        for sample in samples {
            writer.write_sample(sample)?;
        }
        writer.finalize()?;
        info!("WAV file saved to {:?}", path);
        Ok(())
    }
    
    /// Asynchronously save the recorded audio to a temporary WAV file with a proper .wav suffix.
    pub async fn save_to_temp_wav(&mut self) -> Result<NamedTempFile> {
        // Use Builder to ensure the temporary file has a .wav extension.
        let temp_file = Builder::new().suffix(".wav").tempfile()?;
        self.save_to_wav(temp_file.path()).await?;
        Ok(temp_file)
    }
}

impl Default for AudioRecorder {
    fn default() -> Self {
        Self::new()
    }
}
