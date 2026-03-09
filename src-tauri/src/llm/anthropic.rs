//! Anthropic LLM Provider (Claude Haiku)

use super::{LlmError, LlmProvider, LlmRequest, LlmResponse};
use crate::config::SecretsManager;
use async_trait::async_trait;

const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";

pub struct AnthropicLlm {
    model: String,
    client: reqwest::Client,
}

impl AnthropicLlm {
    pub fn new(model: Option<String>) -> Self {
        Self {
            model: model.unwrap_or_else(|| "claude-haiku-4-5-20251001".to_string()),
            client: reqwest::Client::new(),
        }
    }

    pub fn with_client(model: Option<String>, client: reqwest::Client) -> Self {
        Self {
            model: model.unwrap_or_else(|| "claude-haiku-4-5-20251001".to_string()),
            client,
        }
    }

    fn get_api_key(&self) -> Option<String> {
        SecretsManager::get_secret("anthropic_api_key").ok()
    }
}

#[async_trait]
impl LlmProvider for AnthropicLlm {
    async fn complete(&self, request: &LlmRequest) -> Result<LlmResponse, LlmError> {
        let api_key = self
            .get_api_key()
            .ok_or_else(|| LlmError::ApiError("Anthropic API key not configured".to_string()))?;

        let body = serde_json::json!({
            "model": self.model,
            "max_tokens": request.max_tokens,
            "system": request.system_prompt,
            "messages": [
                { "role": "user", "content": request.user_message }
            ]
        });

        let response = self
            .client
            .post(ANTHROPIC_API_URL)
            .header("x-api-key", &api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| LlmError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(LlmError::ApiError(error_text));
        }

        let result: serde_json::Value = response
            .json()
            .await
            .map_err(|e| LlmError::Failed(e.to_string()))?;

        let text = result["content"][0]["text"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let input_tokens = result["usage"]["input_tokens"].as_u64().unwrap_or(0);
        let output_tokens = result["usage"]["output_tokens"].as_u64().unwrap_or(0);

        Ok(LlmResponse {
            text,
            tokens_used: input_tokens + output_tokens,
            provider: "anthropic".to_string(),
        })
    }

    fn name(&self) -> &'static str {
        "anthropic"
    }

    fn is_available(&self) -> bool {
        self.get_api_key().is_some()
    }
}
