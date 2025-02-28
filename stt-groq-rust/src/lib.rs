//! Library for the Groq Whisper speech-to-text application.

// Re-export modules for use in integration tests and other crates
pub mod audio;
pub mod clipboard;
pub mod groq_client;
pub mod keyboard;
pub mod model;
pub mod notifications;
pub mod tracing;

/// A simple function that adds two numbers.
/// This is used for testing purposes.
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        assert_eq!(add(2, 2), 4);
    }
}