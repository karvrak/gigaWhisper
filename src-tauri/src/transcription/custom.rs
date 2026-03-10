//! Custom Endpoint Transcription Provider
//!
//! OpenAI-compatible transcription using a user-configured URL, auth, and model.

use super::{TranscriptionConfig, TranscriptionError, TranscriptionProvider, TranscriptionResult};
use crate::audio::encode_wav;
use crate::config::AuthType;
use crate::config::SecretsManager;
use async_trait::async_trait;
use std::time::{Duration, Instant};

const DEFAULT_TIMEOUT_SECONDS: u64 = 30;
const DEFAULT_MAX_RETRIES: u32 = 3;
const RETRY_BASE_DELAY_MS: u64 = 1000;

/// Custom endpoint transcription provider
pub struct CustomTranscriptionProvider {
    api_url: String,
    model: String,
    client: reqwest::Client,
    max_retries: u32,
    auth_type: AuthType,
    custom_header_name: String,
    api_key: Option<String>,
}

impl CustomTranscriptionProvider {
    pub fn new(
        api_url: String,
        model: String,
        timeout_seconds: u64,
        auth_type: AuthType,
        custom_header_name: String,
        accept_invalid_certs: bool,
    ) -> Self {
        let timeout = Duration::from_secs(if timeout_seconds == 0 {
            DEFAULT_TIMEOUT_SECONDS
        } else {
            timeout_seconds
        });

        let client = reqwest::Client::builder()
            .timeout(timeout)
            .danger_accept_invalid_certs(accept_invalid_certs)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        let api_key = SecretsManager::get_custom_transcription_api_key().ok();

        Self {
            api_url,
            model,
            client,
            max_retries: DEFAULT_MAX_RETRIES,
            auth_type,
            custom_header_name,
            api_key,
        }
    }

    fn apply_auth(
        &self,
        request: reqwest::RequestBuilder,
        api_key: &str,
    ) -> reqwest::RequestBuilder {
        match &self.auth_type {
            AuthType::Bearer => request.bearer_auth(api_key),
            AuthType::XApiKey => request.header("x-api-key", api_key),
            AuthType::Custom => request.header(&self.custom_header_name, api_key),
        }
    }

    fn retry_delay(attempt: u32) -> Duration {
        let delay_ms = RETRY_BASE_DELAY_MS * 2u64.pow(attempt);
        Duration::from_millis(delay_ms.min(30_000))
    }
}

#[async_trait]
impl TranscriptionProvider for CustomTranscriptionProvider {
    async fn transcribe(
        &self,
        audio: &[f32],
        config: &TranscriptionConfig,
    ) -> Result<TranscriptionResult, TranscriptionError> {
        let api_key = self.api_key.clone().ok_or_else(|| {
            TranscriptionError::ApiError("Custom transcription API key not configured.".to_string())
        })?;

        if self.api_url.is_empty() {
            return Err(TranscriptionError::ApiError(
                "Custom transcription API URL not configured.".to_string(),
            ));
        }

        let start = Instant::now();
        let wav_data = encode_wav(audio, 16000, 1);
        let wav_bytes = bytes::Bytes::from(wav_data);
        let mut last_error: Option<TranscriptionError> = None;

        for attempt in 0..=self.max_retries {
            if attempt > 0 {
                let delay = Self::retry_delay(attempt - 1);
                tracing::info!(
                    "Retrying custom transcription API (attempt {}/{})",
                    attempt + 1,
                    self.max_retries + 1
                );
                tokio::time::sleep(delay).await;
            }

            let file_part = match reqwest::multipart::Part::stream(wav_bytes.clone())
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

            if let Some(ref prompt) = config.prompt {
                if !prompt.is_empty() {
                    form = form.text("prompt", prompt.clone());
                }
            }

            let request = self.client.post(&self.api_url).multipart(form);
            let request = self.apply_auth(request, &api_key);

            let response = match request.send().await {
                Ok(resp) => resp,
                Err(e) => {
                    tracing::warn!("Custom transcription API network error: {}", e);
                    last_error = Some(TranscriptionError::NetworkError(e.to_string()));
                    continue;
                }
            };

            if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
                tracing::warn!("Custom transcription API rate limited");
                last_error = Some(TranscriptionError::RateLimited);
                continue;
            }

            if response.status().is_server_error() {
                let error_text = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Server error".to_string());
                tracing::warn!("Custom transcription API server error: {}", error_text);
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

            let result: serde_json::Value = match response.json().await {
                Ok(r) => r,
                Err(e) => {
                    tracing::warn!("Failed to parse custom transcription response: {}", e);
                    last_error = Some(TranscriptionError::Failed(e.to_string()));
                    continue;
                }
            };

            // Standard OpenAI-compatible response: {"text": "..."}
            let text = result["text"].as_str().unwrap_or("").trim().to_string();

            let duration_ms = start.elapsed().as_millis() as u64;

            return Ok(TranscriptionResult {
                text,
                language: None,
                duration_ms,
                provider: "custom".to_string(),
            });
        }

        Err(last_error
            .unwrap_or_else(|| TranscriptionError::Failed("All retry attempts failed".to_string())))
    }

    fn name(&self) -> &'static str {
        "custom"
    }

    fn is_available(&self) -> bool {
        self.api_key.is_some() && !self.api_url.is_empty()
    }

    fn cost_per_minute(&self) -> Option<f64> {
        None
    }
}
