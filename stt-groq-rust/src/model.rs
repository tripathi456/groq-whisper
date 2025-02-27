//! Model selection module for Groq Whisper API.
//! 
//! This module provides functionality to select Groq Whisper models in a round-robin fashion.

use std::sync::Mutex;
use std::iter::Cycle;

/// List of available Groq Whisper models.
pub const MODELS: &[&str] = &[
    "whisper-large-v3",
    "whisper-large-v3-turbo",
    "distil-whisper-large-v3-en",
    // Add additional models as needed.
];

/// ModelSelector provides round-robin selection of Groq Whisper models.
pub struct ModelSelector {
    models_cycle: Mutex<Cycle<std::slice::Iter<'static, &'static str>>>,
}

impl ModelSelector {
    /// Create a new ModelSelector instance.
    pub fn new() -> Self {
        Self {
            models_cycle: Mutex::new(MODELS.iter().cycle()),
        }
    }

    /// Get the next model in the round-robin sequence.
    #[instrument(ret)]
    pub fn get_next_model(&self) -> String {
        let mut cycle = self.models_cycle.lock().unwrap();
        let model = (*cycle.next().unwrap()).to_string();
        debug!(model, "Selected next model");
        model
    }
}

impl Default for ModelSelector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_cycle() {
        let selector = ModelSelector::new();
        
        // Test that we cycle through all models and then repeat
        let mut models = Vec::new();
        for _ in 0..MODELS.len() * 2 {
            models.push(selector.get_next_model());
        }
        
        // Check first cycle
        for i in 0..MODELS.len() {
            assert_eq!(models[i], MODELS[i]);
        }
        
        // Check second cycle
        for i in 0..MODELS.len() {
            assert_eq!(models[i + MODELS.len()], MODELS[i]);
        }
    }
}