//! Groq API Provider
//!
//! Cloud transcription using Groq's Whisper API.

use super::{TranscriptionConfig, TranscriptionError, TranscriptionProvider, TranscriptionResult};
use crate::audio::encode_wav;
use crate::config::SecretsManager;
use async_trait::async_trait;
use std::time::{Duration, Instant};

const GROQ_API_URL: &str = "https://api.groq.com/openai/v1/audio/transcriptions";
const DEFAULT_TIMEOUT_SECONDS: u64 = 30;
const DEFAULT_MAX_RETRIES: u32 = 3;
const RETRY_BASE_DELAY_MS: u64 = 1000;

/// Groq API transcription provider
pub struct GroqProvider {
    model: String,
    client: reqwest::Client,
    timeout: Duration,
    max_retries: u32,
}

impl GroqProvider {
    /// Create a new Groq provider with default timeout
    pub fn new(model: Option<String>) -> Self {
        Self::with_timeout(model, DEFAULT_TIMEOUT_SECONDS)
    }

    /// Create a new Groq provider with custom timeout
    pub fn with_timeout(model: Option<String>, timeout_seconds: u64) -> Self {
        Self::with_config(model, timeout_seconds, DEFAULT_MAX_RETRIES)
    }

    /// Create a new Groq provider with full configuration
    pub fn with_config(model: Option<String>, timeout_seconds: u64, max_retries: u32) -> Self {
        let timeout = Duration::from_secs(timeout_seconds);
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            model: model.unwrap_or_else(|| "whisper-large-v3".to_string()),
            client,
            timeout,
            max_retries,
        }
    }

    /// Get API key from secure storage
    fn get_api_key(&self) -> Option<String> {
        SecretsManager::get_groq_api_key().ok()
    }

    /// Get the current timeout
    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    /// Get the max retries
    pub fn max_retries(&self) -> u32 {
        self.max_retries
    }

    /// Calculate delay for retry with exponential backoff
    fn retry_delay(attempt: u32) -> Duration {
        let delay_ms = RETRY_BASE_DELAY_MS * 2u64.pow(attempt);
        Duration::from_millis(delay_ms.min(30_000)) // Cap at 30 seconds
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

        // Encode audio as WAV (done once, reused across retries)
        let wav_data = encode_wav(audio, 16000, 1);

        let mut last_error: Option<TranscriptionError> = None;

        // Retry loop with exponential backoff
        for attempt in 0..=self.max_retries {
            if attempt > 0 {
                let delay = Self::retry_delay(attempt - 1);
                tracing::info!(
                    "Retrying Groq API request (attempt {}/{}) after {:?}",
                    attempt + 1,
                    self.max_retries + 1,
                    delay
                );
                tokio::time::sleep(delay).await;
            }

            // Build multipart form (must be rebuilt for each attempt)
            let file_part = match reqwest::multipart::Part::bytes(wav_data.clone())
                .file_name("audio.wav")
                .mime_str("audio/wav")
            {
                Ok(part) => part,
                Err(e) => return Err(TranscriptionError::Failed(e.to_string())),
            };

            let mut form = reqwest::multipart::Form::new()
                .part("file", file_part)
                .text("model", self.model.clone())
                .text("response_format", "json");

            // Add language if specified
            if config.language != "auto" {
                form = form.text("language", config.language.clone());
            }

            // Make API request
            let response = match self
                .client
                .post(GROQ_API_URL)
                .bearer_auth(&api_key)
                .multipart(form)
                .send()
                .await
            {
                Ok(resp) => resp,
                Err(e) => {
                    let error = TranscriptionError::NetworkError(e.to_string());
                    tracing::warn!("Groq API network error: {}", e);
                    last_error = Some(error);
                    continue;
                }
            };

            // Check for rate limiting (retryable)
            if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
                tracing::warn!("Groq API rate limited");
                last_error = Some(TranscriptionError::RateLimited);
                continue;
            }

            // Check for server errors (5xx - retryable)
            if response.status().is_server_error() {
                let error_text = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Server error".to_string());
                tracing::warn!("Groq API server error: {}", error_text);
                last_error = Some(TranscriptionError::NetworkError(error_text));
                continue;
            }

            // Check for client errors (4xx - not retryable except rate limit)
            if !response.status().is_success() {
                let error_text = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unknown error".to_string());
                return Err(TranscriptionError::ApiError(error_text));
            }

            // Parse response
            let result: GroqResponse = match response.json().await {
                Ok(r) => r,
                Err(e) => {
                    tracing::warn!("Failed to parse Groq response: {}", e);
                    last_error = Some(TranscriptionError::Failed(e.to_string()));
                    continue;
                }
            };

            let duration_ms = start.elapsed().as_millis() as u64;

            if attempt > 0 {
                tracing::info!("Groq API request succeeded after {} retries", attempt);
            }

            return Ok(TranscriptionResult {
                text: result.text.trim().to_string(),
                language: None,
                duration_ms,
                provider: "groq".to_string(),
            });
        }

        // All retries exhausted
        Err(last_error.unwrap_or_else(|| {
            TranscriptionError::Failed("All retry attempts failed".to_string())
        }))
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

    // ============================================================
    // Provider Creation Tests
    // ============================================================

    #[test]
    fn test_provider_creation_default_model() {
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
    fn test_provider_with_empty_model() {
        let provider = GroqProvider::new(Some("".to_string()));
        assert_eq!(provider.model, "");
    }

    // ============================================================
    // Timeout Configuration Tests
    // ============================================================

    #[test]
    fn test_default_timeout() {
        let provider = GroqProvider::new(None);
        assert_eq!(provider.timeout(), Duration::from_secs(DEFAULT_TIMEOUT_SECONDS));
    }

    #[test]
    fn test_custom_timeout() {
        let provider = GroqProvider::with_timeout(None, 60);
        assert_eq!(provider.timeout(), Duration::from_secs(60));
    }

    #[test]
    fn test_zero_timeout() {
        let provider = GroqProvider::with_timeout(None, 0);
        assert_eq!(provider.timeout(), Duration::ZERO);
    }

    #[test]
    fn test_very_long_timeout() {
        let provider = GroqProvider::with_timeout(None, 3600);
        assert_eq!(provider.timeout(), Duration::from_secs(3600));
    }

    // ============================================================
    // Retry Configuration Tests
    // ============================================================

    #[test]
    fn test_default_max_retries() {
        let provider = GroqProvider::new(None);
        assert_eq!(provider.max_retries(), DEFAULT_MAX_RETRIES);
    }

    #[test]
    fn test_custom_max_retries() {
        let provider = GroqProvider::with_config(None, 30, 5);
        assert_eq!(provider.max_retries(), 5);
    }

    #[test]
    fn test_zero_retries() {
        let provider = GroqProvider::with_config(None, 30, 0);
        assert_eq!(provider.max_retries(), 0);
    }

    // ============================================================
    // Retry Delay Tests
    // ============================================================

    #[test]
    fn test_retry_delay_first_attempt() {
        let delay = GroqProvider::retry_delay(0);
        assert_eq!(delay, Duration::from_millis(RETRY_BASE_DELAY_MS));
    }

    #[test]
    fn test_retry_delay_exponential_backoff() {
        let delay_0 = GroqProvider::retry_delay(0);
        let delay_1 = GroqProvider::retry_delay(1);
        let delay_2 = GroqProvider::retry_delay(2);

        assert_eq!(delay_0, Duration::from_millis(1000));  // 1s
        assert_eq!(delay_1, Duration::from_millis(2000));  // 2s
        assert_eq!(delay_2, Duration::from_millis(4000));  // 4s
    }

    #[test]
    fn test_retry_delay_capped_at_30_seconds() {
        // Very high attempt number
        let delay = GroqProvider::retry_delay(10);
        assert!(delay <= Duration::from_secs(30));
    }

    #[test]
    fn test_retry_delay_multiple_attempts() {
        // Test that delays increase exponentially but cap at 30s
        let delays: Vec<_> = (0..10).map(|i| GroqProvider::retry_delay(i)).collect();

        // Each delay should be >= previous (up to cap)
        for window in delays.windows(2) {
            assert!(window[1] >= window[0] || window[1] == Duration::from_secs(30));
        }
    }

    // ============================================================
    // API Key Validation Tests
    // ============================================================

    #[test]
    fn test_api_key_validation_empty() {
        assert!(SecretsManager::validate_groq_api_key("").is_err());
    }

    #[test]
    fn test_api_key_validation_wrong_prefix() {
        assert!(SecretsManager::validate_groq_api_key("sk_test_key").is_err());
    }

    #[test]
    fn test_api_key_validation_too_short() {
        assert!(SecretsManager::validate_groq_api_key("gsk_short").is_err());
    }

    #[test]
    fn test_api_key_validation_valid_format() {
        let valid_key = "gsk_abcdefghijklmnopqrstuvwxyz123456789012345678901234";
        assert!(SecretsManager::validate_groq_api_key(valid_key).is_ok());
    }

    #[test]
    fn test_api_key_validation_whitespace() {
        assert!(SecretsManager::validate_groq_api_key("  ").is_err());
        assert!(SecretsManager::validate_groq_api_key("\t").is_err());
        assert!(SecretsManager::validate_groq_api_key("\n").is_err());
    }

    #[test]
    fn test_api_key_validation_prefix_only() {
        assert!(SecretsManager::validate_groq_api_key("gsk_").is_err());
    }

    // ============================================================
    // Trait Implementation Tests
    // ============================================================

    #[test]
    fn test_name_returns_groq() {
        let provider = GroqProvider::new(None);
        assert_eq!(provider.name(), "groq");
    }

    #[test]
    fn test_cost_per_minute_returns_some() {
        let provider = GroqProvider::new(None);
        let cost = provider.cost_per_minute();
        assert!(cost.is_some());
        // Groq is free tier
        assert_eq!(cost, Some(0.0));
    }

    // ============================================================
    // Full Configuration Tests
    // ============================================================

    #[test]
    fn test_full_configuration() {
        let provider = GroqProvider::with_config(
            Some("custom-model".to_string()),
            120,
            5,
        );

        assert_eq!(provider.model, "custom-model");
        assert_eq!(provider.timeout(), Duration::from_secs(120));
        assert_eq!(provider.max_retries(), 5);
    }

    #[test]
    fn test_configuration_inheritance() {
        let provider = GroqProvider::with_timeout(None, 45);

        // Should have custom timeout but default retries
        assert_eq!(provider.timeout(), Duration::from_secs(45));
        assert_eq!(provider.max_retries(), DEFAULT_MAX_RETRIES);
    }

    // ============================================================
    // Edge Cases
    // ============================================================

    #[test]
    fn test_model_with_special_characters() {
        let provider = GroqProvider::new(Some("model-v3.5_turbo".to_string()));
        assert_eq!(provider.model, "model-v3.5_turbo");
    }

    #[test]
    fn test_model_with_unicode() {
        let provider = GroqProvider::new(Some("model".to_string()));
        assert_eq!(provider.model, "model");
    }

    // ============================================================
    // Availability Tests (without real API key)
    // ============================================================

    #[test]
    fn test_is_available_without_api_key() {
        // Without a configured API key, should not be available
        let provider = GroqProvider::new(None);
        // This depends on whether an API key is configured in the keyring
        // In test environment, typically no key is configured
        let _available = provider.is_available();
        // We just verify it doesn't panic
    }

    // ============================================================
    // GroqResponse Parsing Tests (via serde)
    // ============================================================

    #[test]
    fn test_groq_response_deserialization() {
        let json = r#"{"text": "Hello, world!"}"#;
        let response: Result<GroqResponse, _> = serde_json::from_str(json);
        assert!(response.is_ok());
        assert_eq!(response.unwrap().text, "Hello, world!");
    }

    #[test]
    fn test_groq_response_with_empty_text() {
        let json = r#"{"text": ""}"#;
        let response: Result<GroqResponse, _> = serde_json::from_str(json);
        assert!(response.is_ok());
        assert_eq!(response.unwrap().text, "");
    }

    #[test]
    fn test_groq_response_with_unicode() {
        let json = r#"{"text": "Bonjour le monde!"}"#;
        let response: Result<GroqResponse, _> = serde_json::from_str(json);
        assert!(response.is_ok());
        assert_eq!(response.unwrap().text, "Bonjour le monde!");
    }

    #[test]
    fn test_groq_response_with_newlines() {
        let json = r#"{"text": "Line 1\nLine 2"}"#;
        let response: Result<GroqResponse, _> = serde_json::from_str(json);
        assert!(response.is_ok());
        assert_eq!(response.unwrap().text, "Line 1\nLine 2");
    }

    #[test]
    fn test_groq_response_missing_text_field() {
        let json = r#"{"error": "something went wrong"}"#;
        let response: Result<GroqResponse, _> = serde_json::from_str(json);
        assert!(response.is_err());
    }

    // ============================================================
    // Async Tests (without network calls)
    // ============================================================

    #[tokio::test]
    async fn test_transcribe_without_api_key() {
        let provider = GroqProvider::new(None);
        let config = TranscriptionConfig::default();

        // Without API key, should fail early
        let result = provider.transcribe(&[0.0; 1000], &config).await;

        // Expected to fail with ApiError about missing key
        assert!(result.is_err());
        match result {
            Err(TranscriptionError::ApiError(msg)) => {
                assert!(msg.contains("API key"));
            }
            Err(e) => panic!("Expected ApiError, got: {:?}", e),
            Ok(_) => panic!("Expected error without API key"),
        }
    }

    // ============================================================
    // HTTP Client Configuration Tests
    // ============================================================

    #[test]
    fn test_client_is_created() {
        // Just verify that creating the provider doesn't panic
        let _provider = GroqProvider::new(None);
        let _provider = GroqProvider::with_timeout(None, 60);
        let _provider = GroqProvider::with_config(None, 30, 3);
    }

    #[test]
    fn test_client_with_various_timeouts() {
        // Test that various timeout values don't cause panics
        let timeouts = [0, 1, 30, 60, 120, 300, 3600];

        for timeout in timeouts {
            let provider = GroqProvider::with_timeout(None, timeout);
            assert_eq!(provider.timeout(), Duration::from_secs(timeout));
        }
    }
}
