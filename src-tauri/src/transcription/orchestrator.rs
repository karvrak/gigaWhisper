//! Transcription Orchestrator
//!
//! Manages provider selection and fallback logic.

use super::{TranscriptionConfig, TranscriptionError, TranscriptionProvider, TranscriptionResult};

/// Orchestrates transcription across multiple providers
pub struct TranscriptionOrchestrator {
    primary: Box<dyn TranscriptionProvider>,
    fallback: Option<Box<dyn TranscriptionProvider>>,
}

impl TranscriptionOrchestrator {
    /// Create orchestrator with a single provider
    pub fn new(primary: Box<dyn TranscriptionProvider>) -> Self {
        Self {
            primary,
            fallback: None,
        }
    }

    /// Create orchestrator with fallback provider
    pub fn with_fallback(
        primary: Box<dyn TranscriptionProvider>,
        fallback: Box<dyn TranscriptionProvider>,
    ) -> Self {
        Self {
            primary,
            fallback: Some(fallback),
        }
    }

    /// Transcribe audio using primary provider with optional fallback
    pub async fn transcribe(
        &self,
        audio: &[f32],
        config: &TranscriptionConfig,
    ) -> Result<TranscriptionResult, TranscriptionError> {
        // Validate audio
        if audio.is_empty() {
            return Err(TranscriptionError::InvalidAudio("Empty audio".to_string()));
        }

        // Try primary provider
        match self.primary.transcribe(audio, config).await {
            Ok(result) => {
                tracing::info!(
                    "Transcription completed by {} in {}ms",
                    result.provider,
                    result.duration_ms
                );
                return Ok(result);
            }
            Err(e) => {
                tracing::warn!("Primary provider {} failed: {}", self.primary.name(), e);

                // Try fallback if available
                if let Some(fallback) = &self.fallback {
                    if fallback.is_available() {
                        tracing::info!("Attempting fallback provider: {}", fallback.name());
                        return fallback.transcribe(audio, config).await;
                    }
                }

                // No fallback available, propagate error
                return Err(e);
            }
        }
    }

