//! Deepgram API Provider
//!
//! Cloud transcription using Deepgram's Nova-2 API.

use super::{TranscriptionConfig, TranscriptionError, TranscriptionResult};
use crate::audio::encode_wav;
use crate::config::SecretsManager;
use async_trait::async_trait;
use std::time::{Duration, Instant};

const DEEPGRAM_API_URL: &str = "https://api.deepgram.com/v1/listen";
const DEFAULT_TIMEOUT_SECONDS: u64 = 30;
const DEFAULT_MAX_RETRIES: u32 = 3;
const RETRY_BASE_DELAY_MS: u64 = 1000;
const SECRET_NAME: &str = "deepgram_api_key";

/// Deepgram transcription provider
pub struct DeepgramProvider {
    model: String,
    client: reqwest::Client,
    max_retries: u32,
}

impl DeepgramProvider {
    pub fn new(model: Option<String>) -> Self {
        Self::with_timeout(model, DEFAULT_TIMEOUT_SECONDS)
    }

    pub fn with_timeout(model: Option<String>, timeout_seconds: u64) -> Self {
        let timeout = Duration::from_secs(timeout_seconds);
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            model: model.unwrap_or_else(|| "nova-2".to_string()),
            client,
            max_retries: DEFAULT_MAX_RETRIES,
        }
    }

    fn get_api_key(&self) -> Option<String> {
        SecretsManager::get_secret(SECRET_NAME).ok()
    }

    fn retry_delay(attempt: u32) -> Duration {
        let delay_ms = RETRY_BASE_DELAY_MS * 2u64.pow(attempt);
        Duration::from_millis(delay_ms.min(30_000))
    }
}

#[async_trait]
impl super::TranscriptionProvider for DeepgramProvider {
    async fn transcribe(
        &self,
        audio: &[f32],
        config: &TranscriptionConfig,
    ) -> Result<TranscriptionResult, TranscriptionError> {
        let api_key = self.get_api_key().ok_or_else(|| {
            TranscriptionError::ApiError(
                "Deepgram API key not configured. Please set your API key in settings.".to_string(),
            )
        })?;

        let start = Instant::now();
        let wav_data = encode_wav(audio, 16000, 1);
        let mut last_error: Option<TranscriptionError> = None;

        for attempt in 0..=self.max_retries {
            if attempt > 0 {
                let delay = Self::retry_delay(attempt - 1);
                tracing::info!(
                    "Retrying Deepgram API request (attempt {}/{})",
                    attempt + 1,
                    self.max_retries + 1
                );
                tokio::time::sleep(delay).await;
            }

            // Build URL with query parameters
            let mut url = format!(
                "{}?model={}&smart_format=true",
                DEEPGRAM_API_URL, self.model
            );
            if config.language != "auto" {
                url.push_str(&format!("&language={}", config.language));
            } else {
                url.push_str("&detect_language=true");
            }

            let response = match self
                .client
                .post(&url)
                .header("Authorization", format!("Token {}", api_key))
                .header("Content-Type", "audio/wav")
                .body(wav_data.clone())
                .send()
                .await
            {
                Ok(resp) => resp,
                Err(e) => {
                    tracing::warn!("Deepgram API network error: {}", e);
                    last_error = Some(TranscriptionError::NetworkError(e.to_string()));
                    continue;
                }
            };

            if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
                tracing::warn!("Deepgram API rate limited");
                last_error = Some(TranscriptionError::RateLimited);
                continue;
            }

            if response.status().is_server_error() {
                let error_text = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Server error".to_string());
                tracing::warn!("Deepgram API server error: {}", error_text);
                last_error = Some(TranscriptionError::NetworkError(error_text));
                continue;
            }

            if !response.status().is_success() {
                let error_text = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unknown error".to_string());
                return Err(TranscriptionError::ApiError(error_text));
            }

            let result: DeepgramResponse = match response.json().await {
                Ok(r) => r,
                Err(e) => {
                    tracing::warn!("Failed to parse Deepgram response: {}", e);
                    last_error = Some(TranscriptionError::Failed(e.to_string()));
                    continue;
                }
            };

            let duration_ms = start.elapsed().as_millis() as u64;

            let text = result
                .results
                .channels
                .first()
                .and_then(|ch| ch.alternatives.first())
                .map(|alt| alt.transcript.trim().to_string())
                .unwrap_or_default();

            let language = result
                .results
                .channels
                .first()
                .and_then(|ch| ch.detected_language.clone());

            return Ok(TranscriptionResult {
                text,
                language,
                duration_ms,
                provider: "deepgram".to_string(),
            });
        }

        Err(last_error.unwrap_or_else(|| {
            TranscriptionError::Failed("All retry attempts failed".to_string())
        }))
    }

    fn name(&self) -> &'static str {
        "deepgram"
    }

    fn is_available(&self) -> bool {
        self.get_api_key().is_some()
    }

    fn cost_per_minute(&self) -> Option<f64> {
        Some(0.0043) // $0.0043 per minute for Nova-2
    }
}

/// Deepgram API response structure
#[derive(serde::Deserialize)]
struct DeepgramResponse {
    results: DeepgramResults,
}

#[derive(serde::Deserialize)]
struct DeepgramResults {
    channels: Vec<DeepgramChannel>,
}

#[derive(serde::Deserialize)]
struct DeepgramChannel {
    alternatives: Vec<DeepgramAlternative>,
    detected_language: Option<String>,
}

#[derive(serde::Deserialize)]
struct DeepgramAlternative {
    transcript: String,
}
