//! Credits System
//!
//! Track cloud API usage and credit balance.

use serde::{Deserialize, Serialize};

/// Estimated costs per minute for different providers (in EUR)
const OPENAI_COST_PER_MINUTE: f64 = 0.006;
const DEEPGRAM_COST_PER_MINUTE: f64 = 0.0043;
const GROQ_COST_PER_MINUTE: f64 = 0.0; // Free tier
const LLM_COST_PER_1K_TOKENS: f64 = 0.00015; // GPT-4o-mini pricing

/// Credit balance and usage tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreditState {
    /// Current balance in EUR (cached from server)
    pub balance_eur: f64,
    /// Total credits used (lifetime)
    pub total_used_eur: f64,
    /// Pending offline deductions not yet synced
    pub pending_deductions: Vec<CreditDeduction>,
    /// Low balance threshold in EUR
    pub low_threshold_eur: f64,
    /// Last sync timestamp
    pub last_synced: Option<u64>,
}

impl Default for CreditState {
    fn default() -> Self {
        Self {
            balance_eur: 0.0,
            total_used_eur: 0.0,
            pending_deductions: Vec::new(),
            low_threshold_eur: 5.0,
            last_synced: None,
        }
    }
}

/// A single credit deduction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreditDeduction {
    /// Timestamp of the deduction
    pub timestamp: u64,
    /// Amount in EUR
    pub amount_eur: f64,
    /// Description
    pub description: String,
    /// Provider used
    pub provider: String,
}

/// Credit tracker for pre/post-flight checks
pub struct CreditTracker;

impl CreditTracker {
    /// Estimate cost for a transcription request
    pub fn estimate_transcription_cost(provider: &str, audio_duration_seconds: f64) -> f64 {
        let minutes = audio_duration_seconds / 60.0;
        match provider {
            "openai" => minutes * OPENAI_COST_PER_MINUTE,
            "deepgram" => minutes * DEEPGRAM_COST_PER_MINUTE,
            "groq" => minutes * GROQ_COST_PER_MINUTE,
            _ => 0.0,
        }
    }

    /// Estimate cost for LLM post-processing
    pub fn estimate_llm_cost(estimated_tokens: u64) -> f64 {
        (estimated_tokens as f64 / 1000.0) * LLM_COST_PER_1K_TOKENS
    }

    /// Check if user has enough credits for estimated cost
    pub fn has_sufficient_credits(state: &CreditState, estimated_cost: f64) -> bool {
        state.balance_eur >= estimated_cost
    }

    /// Check if balance is below threshold
    pub fn is_low_balance(state: &CreditState) -> bool {
        state.balance_eur <= state.low_threshold_eur
    }

    /// Record a deduction (offline queue)
    pub fn record_deduction(
        state: &mut CreditState,
        amount_eur: f64,
        description: String,
        provider: String,
    ) {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        state.balance_eur = (state.balance_eur - amount_eur).max(0.0);
        state.total_used_eur += amount_eur;
        state.pending_deductions.push(CreditDeduction {
            timestamp,
            amount_eur,
            description,
            provider,
        });
    }
}
