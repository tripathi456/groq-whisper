# main.py

import os
import tempfile
import wave
import threading
import time
import pyaudio
import pyperclip
import pyautogui
from groq import Groq
from pynput import keyboard
from notifications import show_notification
from model import get_next_model  # Import the round-robin model selector

# Set up the Groq client using your environment variable for API key.
client = Groq(api_key=os.environ.get("GROQ_API_KEY"))

# Global variables for recording state
recording = False
recording_frames = []
recording_start_time = None
recording_thread = None
audio_stream = None
pyaudio_instance = None

# Audio configuration
SAMPLE_RATE = 16000
CHANNELS = 1
CHUNK = 1024

# Double-tap detection for Ctrl
last_alt_time = 0
ALT_THRESHOLD = 0.5  # seconds within which two Ctrl presses count as a double tap

def start_audio_stream():
    """Initialize the PyAudio stream."""
    global pyaudio_instance, audio_stream
    pyaudio_instance = pyaudio.PyAudio()
    audio_stream = pyaudio_instance.open(
        format=pyaudio.paInt16,
        channels=CHANNELS,
        rate=SAMPLE_RATE,
        input=True,
        frames_per_buffer=CHUNK,
    )

def stop_audio_stream():
    """Terminate the PyAudio stream."""
    global pyaudio_instance, audio_stream
    if audio_stream is not None:
        audio_stream.stop_stream()
        audio_stream.close()
    if pyaudio_instance is not None:
        pyaudio_instance.terminate()

def audio_recording():
    """Record audio continuously until 'recording' is set to False."""
    global recording, recording_frames, audio_stream
    while recording:
        try:
            data = audio_stream.read(CHUNK)
            recording_frames.append(data)
        except Exception as e:
            print("Error while recording audio:", e)
            break

def save_audio(frames, sample_rate):
    """Save recorded frames to a temporary WAV file."""
    with tempfile.NamedTemporaryFile(suffix=".wav", delete=False) as temp_audio:
        wf = wave.open(temp_audio.name, "wb")
        wf.setnchannels(CHANNELS)
        wf.setsampwidth(pyaudio.PyAudio().get_sample_size(pyaudio.paInt16))
        wf.setframerate(sample_rate)
        wf.writeframes(b"".join(frames))
        wf.close()
        return temp_audio.name

def transcribe_audio(audio_file_path):
    """
    Transcribe the audio file using Groq's Whisper implementation.
    Automatically selects the next model from the round-robin list.
    Returns the transcription text on success or None on failure.
    """
    try:
        with open(audio_file_path, "rb") as file:
            # Select the next model from the round-robin list.
            model_name = get_next_model()
            print(f"Using model: {model_name}")
            transcription = client.audio.transcriptions.create(
                file=(os.path.basename(audio_file_path), file.read()),
                model=model_name,
                prompt=(
                    "The audio is by a programmer discussing programming issues "
                ),
                response_format="text",
                language="en",
            )
        return transcription
    except Exception as e:
        print(f"An error occurred during transcription: {str(e)}")
        return None

def copy_transcription_to_clipboard(text):
    """Copy the transcribed text to the clipboard and paste it using pyautogui."""
    pyperclip.copy(text)
    pyautogui.hotkey("ctrl", "v")

def on_press(key):
    """
    Callback for key press events.
    Detects a double-tap on the Alt key to toggle recording.
    """
    global last_alt_time, recording, recording_frames, recording_start_time, recording_thread
    if key in (keyboard.Key.alt, keyboard.Key.alt_l, keyboard.Key.alt_r):
        current_time = time.time()
        if current_time - last_alt_time < ALT_THRESHOLD:
            # Double Alt detected: toggle recording
            if not recording:
                # Start recording
                recording = True
                recording_frames = []
                recording_start_time = current_time
                start_audio_stream()
                recording_thread = threading.Thread(target=audio_recording)
                recording_thread.start()
                print("Recording started.")
                show_notification("Recording Started", "Audio recording has started.")
            else:
                # Stop recording
                recording = False
                print("Recording stopped.")
            last_alt_time = 0  # reset the timer
        else:
            last_alt_time = current_time

def on_release(key):
    # Not used, but required by the Listener.
    pass

def keyboard_listener():
    """Starts the keyboard listener to monitor key presses."""
    with keyboard.Listener(on_press=on_press, on_release=on_release) as listener:
        listener.join()

def main():
    global recording, recording_frames, recording_start_time, recording_thread
    # Start the keyboard listener in a separate thread.
    listener_thread = threading.Thread(target=keyboard_listener, daemon=True)
    listener_thread.start()

    print("Double-tap the Alt key (press Alt twice quickly) to toggle recording on/off.")

    while True:
        # When not recording and there is a finished recording thread, process the recording.
        if not recording and recording_thread is not None:
            # Wait for the recording thread to finish.
            recording_thread.join()
            recording_thread = None
            recording_duration = time.time() - recording_start_time if recording_start_time else 0
            stop_audio_stream()
            if recording_duration < 5:
                print("Recording duration was less than 5 seconds. Skipping transcription.")
            else:
                # Save the recorded audio to a file.
                audio_file = save_audio(recording_frames, SAMPLE_RATE)
                print("Transcribing...")
                transcription = transcribe_audio(audio_file)
                if transcription:
                    print("\nTranscription:")
                    print(transcription)
                    copy_transcription_to_clipboard(transcription)
                    print("Transcription copied to clipboard.")
                else:
                    print("Transcription failed.")
                # Clean up the temporary file.
                os.unlink(audio_file)
            # Reset recording data for the next recording session.
            recording_thread = None
            recording_frames = []
            recording_start_time = None
        time.sleep(0.1)

if __name__ == "__main__":
    main()
