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

    struct MockProvider {
        name: &'static str,
        available: bool,
        should_fail: bool,
    }

    #[async_trait]
    impl TranscriptionProvider for MockProvider {
        async fn transcribe(
            &self,
            _audio: &[f32],
            _config: &TranscriptionConfig,
        ) -> Result<TranscriptionResult, TranscriptionError> {
            if self.should_fail {
                Err(TranscriptionError::Failed("Mock failure".to_string()))
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

    #[tokio::test]
    async fn test_primary_success() {
        let orchestrator = TranscriptionOrchestrator::new(Box::new(MockProvider {
            name: "primary",
            available: true,
            should_fail: false,
        }));

        let result = orchestrator
            .transcribe(&[0.0; 100], &TranscriptionConfig::default())
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().provider, "primary");
    }

    #[tokio::test]
    async fn test_fallback_on_failure() {
        let orchestrator = TranscriptionOrchestrator::with_fallback(
            Box::new(MockProvider {
                name: "primary",
                available: true,
                should_fail: true,
            }),
            Box::new(MockProvider {
                name: "fallback",
                available: true,
                should_fail: false,
            }),
        );

        let result = orchestrator
            .transcribe(&[0.0; 100], &TranscriptionConfig::default())
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().provider, "fallback");
    }
}
