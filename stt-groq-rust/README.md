# Groq Whisper Speech-to-Text (Rust Implementation)

A Rust implementation of the Groq Whisper speech-to-text application. This application allows you to record audio by double-tapping the Alt key, transcribe it using Groq's Whisper API, and automatically paste the transcription.

## Features

- Record audio with a simple double-tap of the Alt key
- Transcribe audio using Groq's Whisper API
- Round-robin selection of Whisper models for optimal performance
- Automatic clipboard pasting of transcriptions
- Desktop notifications for recording status

## Prerequisites

- Rust and Cargo installed
- A Groq API key (set as the `GROQ_API_KEY` environment variable)
- Audio input device (microphone)

## Installation

1. Clone the repository
2. Build the application:

```bash
cd stt-groq-rust
cargo build --release
```

## Usage

1. Set your Groq API key as an environment variable:

```bash
export GROQ_API_KEY="your-api-key-here"
```

Alternatively, you can create a `.env` file in the project directory with:

```
GROQ_API_KEY=your-api-key-here
```

2. Run the application:

```bash
cargo run --release
```

3. Double-tap the Alt key to start recording
4. Speak into your microphone
5. Double-tap the Alt key again to stop recording and start transcription
6. The transcription will be automatically copied to your clipboard and pasted

## Configuration

The application uses the following Groq Whisper models in a round-robin fashion:

- whisper-large-v3
- whisper-large-v3-turbo
- distil-whisper-large-v3-en

You can modify the list of models in `src/model.rs`.

## License

This project is licensed under the same terms as the original Python implementation.