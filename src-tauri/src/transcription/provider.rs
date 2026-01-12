//! Transcription Provider Trait
//!
//! Common interface for transcription backends.

use async_trait::async_trait;

/// Configuration for transcription
#[derive(Debug, Clone)]
pub struct TranscriptionConfig {
    /// Language code (ISO 639-1) or "auto" for detection
    pub language: String,
    /// Translate to English
    pub translate: bool,
}

impl Default for TranscriptionConfig {
    fn default() -> Self {
        Self {
            language: "auto".to_string(),
            translate: false,
        }
    }
}

/// Result of transcription
#[derive(Debug, Clone)]
pub struct TranscriptionResult {
    /// Transcribed text
    pub text: String,
    /// Detected language
    pub language: Option<String>,
    /// Transcription duration in milliseconds
    pub duration_ms: u64,
    /// Provider that performed the transcription
    pub provider: String,
}

/// Transcription errors
#[derive(Debug, thiserror::Error)]
pub enum TranscriptionError {
    #[error("Model not loaded")]
    ModelNotLoaded,

    #[error("Model not found: {0}")]
    ModelNotFound(String),

    #[error("Invalid audio: {0}")]
    InvalidAudio(String),

    #[error("API error: {0}")]
    ApiError(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Rate limited")]
    RateLimited,

    #[error("Transcription failed: {0}")]
    Failed(String),
}

/// Trait for transcription providers
#[async_trait]
pub trait TranscriptionProvider: Send + Sync {
    /// Transcribe audio samples to text
    async fn transcribe(
        &self,
        audio: &[f32],
        config: &TranscriptionConfig,
    ) -> Result<TranscriptionResult, TranscriptionError>;

    /// Get provider name
    fn name(&self) -> &'static str;

    /// Check if provider is available/configured
    fn is_available(&self) -> bool;

    /// Get estimated cost per minute (for cloud providers)
    fn cost_per_minute(&self) -> Option<f64> {
        None
    }
}
