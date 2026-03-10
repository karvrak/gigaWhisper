//! Custom LLM Provider
//!
//! OpenAI-compatible chat completions using a user-configured endpoint.

use super::{LlmError, LlmProvider, LlmRequest, LlmResponse};
use crate::config::AuthType;
use crate::config::SecretsManager;
use async_trait::async_trait;

pub struct CustomLlm {
    api_url: String,
    model: String,
    client: reqwest::Client,
    auth_type: AuthType,
    custom_header_name: String,
}

impl CustomLlm {
    pub fn new(
        api_url: String,
        model: String,
        timeout_seconds: u64,
        auth_type: AuthType,
        custom_header_name: String,
        accept_invalid_certs: bool,
    ) -> Self {
        let timeout = std::time::Duration::from_secs(if timeout_seconds == 0 {
            30
        } else {
            timeout_seconds
        });

        let client = reqwest::Client::builder()
            .timeout(timeout)
            .danger_accept_invalid_certs(accept_invalid_certs)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            api_url,
            model,
            client,
            auth_type,
            custom_header_name,
        }
    }

    fn get_api_key(&self) -> Option<String> {
        SecretsManager::get_custom_llm_api_key().ok()
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
}

#[async_trait]
impl LlmProvider for CustomLlm {
    async fn complete(&self, request: &LlmRequest) -> Result<LlmResponse, LlmError> {
        let api_key = self
            .get_api_key()
            .ok_or_else(|| LlmError::ApiError("Custom LLM API key not configured".to_string()))?;

        if self.api_url.is_empty() {
            return Err(LlmError::ApiError(
                "Custom LLM API URL not configured".to_string(),
            ));
        }

        let body = serde_json::json!({
            "model": self.model,
            "messages": [
                { "role": "system", "content": request.system_prompt },
                { "role": "user", "content": request.user_message }
            ],
            "max_tokens": request.max_tokens,
            "temperature": 0.3
        });

        let req = self.client.post(&self.api_url).json(&body);
        let req = self.apply_auth(req, &api_key);

        let response = req
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
            provider: "custom".to_string(),
        })
    }

    fn name(&self) -> &'static str {
        "custom"
    }

    fn is_available(&self) -> bool {
        self.get_api_key().is_some() && !self.api_url.is_empty()
    }
}
