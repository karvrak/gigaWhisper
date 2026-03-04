//! Premium Commands
//!
//! Tauri commands for license management and premium features.

use crate::AppState;
use crate::licensing::{FeatureGate, LicenseManager, PremiumFeature};
use serde::Serialize;

/// Premium status response sent to frontend
#[derive(Debug, Clone, Serialize)]
pub struct PremiumStatus {
    pub is_premium: bool,
    pub expires_at: Option<u64>,
    pub last_validated: Option<u64>,
    pub credits_balance: f64,
    pub credits_low: bool,
    pub features: Vec<FeatureStatus>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FeatureStatus {
    pub id: String,
    pub name: String,
    pub available: bool,
}

/// Activate a premium license
#[tauri::command]
pub async fn activate_license(
    key: String,
    state: tauri::State<'_, AppState>,
) -> Result<PremiumStatus, String> {
    // Validate license key input
    let key = key.trim().to_string();
    if key.is_empty() {
        return Err("License key cannot be empty".to_string());
    }
    if key.len() > 256 {
        return Err("License key is too long".to_string());
    }
    if !key.chars().all(|c| c.is_alphanumeric() || c == '-') {
        return Err("License key contains invalid characters (only alphanumeric and dashes allowed)".to_string());
    }

    let mut manager = LicenseManager::new();

    // Restore existing license key from credential manager if available
    if let Ok(Some(stored_key)) = LicenseManager::get_stored_license_key() {
        let mut restored_state = crate::licensing::LicenseState::default();
        restored_state.license_key = Some(stored_key);
        manager.restore_state(restored_state);
    }

    let activation = manager.activate(&key).await.map_err(|e| e.to_string())?;

    // Update settings
    {
        let mut config = state.config.write();
        config.premium.is_premium = activation.valid;
        config.premium.expires_at = activation.expires_at;
        config.premium.last_validated = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        );
        config.save().map_err(|e| e.to_string())?;
    }

    get_premium_status(state).await
}

/// Deactivate the premium license
#[tauri::command]
pub async fn deactivate_license(
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let mut manager = LicenseManager::new();

    // Restore license key from credential manager so the server can be notified
    if let Ok(Some(stored_key)) = LicenseManager::get_stored_license_key() {
        let mut restored_state = crate::licensing::LicenseState::default();
        restored_state.license_key = Some(stored_key);
        manager.restore_state(restored_state);
    }

    manager.deactivate().await.map_err(|e| e.to_string())?;

    // Update settings
    {
        let mut config = state.config.write();
        config.premium.is_premium = false;
        config.premium.expires_at = None;
        config.premium.last_validated = None;
        config.save().map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// Get current premium status
#[tauri::command]
pub async fn get_premium_status(
    state: tauri::State<'_, AppState>,
) -> Result<PremiumStatus, String> {
    let config = state.config.read().clone();
    let is_premium = config.premium.is_premium;

    let features: Vec<FeatureStatus> = PremiumFeature::all()
        .iter()
        .map(|f| FeatureStatus {
            id: format!("{:?}", f).to_lowercase().replace(' ', "-"),
            name: f.display_name().to_string(),
            available: FeatureGate::is_available(*f, is_premium),
        })
        .collect();

    Ok(PremiumStatus {
        is_premium,
        expires_at: config.premium.expires_at,
        last_validated: config.premium.last_validated,
        credits_balance: config.premium.credits.balance_eur,
        credits_low: config.premium.credits.balance_eur <= config.premium.credits.low_threshold_eur,
        features,
    })
}

/// Check if a specific feature is available
#[tauri::command]
pub async fn check_feature(
    feature: String,
    state: tauri::State<'_, AppState>,
) -> Result<bool, String> {
    let config = state.config.read();
    let is_premium = config.premium.is_premium;

    let premium_feature = match feature.as_str() {
        "custom-themes" | "CustomThemes" => PremiumFeature::CustomThemes,
        "multi-context" | "MultiContext" => PremiumFeature::MultiContext,
        "llm-post-processing" | "LlmPostProcessing" => PremiumFeature::LlmPostProcessing,
        "openai-provider" | "OpenAiProvider" => PremiumFeature::OpenAiProvider,
        "deepgram-provider" | "DeepgramProvider" => PremiumFeature::DeepgramProvider,
        _ => return Err(format!("Unknown feature: {}", feature)),
    };

    Ok(FeatureGate::is_available(premium_feature, is_premium))
}

/// Get credit balance
#[tauri::command]
pub async fn get_credits_balance(
    state: tauri::State<'_, AppState>,
) -> Result<f64, String> {
    let config = state.config.read();
    Ok(config.premium.credits.balance_eur)
}
