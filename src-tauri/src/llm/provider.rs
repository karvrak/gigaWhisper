//! LLM Provider Trait

use async_trait::async_trait;

/// Request for LLM completion
#[derive(Debug, Clone)]
pub struct LlmRequest {
    pub system_prompt: String,
    pub user_message: String,
    pub max_tokens: u32,
}

/// Response from LLM completion
#[derive(Debug, Clone)]
pub struct LlmResponse {
    pub text: String,
    pub tokens_used: u64,
    pub provider: String,
}

/// LLM errors
#[derive(Debug, Clone, thiserror::Error)]
pub enum LlmError {
    #[error("API error: {0}")]
    ApiError(String),
    #[error("Network error: {0}")]
    NetworkError(String),
    #[error("Rate limited")]
    RateLimited,
    #[error("Timeout")]
    Timeout,
    #[error("Failed: {0}")]
    Failed(String),
}

/// Trait for LLM providers
#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn complete(&self, request: &LlmRequest) -> Result<LlmResponse, LlmError>;
    fn name(&self) -> &'static str;
    fn is_available(&self) -> bool;
}
