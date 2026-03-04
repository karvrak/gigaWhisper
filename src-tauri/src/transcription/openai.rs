//! OpenAI Whisper API Provider
//!
//! Cloud transcription using OpenAI's Whisper API.

use super::{TranscriptionConfig, TranscriptionError, TranscriptionResult};
use crate::audio::encode_wav;
use crate::config::SecretsManager;
use async_trait::async_trait;
use std::time::{Duration, Instant};

const OPENAI_API_URL: &str = "https://api.openai.com/v1/audio/transcriptions";
const DEFAULT_TIMEOUT_SECONDS: u64 = 30;
const DEFAULT_MAX_RETRIES: u32 = 3;
const RETRY_BASE_DELAY_MS: u64 = 1000;
const SECRET_NAME: &str = "openai_api_key";

/// OpenAI Whisper transcription provider
pub struct OpenAiProvider {
    model: String,
    client: reqwest::Client,
    max_retries: u32,
}

impl OpenAiProvider {
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
            model: model.unwrap_or_else(|| "whisper-1".to_string()),
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
impl super::TranscriptionProvider for OpenAiProvider {
    async fn transcribe(
        &self,
        audio: &[f32],
        config: &TranscriptionConfig,
    ) -> Result<TranscriptionResult, TranscriptionError> {
        let api_key = self.get_api_key().ok_or_else(|| {
            TranscriptionError::ApiError(
                "OpenAI API key not configured. Please set your API key in settings.".to_string(),
            )
        })?;

        let start = Instant::now();
        let wav_data = encode_wav(audio, 16000, 1);
        let mut last_error: Option<TranscriptionError> = None;

        for attempt in 0..=self.max_retries {
            if attempt > 0 {
                let delay = Self::retry_delay(attempt - 1);
                tracing::info!(
                    "Retrying OpenAI API request (attempt {}/{})",
                    attempt + 1,
                    self.max_retries + 1
                );
                tokio::time::sleep(delay).await;
            }

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

            if config.language != "auto" {
                form = form.text("language", config.language.clone());
            }

            let response = match self
                .client
                .post(OPENAI_API_URL)
                .bearer_auth(&api_key)
                .multipart(form)
                .send()
                .await
            {
                Ok(resp) => resp,
                Err(e) => {
                    tracing::warn!("OpenAI API network error: {}", e);
                    last_error = Some(TranscriptionError::NetworkError(e.to_string()));
                    continue;
                }
            };

            if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
                tracing::warn!("OpenAI API rate limited");
                last_error = Some(TranscriptionError::RateLimited);
                continue;
            }

            if response.status().is_server_error() {
                let error_text = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Server error".to_string());
                tracing::warn!("OpenAI API server error: {}", error_text);
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

            let result: OpenAiResponse = match response.json().await {
                Ok(r) => r,
                Err(e) => {
                    tracing::warn!("Failed to parse OpenAI response: {}", e);
                    last_error = Some(TranscriptionError::Failed(e.to_string()));
                    continue;
                }
            };

            let duration_ms = start.elapsed().as_millis() as u64;

            return Ok(TranscriptionResult {
                text: result.text.trim().to_string(),
                language: None,
                duration_ms,
                provider: "openai".to_string(),
            });
        }

        Err(last_error
            .unwrap_or_else(|| TranscriptionError::Failed("All retry attempts failed".to_string())))
    }

    fn name(&self) -> &'static str {
        "openai"
    }

    fn is_available(&self) -> bool {
        self.get_api_key().is_some()
    }

    fn cost_per_minute(&self) -> Option<f64> {
        Some(0.006) // $0.006 per minute
    }
}

#[derive(serde::Deserialize)]
struct OpenAiResponse {
    text: String,
}