    /// Get primary provider name
    pub fn primary_provider(&self) -> &'static str {
        self.primary.name()
    }

    /// Get fallback provider name if configured
    pub fn fallback_provider(&self) -> Option<&'static str> {
        self.fallback.as_ref().map(|p| p.name())
    }

    /// Check if primary provider is available
    pub fn is_primary_available(&self) -> bool {
        self.primary.is_available()
    }

    /// Check if fallback provider is available
    pub fn is_fallback_available(&self) -> bool {
        self.fallback.as_ref().map(|p| p.is_available()).unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    /// Mock provider for testing with configurable behavior
    struct MockProvider {
        name: &'static str,
        available: bool,
        should_fail: bool,
        fail_error: Option<TranscriptionError>,
        delay_ms: Option<u64>,
        call_count: Arc<AtomicU32>,
    }

    impl MockProvider {
        fn new(name: &'static str) -> Self {
            Self {
                name,
                available: true,
                should_fail: false,
                fail_error: None,
                delay_ms: None,
                call_count: Arc::new(AtomicU32::new(0)),
            }
        }

        fn available(mut self, available: bool) -> Self {
            self.available = available;
            self
        }

        fn failing(mut self) -> Self {
            self.should_fail = true;
            self
        }

        fn with_error(mut self, error: TranscriptionError) -> Self {
            self.should_fail = true;
            self.fail_error = Some(error);
            self
        }

        fn with_delay(mut self, delay_ms: u64) -> Self {
            self.delay_ms = Some(delay_ms);
            self
        }

        fn with_call_counter(mut self, counter: Arc<AtomicU32>) -> Self {
            self.call_count = counter;
            self
        }
    }

    #[async_trait]
    impl TranscriptionProvider for MockProvider {
        async fn transcribe(
            &self,
            _audio: &[f32],
            _config: &TranscriptionConfig,
        ) -> Result<TranscriptionResult, TranscriptionError> {
            self.call_count.fetch_add(1, Ordering::SeqCst);

            // Simulate processing delay if configured
            if let Some(delay) = self.delay_ms {
                tokio::time::sleep(Duration::from_millis(delay)).await;
            }

            if self.should_fail {
                Err(self.fail_error.clone().unwrap_or_else(|| {
                    TranscriptionError::Failed("Mock failure".to_string())
                }))
            } else {
                Ok(TranscriptionResult {
                    text: "Hello world".to_string(),
                    language: Some("en".to_string()),
                    duration_ms: 100,
                    provider: self.name.to_string(),
                })
            }
        }

        fn name(&self) -> &'static str {
            self.name
        }

        fn is_available(&self) -> bool {
            self.available
        }
    }

    // ============================================================
    // Provider Selection Tests
    // ============================================================

    #[tokio::test]
    async fn test_primary_success() {
        let orchestrator = TranscriptionOrchestrator::new(Box::new(MockProvider::new("primary")));

        let result = orchestrator
            .transcribe(&[0.0; 100], &TranscriptionConfig::default())
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().provider, "primary");
    }

    #[tokio::test]
    async fn test_selects_local_provider_when_configured() {
        // Simulates selecting local Whisper provider
        let orchestrator = TranscriptionOrchestrator::new(
            Box::new(MockProvider::new("whisper.cpp"))
        );

        assert_eq!(orchestrator.primary_provider(), "whisper.cpp");
        assert!(orchestrator.is_primary_available());
    }

    #[tokio::test]
    async fn test_selects_cloud_provider_when_configured() {
        // Simulates selecting Groq cloud provider
        let orchestrator = TranscriptionOrchestrator::new(
            Box::new(MockProvider::new("groq"))
        );

        assert_eq!(orchestrator.primary_provider(), "groq");
        assert!(orchestrator.is_primary_available());
    }

    #[tokio::test]
    async fn test_provider_availability_check() {
        let orchestrator = TranscriptionOrchestrator::new(
            Box::new(MockProvider::new("primary").available(false))
        );

        assert!(!orchestrator.is_primary_available());
    }

    // ============================================================
    // Error Handling Tests
    // ============================================================

    #[tokio::test]
    async fn test_empty_audio_returns_error() {
        let orchestrator = TranscriptionOrchestrator::new(Box::new(MockProvider::new("primary")));

        let result = orchestrator
            .transcribe(&[], &TranscriptionConfig::default())
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            TranscriptionError::InvalidAudio(msg) => assert!(msg.contains("Empty")),
            _ => panic!("Expected InvalidAudio error"),
        }
    }

    #[tokio::test]
    async fn test_model_not_loaded_error() {
        let orchestrator = TranscriptionOrchestrator::new(
            Box::new(MockProvider::new("primary").with_error(TranscriptionError::ModelNotLoaded))
        );

        let result = orchestrator
            .transcribe(&[0.0; 100], &TranscriptionConfig::default())
            .await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TranscriptionError::ModelNotLoaded));
    }

    #[tokio::test]
    async fn test_model_not_found_error() {
        let orchestrator = TranscriptionOrchestrator::new(
            Box::new(MockProvider::new("primary").with_error(
                TranscriptionError::ModelNotFound("tiny.bin".to_string())
            ))
        );

        let result = orchestrator
            .transcribe(&[0.0; 100], &TranscriptionConfig::default())
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            TranscriptionError::ModelNotFound(path) => assert_eq!(path, "tiny.bin"),
            _ => panic!("Expected ModelNotFound error"),
        }
    }

    #[tokio::test]
    async fn test_api_error_propagation() {
        let orchestrator = TranscriptionOrchestrator::new(
            Box::new(MockProvider::new("groq").with_error(
                TranscriptionError::ApiError("Invalid API key".to_string())
            ))
        );

        let result = orchestrator
            .transcribe(&[0.0; 100], &TranscriptionConfig::default())
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            TranscriptionError::ApiError(msg) => assert!(msg.contains("Invalid API key")),
            _ => panic!("Expected ApiError"),
        }
    }

    #[tokio::test]
    async fn test_network_error_propagation() {
        let orchestrator = TranscriptionOrchestrator::new(
            Box::new(MockProvider::new("groq").with_error(
                TranscriptionError::NetworkError("Connection refused".to_string())
            ))
        );

        let result = orchestrator
            .transcribe(&[0.0; 100], &TranscriptionConfig::default())
            .await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TranscriptionError::NetworkError(_)));
    }

    #[tokio::test]
    async fn test_rate_limited_error() {
        let orchestrator = TranscriptionOrchestrator::new(
            Box::new(MockProvider::new("groq").with_error(TranscriptionError::RateLimited))
        );

        let result = orchestrator
            .transcribe(&[0.0; 100], &TranscriptionConfig::default())
            .await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TranscriptionError::RateLimited));
    }

    // ============================================================
    // Fallback Tests
    // ============================================================

    #[tokio::test]
    async fn test_fallback_on_failure() {
        let orchestrator = TranscriptionOrchestrator::with_fallback(
            Box::new(MockProvider::new("primary").failing()),
            Box::new(MockProvider::new("fallback")),
        );

        let result = orchestrator
            .transcribe(&[0.0; 100], &TranscriptionConfig::default())
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().provider, "fallback");
    }

    #[tokio::test]
    async fn test_no_fallback_when_primary_succeeds() {
        let primary_calls = Arc::new(AtomicU32::new(0));
        let fallback_calls = Arc::new(AtomicU32::new(0));

        let orchestrator = TranscriptionOrchestrator::with_fallback(
            Box::new(MockProvider::new("primary").with_call_counter(primary_calls.clone())),
            Box::new(MockProvider::new("fallback").with_call_counter(fallback_calls.clone())),
        );

        let result = orchestrator
            .transcribe(&[0.0; 100], &TranscriptionConfig::default())
            .await;

        assert!(result.is_ok());
        assert_eq!(primary_calls.load(Ordering::SeqCst), 1);
        assert_eq!(fallback_calls.load(Ordering::SeqCst), 0); // Fallback never called
    }

    #[tokio::test]
    async fn test_fallback_skipped_when_unavailable() {
        let orchestrator = TranscriptionOrchestrator::with_fallback(
            Box::new(MockProvider::new("primary").failing()),
            Box::new(MockProvider::new("fallback").available(false)),
        );

        let result = orchestrator
            .transcribe(&[0.0; 100], &TranscriptionConfig::default())
            .await;

        // Should fail because fallback is not available
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_both_providers_fail() {
        let orchestrator = TranscriptionOrchestrator::with_fallback(
            Box::new(MockProvider::new("primary").failing()),
            Box::new(MockProvider::new("fallback").failing()),
        );

        let result = orchestrator
            .transcribe(&[0.0; 100], &TranscriptionConfig::default())
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_fallback_provider_names() {
        let orchestrator = TranscriptionOrchestrator::with_fallback(
            Box::new(MockProvider::new("whisper.cpp")),
            Box::new(MockProvider::new("groq")),
        );

        assert_eq!(orchestrator.primary_provider(), "whisper.cpp");
        assert_eq!(orchestrator.fallback_provider(), Some("groq"));
    }

    #[tokio::test]
    async fn test_no_fallback_configured() {
        let orchestrator = TranscriptionOrchestrator::new(
            Box::new(MockProvider::new("primary"))
        );

        assert!(orchestrator.fallback_provider().is_none());
        assert!(!orchestrator.is_fallback_available());
    }

    #[tokio::test]
    async fn test_fallback_on_timeout_error() {
        let orchestrator = TranscriptionOrchestrator::with_fallback(
            Box::new(MockProvider::new("primary").with_error(TranscriptionError::Timeout(30))),
            Box::new(MockProvider::new("fallback")),
        );

        let result = orchestrator
            .transcribe(&[0.0; 100], &TranscriptionConfig::default())
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().provider, "fallback");
    }

    #[tokio::test]
    async fn test_fallback_on_network_error() {
        let orchestrator = TranscriptionOrchestrator::with_fallback(
            Box::new(MockProvider::new("groq").with_error(
                TranscriptionError::NetworkError("DNS lookup failed".to_string())
            )),
            Box::new(MockProvider::new("whisper.cpp")),
        );

        let result = orchestrator
            .transcribe(&[0.0; 100], &TranscriptionConfig::default())
            .await;

        // Should fallback to local whisper on network error
        assert!(result.is_ok());
        assert_eq!(result.unwrap().provider, "whisper.cpp");
    }

    // ============================================================
    // Timeout Handling Tests
    // ============================================================

    #[tokio::test]
    async fn test_timeout_error_type() {
        let orchestrator = TranscriptionOrchestrator::new(
            Box::new(MockProvider::new("primary").with_error(TranscriptionError::Timeout(60)))
        );

        let result = orchestrator
            .transcribe(&[0.0; 100], &TranscriptionConfig::default())
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            TranscriptionError::Timeout(secs) => assert_eq!(secs, 60),
            _ => panic!("Expected Timeout error"),
        }
    }

    // ============================================================
    // Configuration Tests
    // ============================================================

    #[tokio::test]
    async fn test_transcription_with_language_config() {
        let orchestrator = TranscriptionOrchestrator::new(Box::new(MockProvider::new("primary")));

        let config = TranscriptionConfig {
            language: "fr".to_string(),
            translate: false,
        };

        let result = orchestrator.transcribe(&[0.0; 100], &config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_transcription_with_translate() {
        let orchestrator = TranscriptionOrchestrator::new(Box::new(MockProvider::new("primary")));

        let config = TranscriptionConfig {
            language: "auto".to_string(),
            translate: true,
        };

        let result = orchestrator.transcribe(&[0.0; 100], &config).await;
        assert!(result.is_ok());
    }

    // ============================================================
    // Edge Cases
    // ============================================================

    #[tokio::test]
    async fn test_very_short_audio() {
        let orchestrator = TranscriptionOrchestrator::new(Box::new(MockProvider::new("primary")));

        // Single sample
        let result = orchestrator
            .transcribe(&[0.0], &TranscriptionConfig::default())
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_large_audio_buffer() {
        let orchestrator = TranscriptionOrchestrator::new(Box::new(MockProvider::new("primary")));

        // 1 minute of audio at 16kHz
        let large_audio = vec![0.0f32; 16000 * 60];
        let result = orchestrator
            .transcribe(&large_audio, &TranscriptionConfig::default())
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_audio_with_nan_values() {
        // This tests that the mock provider handles NaN gracefully
        // Real providers should validate this
        let orchestrator = TranscriptionOrchestrator::new(Box::new(MockProvider::new("primary")));

        let audio_with_nan = vec![0.0, f32::NAN, 0.5, f32::NAN];
        let result = orchestrator
            .transcribe(&audio_with_nan, &TranscriptionConfig::default())
            .await;

        // Mock provider accepts it; real provider may reject
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_audio_with_infinity() {
        let orchestrator = TranscriptionOrchestrator::new(Box::new(MockProvider::new("primary")));

        let audio_with_inf = vec![0.0, f32::INFINITY, -f32::INFINITY, 0.5];
        let result = orchestrator
            .transcribe(&audio_with_inf, &TranscriptionConfig::default())
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_audio_with_extreme_values() {
        let orchestrator = TranscriptionOrchestrator::new(Box::new(MockProvider::new("primary")));

        // Audio values should normally be in [-1.0, 1.0] range
        let extreme_audio = vec![f32::MAX, f32::MIN, 0.0, -1.0, 1.0];
        let result = orchestrator
            .transcribe(&extreme_audio, &TranscriptionConfig::default())
            .await;

        assert!(result.is_ok());
    }
}
