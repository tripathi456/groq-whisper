import os
import tempfile
import wave
import time
import pyaudio
import pyautogui
import pyperclip
from groq import Groq
from pynput.mouse import Listener as MouseListener, Button

# Set up Groq client
client = Groq(api_key=os.environ.get("GROQ_API_KEY"))

def record_audio(sample_rate=16000, channels=1, chunk=1024):
    """
    Record audio from the microphone while the middle mouse button is pressed.
    """
    p = pyaudio.PyAudio()
    stream = p.open(
        format=pyaudio.paInt16,
        channels=channels,
        rate=sample_rate,
        input=True,
        frames_per_buffer=chunk,
    )

    print("Press and hold the MIDDLE mouse button to start recording...")

    recording = False  # Flag to indicate if we are recording

    # Callback for mouse events
    def on_click(x, y, button, pressed):
        nonlocal recording
        if button == Button.middle:
            if pressed:
                if not recording:
                    recording = True
                    print("Recording... (Release the middle mouse button to stop)")
            else:
                if recording:
                    recording = False
                    # Return False to stop the mouse listener once the button is released
                    return False

    # Start mouse listener in a separate thread
    listener = MouseListener(on_click=on_click)
    listener.start()

    # Wait until the middle mouse button is pressed
    while not recording:
        time.sleep(0.05)

    frames = []
    # Continue recording while the middle mouse button is pressed
    while recording:
        try:
            data = stream.read(chunk)
            frames.append(data)
        except Exception as e:
            print(f"Error while recording: {e}")
            break

    # Wait for the listener to finish (i.e. button release)
    listener.join()

    print("Recording finished.")
    stream.stop_stream()
    stream.close()
    p.terminate()

    return frames, sample_rate


def save_audio(frames, sample_rate):
    """
    Save recorded audio to a temporary WAV file.
    """
    with tempfile.NamedTemporaryFile(suffix=".wav", delete=False) as temp_audio:
        wf = wave.open(temp_audio.name, "wb")
        wf.setnchannels(1)
        wf.setsampwidth(pyaudio.PyAudio().get_sample_size(pyaudio.paInt16))
        wf.setframerate(sample_rate)
        wf.writeframes(b"".join(frames))
        wf.close()
        return temp_audio.name


def transcribe_audio(audio_file_path):
    """
    Transcribe audio using Groq's Whisper implementation.
    """
    try:
        with open(audio_file_path, "rb") as file:
            transcription = client.audio.transcriptions.create(
                file=(os.path.basename(audio_file_path), file.read()),
                model="whisper-large-v3",
                prompt="""The audio is by a programmer discussing programming issues, the programmer mostly uses python and might mention python libraries or reference code in his speech.""",
                response_format="text",
                language="en",
            )
        return transcription  # This is directly the transcription text
    except Exception as e:
        print(f"An error occurred during transcription: {str(e)}")
        return None


def copy_transcription_to_clipboard(text):
    """
    Copy the transcribed text to clipboard using pyperclip.
    """
    pyperclip.copy(text)
    pyautogui.hotkey("ctrl", "v")


def main():
    while True:
        # Record audio using the middle mouse button
        frames, sample_rate = record_audio()

        # Save audio to a temporary file
        temp_audio_file = save_audio(frames, sample_rate)

        # Transcribe audio
        print("Transcribing...")
        transcription = transcribe_audio(temp_audio_file)

        # Copy transcription to clipboard
        if transcription:
            print("\nTranscription:")
            print(transcription)
            print("Copying transcription to clipboard...")
            copy_transcription_to_clipboard(transcription)
            print("Transcription copied to clipboard and pasted into the application.")
        else:
            print("Transcription failed.")

        # Clean up temporary file
        os.unlink(temp_audio_file)

        print("\nReady for next recording. Press and hold the middle mouse button to start.")

if __name__ == "__main__":
    main()
