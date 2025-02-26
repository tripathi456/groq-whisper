//! Library for the Groq Whisper speech-to-text application.

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