//! Feature Gating
//!
//! Controls access to premium features.

use serde::{Deserialize, Serialize};

/// Premium features that require a license
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PremiumFeature {
    /// Custom themes beyond Light/Dark/System
    CustomThemes,
    /// Multiple transcription contexts with separate shortcuts
    MultiContext,
    /// LLM post-processing of transcriptions
    LlmPostProcessing,
    /// OpenAI Whisper cloud provider
    OpenAiProvider,
    /// Deepgram cloud provider
    DeepgramProvider,
    /// Custom endpoint provider (transcription + LLM)
    CustomProvider,
}

impl PremiumFeature {
    /// Get display name
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::CustomThemes => "Custom Themes",
            Self::MultiContext => "Multi-Context Shortcuts",
            Self::LlmPostProcessing => "AI Post-Processing",
            Self::OpenAiProvider => "OpenAI Whisper",
            Self::DeepgramProvider => "Deepgram Nova",
            Self::CustomProvider => "Custom Endpoint",
        }
    }

    /// Get all premium features
    pub fn all() -> &'static [PremiumFeature] {
        &[
            Self::CustomThemes,
            Self::MultiContext,
            Self::LlmPostProcessing,
            Self::OpenAiProvider,
            Self::DeepgramProvider,
            Self::CustomProvider,
        ]
    }
}

/// Feature gate that checks license status
pub struct FeatureGate;

impl FeatureGate {
    /// Check if a premium feature is available
    /// Returns true if user has premium license (active or in grace period)
    pub fn is_available(_feature: PremiumFeature, is_premium: bool) -> bool {
        is_premium
    }

    /// Check if user can use own API key for a provider (always allowed)
    pub fn can_use_own_api_key() -> bool {
        true
    }
}
