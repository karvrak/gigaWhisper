//! Groq API Provider
//!
//! Cloud transcription using Groq's Whisper API.

use super::{TranscriptionConfig, TranscriptionError, TranscriptionProvider, TranscriptionResult};
use crate::audio::encode_wav;
use crate::config::SecretsManager;
use async_trait::async_trait;
use std::time::Instant;

const GROQ_API_URL: &str = "https://api.groq.com/openai/v1/audio/transcriptions";

/// Groq API transcription provider
pub struct GroqProvider {
    model: String,
    client: reqwest::Client,
}

impl GroqProvider {
    /// Create a new Groq provider
    pub fn new(model: Option<String>) -> Self {
        Self {
            model: model.unwrap_or_else(|| "whisper-large-v3".to_string()),
            client: reqwest::Client::new(),
        }
    }

    /// Get API key from secure storage
    fn get_api_key(&self) -> Option<String> {
        SecretsManager::get_groq_api_key().ok()
    }
}

#[async_trait]
impl TranscriptionProvider for GroqProvider {
    async fn transcribe(
        &self,
        audio: &[f32],
        config: &TranscriptionConfig,
    ) -> Result<TranscriptionResult, TranscriptionError> {
        let api_key = self.get_api_key().ok_or_else(|| {
            TranscriptionError::ApiError("API key not configured. Please set your Groq API key in settings.".to_string())
        })?;

        // Validate API key format
        if let Err(e) = SecretsManager::validate_groq_api_key(&api_key) {
            return Err(TranscriptionError::ApiError(format!("Invalid API key: {}", e)));
        }

        let start = Instant::now();

        // Encode audio as WAV
        let wav_data = encode_wav(audio, 16000, 1);

        // Build multipart form
        let file_part = reqwest::multipart::Part::bytes(wav_data)
            .file_name("audio.wav")
            .mime_str("audio/wav")
            .map_err(|e| TranscriptionError::Failed(e.to_string()))?;

        let mut form = reqwest::multipart::Form::new()
            .part("file", file_part)
            .text("model", self.model.clone())
            .text("response_format", "json");

        // Add language if specified
        if config.language != "auto" {
            form = form.text("language", config.language.clone());
        }

        // Make API request
        let response = self
            .client
            .post(GROQ_API_URL)
            .bearer_auth(&api_key)
            .multipart(form)
            .send()
            .await
            .map_err(|e| TranscriptionError::NetworkError(e.to_string()))?;

        // Check for rate limiting
        if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(TranscriptionError::RateLimited);
        }

        // Check for errors
        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(TranscriptionError::ApiError(error_text));
        }

        // Parse response
        let result: GroqResponse = response
            .json()
            .await
            .map_err(|e| TranscriptionError::Failed(e.to_string()))?;

        let duration_ms = start.elapsed().as_millis() as u64;

        Ok(TranscriptionResult {
            text: result.text.trim().to_string(),
            language: None,
            duration_ms,
            provider: "groq".to_string(),
        })
    }

    fn name(&self) -> &'static str {
        "groq"
    }

    fn is_available(&self) -> bool {
        self.get_api_key()
            .map(|key| SecretsManager::validate_groq_api_key(&key).is_ok())
            .unwrap_or(false)
    }

    fn cost_per_minute(&self) -> Option<f64> {
        // Groq pricing: Free tier with rate limits
        Some(0.0)
    }
}

/// Groq API response
#[derive(serde::Deserialize)]
struct GroqResponse {
    text: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_creation() {
        let provider = GroqProvider::new(None);
        assert_eq!(provider.name(), "groq");
        assert_eq!(provider.model, "whisper-large-v3");
    }

    #[test]
    fn test_provider_with_custom_model() {
        let provider = GroqProvider::new(Some("whisper-large-v3-turbo".to_string()));
        assert_eq!(provider.model, "whisper-large-v3-turbo");
    }

    #[test]
    fn test_api_key_validation() {
        // Invalid: empty
        assert!(SecretsManager::validate_groq_api_key("").is_err());

        // Invalid: wrong prefix
        assert!(SecretsManager::validate_groq_api_key("sk_test_key").is_err());

        // Invalid: too short
        assert!(SecretsManager::validate_groq_api_key("gsk_short").is_err());

        // Valid format
        let valid_key = "gsk_abcdefghijklmnopqrstuvwxyz123456789012345678901234";
        assert!(SecretsManager::validate_groq_api_key(valid_key).is_ok());
    }
}
