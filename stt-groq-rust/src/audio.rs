use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Sample, SampleFormat, SizedSample};
use hound::{WavSpec, WavWriter};
use num_traits::cast::ToPrimitive;
use std::path::Path;
use tempfile::{Builder, NamedTempFile};
use tokio::sync::mpsc::{self, Sender, Receiver};

pub const SAMPLE_RATE: u32 = 16000;
pub const CHANNELS: u16 = 1;

pub struct AudioRecorder {
    // Instead of a shared Vec, we use a Tokio mpsc channel to send samples.
    samples_tx: Option<Sender<i16>>,
    samples_rx: Option<Receiver<i16>>,
    #[allow(dead_code)]
    stream: Option<StreamWrapper>,
}

struct StreamWrapper(Arc<Mutex<cpal::Stream>>);
use std::sync::{Arc, Mutex};

unsafe impl Send for StreamWrapper {}

impl StreamWrapper {
    fn new(stream: cpal::Stream) -> Self {
        StreamWrapper(Arc::new(Mutex::new(stream)))
    }
}

impl AudioRecorder {
    pub fn new() -> Self {
        Self {
            samples_tx: None,
            samples_rx: None,
            stream: None,
        }
    }

    /// Start recording audio via CPAL. Samples are sent over a Tokio mpsc channel.
    pub fn start_recording(&mut self) -> Result<()> {
        // Create a channel with capacity for many samples.
        let (tx, rx) = mpsc::channel(4096);
        self.samples_tx = Some(tx);
        self.samples_rx = Some(rx);

        // Get default host and input device.
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .context("Failed to get default input device")?;
        println!("Using input device: {}", device.name()?);

        // Configure the input stream.
        let config = cpal::StreamConfig {
            channels: CHANNELS as cpal::ChannelCount,
            sample_rate: cpal::SampleRate(SAMPLE_RATE),
            buffer_size: cpal::BufferSize::Default,
        };

        // Clone the sender to move into the callback.
        let tx_clone = self.samples_tx.as_ref().unwrap().clone();

        // Build and start the stream.
        let stream = match device.default_input_config()?.sample_format() {
            SampleFormat::I16 => self.build_stream::<i16>(&device, &config, tx_clone)?,
            SampleFormat::U16 => self.build_stream::<u16>(&device, &config, tx_clone)?,
            SampleFormat::F32 => self.build_stream::<f32>(&device, &config, tx_clone)?,
            format => return Err(anyhow::anyhow!("Unsupported sample format: {:?}", format)),
        };

        stream.play()?;
        self.stream = Some(StreamWrapper::new(stream));
        Ok(())
    }

    fn build_stream<T>(
        &self,
        device: &cpal::Device,
        config: &cpal::StreamConfig,
        tx: Sender<i16>,
    ) -> Result<cpal::Stream>
    where
        T: Sample + SizedSample + Send + 'static + ToPrimitive,
    {
        let err_fn = |err| eprintln!("An error occurred on the audio stream: {}", err);

        let stream = device.build_input_stream(
            config,
            move |data: &[T], _: &cpal::InputCallbackInfo| {
                for &sample in data {
                    if let Some(value) = sample.to_i16() {
                        // Try sending without blocking. If full, drop the sample.
                        let _ = tx.try_send(value);
                    }
                }
            },
            err_fn,
            None,
        )?;

        Ok(stream)
    }

    /// Stop recording by dropping the sender and stream.
    pub fn stop_recording(&mut self) {
        self.samples_tx = None; // Dropping the sender closes the channel.
        self.stream = None;
    }

    /// Asynchronously drain the channel and save the recorded audio as a WAV file.
    pub async fn save_to_wav(&mut self, path: &Path) -> Result<()> {
        let mut samples = Vec::new();
        if let Some(rx) = self.samples_rx.as_mut() {
            while let Some(sample) = rx.recv().await {
                samples.push(sample);
            }
        }

        let spec = WavSpec {
            channels: CHANNELS,
            sample_rate: SAMPLE_RATE,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut writer = WavWriter::create(path, spec)?;
        for sample in samples {
            writer.write_sample(sample)?;
        }
        writer.finalize()?;
        Ok(())
    }

    /// Asynchronously save to a temporary WAV file.
    pub async fn save_to_temp_wav(&mut self) -> Result<NamedTempFile> {
        // Create a temporary file with a .wav suffix.
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
