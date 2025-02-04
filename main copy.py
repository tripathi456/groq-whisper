import os
import tempfile
import wave
import pyaudio
import pyautogui
import pyperclip
import time
from groq import Groq
from pynput import keyboard  # Using pynput instead of keyboard

# Set up Groq client
client = Groq(api_key=os.environ.get("GROQ_API_KEY"))


def record_audio(sample_rate=16000, channels=1, chunk=1024):
    """
    Record audio from the microphone while the PAUSE button is held down.
    Uses pynput to listen for the PAUSE key.
    """
    p = pyaudio.PyAudio()
    stream = p.open(
        format=pyaudio.paInt16,
        channels=channels,
        rate=sample_rate,
        input=True,
        frames_per_buffer=chunk,
    )

    print("Press and hold the PAUSE button to start recording...")

    recording = False  # Flag to indicate whether we are recording

    def on_press(key):
        nonlocal recording
        try:
            # Change this line to use F9 instead of Pause
            if key == keyboard.Key.f9 and not recording:
                recording = True
                print("Recording... (Release F9 to stop)")
        except Exception as e:
            print(f"Error in on_press: {e}")

    def on_release(key):
        nonlocal recording
        try:
            # Change this line to use F9 instead of Pause
            if key == keyboard.Key.f9:
                recording = False
                return False  # Stop the listener once F9 is released
        except Exception as e:
            print(f"Error in on_release: {e}")

    # # Define functions to update the recording flag
    # def on_press(key):
    #     nonlocal recording
    #     try:
    #         if key == keyboard.Key.pause and not recording:
    #             recording = True
    #             print("Recording... (Release PAUSE to stop)")
    #     except Exception as e:
    #         print(f"Error in on_press: {e}")

    # def on_release(key):
    #     nonlocal recording
    #     try:
    #         if key == keyboard.Key.pause:
    #             recording = False
    #             # Stop the listener once PAUSE is released
    #             return False
    #     except Exception as e:
    #         print(f"Error in on_release: {e}")

    # Start the listener in a separate thread
    listener = keyboard.Listener(on_press=on_press, on_release=on_release)
    listener.start()

    # Wait until the PAUSE key is pressed to start recording
    while not recording:
        time.sleep(0.05)

    frames = []
    # Continue recording while the PAUSE key remains pressed
    while recording:
        try:
            data = stream.read(chunk)
            frames.append(data)
        except Exception as e:
            print(f"Error while recording: {e}")
            break

    # Ensure the listener is stopped
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
        return transcription  # This is now directly the transcription text
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
        # Record audio
        frames, sample_rate = record_audio()

        # Save audio to temporary file
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

        print("\nReady for next recording. Press and hold PAUSE to start.")

if __name__ == "__main__":
    main()
