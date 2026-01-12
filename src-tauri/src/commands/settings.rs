//! Settings Commands
//!
//! Handle configuration read/write operations.

use crate::config::{SecretsManager, Settings};
use crate::AppState;
use tauri::State;

/// Get current settings
#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Settings {
    state.config.read().clone()
}

/// Save settings
#[tauri::command]
pub async fn save_settings(
    state: State<'_, AppState>,
    settings: Settings,
) -> Result<(), String> {
    tracing::info!("Saving settings");

    // Validate settings
    settings.validate().map_err(|e| e.to_string())?;

    // Update in-memory state
    {
        let mut config = state.config.write();
        *config = settings.clone();
    }

    // Persist to disk
    settings.save().map_err(|e| e.to_string())?;

    tracing::info!("Settings saved successfully");
    Ok(())
}

/// Get available audio input devices
#[tauri::command]
pub fn get_audio_devices() -> Result<Vec<AudioDeviceDto>, String> {
    use crate::audio::AudioCapture;

    match AudioCapture::list_devices() {
        Ok(devices) => Ok(devices
            .into_iter()
            .map(|d| AudioDeviceDto {
                id: d.id,
                name: d.name,
                is_default: d.is_default,
            })
            .collect()),
        Err(e) => {
            tracing::warn!("Failed to list audio devices: {}", e);
            // Return default device as fallback
            Ok(vec![AudioDeviceDto {
                id: "default".to_string(),
                name: "Default Microphone".to_string(),
                is_default: true,
            }])
        }
    }
}

/// DTO for audio device
#[derive(serde::Serialize)]
pub struct AudioDeviceDto {
    pub id: String,
    pub name: String,
    pub is_default: bool,
}

/// Set Groq API key (stored securely in Windows Credential Manager)
#[tauri::command]
pub async fn set_groq_api_key(
    state: State<'_, AppState>,
    api_key: String,
) -> Result<(), String> {
    // Validate and store the API key securely
    SecretsManager::set_groq_api_key(&api_key).map_err(|e| e.to_string())?;

    // Update config to mark that API key is configured
    {
        let mut config = state.config.write();
        config.transcription.groq.api_key_configured = true;
    }

    // Persist config
    let config = state.config.read().clone();
    config.save().map_err(|e| e.to_string())?;

    tracing::info!("Groq API key saved securely");
    Ok(())
}

/// Check if Groq API key is configured
#[tauri::command]
pub fn has_groq_api_key() -> bool {
    SecretsManager::has_groq_api_key()
}

/// Remove Groq API key
#[tauri::command]
pub async fn clear_groq_api_key(state: State<'_, AppState>) -> Result<(), String> {
    // Remove from secure storage
    let _ = SecretsManager::delete_groq_api_key();

    // Update config
    {
        let mut config = state.config.write();
        config.transcription.groq.api_key_configured = false;
    }

    // Persist config
    let config = state.config.read().clone();
    config.save().map_err(|e| e.to_string())?;

    tracing::info!("Groq API key removed");
    Ok(())
}

/// Validate Groq API key format (without storing)
#[tauri::command]
pub fn validate_groq_api_key(api_key: String) -> Result<(), String> {
    SecretsManager::validate_groq_api_key(&api_key).map_err(|e| e.to_string())
}
