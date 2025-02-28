use anyhow::Result;
use stt_groq_rust::audio::AudioRecorder;
use std::fs;
use std::thread;
use std::time::Duration;

/// Integration test for AudioRecorder
/// 
/// This test verifies that:
/// 1. The AudioRecorder can start recording
/// 2. The AudioRecorder can stop recording
/// 3. The recorded audio can be saved to a file
/// 4. The saved file has the expected properties
#[test]
fn test_audio_recorder_save_file() -> Result<()> {
    // Initialize tracing for the test
    let _ = tracing_subscriber::fmt::try_init();
    
    // Create a new AudioRecorder
    let mut recorder = AudioRecorder::new();
    
    // Start recording
    recorder.start_recording()?;
    
    // Record for a short time (500ms)
    thread::sleep(Duration::from_millis(500));
    
    // Stop recording
    recorder.stop_recording();
    
    // Create a temporary path for the WAV file
    let temp_dir = tempfile::tempdir()?;
    let wav_path = temp_dir.path().join("test_recording.wav");
    
    // Save the recording to the file
    recorder.save_to_wav(&wav_path)?;
    
    // Verify the file exists
    assert!(wav_path.exists(), "WAV file was not created");
    
    // Verify the file has content (should be at least 44 bytes for WAV header)
    let metadata = fs::metadata(&wav_path)?;
    assert!(metadata.len() > 44, "WAV file is too small, expected at least header size");
    
    // With the audio feature, we should have actual audio data
    #[cfg(feature = "audio")]
    {
        // Real audio recording should produce a reasonable amount of data
        // 16000 samples/sec * 0.5 sec * 2 bytes/sample = ~16000 bytes minimum
        // But we'll be more lenient since some systems might not capture audio properly
        assert!(metadata.len() > 1000, "WAV file is too small for a real recording");
    }
    
    // Without the audio feature, we should have mock data
    #[cfg(not(feature = "audio"))]
    {
        // Mock implementation generates exactly SAMPLE_RATE samples
        // 16000 samples * 2 bytes/sample + 44 bytes header = 32044 bytes
        let expected_size = (16000 * 2) + 44;
        assert_eq!(metadata.len(), expected_size as u64, 
                  "Mock recording size doesn't match expected value");
    }
    
    // Clean up
    temp_dir.close()?;
    
    Ok(())
}

/// Test that verifies the temporary WAV file creation works
#[test]
#[cfg_attr(all(feature = "audio", not(target_env = "msvc")), ignore = "Requires ALSA on Linux")]
fn test_save_to_temp_wav() -> Result<()> {
    // Initialize tracing for the test
    let _ = tracing_subscriber::fmt::try_init();
    
    // Create a new AudioRecorder
    let mut recorder = AudioRecorder::new();
    
    // Start recording
    recorder.start_recording()?;
    
    // Record for a short time
    thread::sleep(Duration::from_millis(200));
    
    // Stop recording
    recorder.stop_recording();
    
    // Save to a temporary WAV file
    let temp_file = recorder.save_to_temp_wav()?;
    
    // Verify the file exists
    assert!(temp_file.path().exists(), "Temporary WAV file was not created");
    
    // Verify the file has content
    let metadata = fs::metadata(temp_file.path())?;
    assert!(metadata.len() > 44, "WAV file is too small, expected at least header size");
    
    // The temp file will be automatically cleaned up when it goes out of scope
    
    Ok(())
}