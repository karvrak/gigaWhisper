//! OpenAI LLM Provider (GPT-4o-mini)

use super::{LlmError, LlmProvider, LlmRequest, LlmResponse};
use crate::config::SecretsManager;
use async_trait::async_trait;

const OPENAI_CHAT_URL: &str = "https://api.openai.com/v1/chat/completions";

pub struct OpenAiLlm {
    model: String,
    client: reqwest::Client,
}

impl OpenAiLlm {
    pub fn new(model: Option<String>) -> Self {
        Self {
            model: model.unwrap_or_else(|| "gpt-4o-mini".to_string()),
            client: reqwest::Client::new(),
        }
    }

    pub fn with_client(model: Option<String>, client: reqwest::Client) -> Self {
        Self {
            model: model.unwrap_or_else(|| "gpt-4o-mini".to_string()),
            client,
        }
    }

    fn get_api_key(&self) -> Option<String> {
        SecretsManager::get_secret("openai_api_key").ok()
    }
}

#[async_trait]
impl LlmProvider for OpenAiLlm {
    async fn complete(&self, request: &LlmRequest) -> Result<LlmResponse, LlmError> {
        let api_key = self
            .get_api_key()
            .ok_or_else(|| LlmError::ApiError("OpenAI API key not configured".to_string()))?;

        let body = serde_json::json!({
            "model": self.model,
            "messages": [
                { "role": "system", "content": request.system_prompt },
                { "role": "user", "content": request.user_message }
            ],
            "max_tokens": request.max_tokens,
            "temperature": 0.3
        });

        let response = self
            .client
            .post(OPENAI_CHAT_URL)
            .bearer_auth(&api_key)
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

        let text = result["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let tokens_used = result["usage"]["total_tokens"].as_u64().unwrap_or(0);

        Ok(LlmResponse {
            text,
            tokens_used,
            provider: "openai".to_string(),
        })
    }

    fn name(&self) -> &'static str {
        "openai"
    }

    fn is_available(&self) -> bool {
        self.get_api_key().is_some()
    }
}
