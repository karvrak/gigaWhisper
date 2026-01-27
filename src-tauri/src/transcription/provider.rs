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
#[derive(Debug, Clone, thiserror::Error)]
pub enum TranscriptionError {
    #[error("Model not loaded")]
    ModelNotLoaded,

    #[error("Model not found: {0}")]
    ModelNotFound(String),

    #[error("Invalid audio: {0}")]
    InvalidAudio(String),

    #[error("Invalid path encoding: {0}")]
    InvalidPath(String),

    #[error("API error: {0}")]
    ApiError(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Rate limited")]
    RateLimited,

    #[error("Transcription timed out after {0} seconds")]
    Timeout(u64),

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transcription_config_default() {
        let config = TranscriptionConfig::default();
        assert_eq!(config.language, "auto");
        assert!(!config.translate);
    }

    #[test]
    fn test_transcription_result_clone() {
        let result = TranscriptionResult {
            text: "Hello world".to_string(),
            language: Some("en".to_string()),
            duration_ms: 1500,
            provider: "test".to_string(),
        };

        let cloned = result.clone();
        assert_eq!(cloned.text, result.text);
        assert_eq!(cloned.language, result.language);
        assert_eq!(cloned.duration_ms, result.duration_ms);
        assert_eq!(cloned.provider, result.provider);
    }

    #[test]
    fn test_transcription_error_display() {
        let err = TranscriptionError::ModelNotLoaded;
        assert_eq!(format!("{}", err), "Model not loaded");

        let err = TranscriptionError::ModelNotFound("tiny.bin".to_string());
        assert!(format!("{}", err).contains("tiny.bin"));

        let err = TranscriptionError::Timeout(30);
        assert!(format!("{}", err).contains("30"));

        let err = TranscriptionError::RateLimited;
        assert_eq!(format!("{}", err), "Rate limited");
    }

    #[test]
    fn test_transcription_config_custom() {
        let config = TranscriptionConfig {
            language: "fr".to_string(),
            translate: true,
        };
        assert_eq!(config.language, "fr");
        assert!(config.translate);
    }
}
