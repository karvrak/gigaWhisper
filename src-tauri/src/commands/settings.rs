//! Settings Commands
//!
//! Handle configuration read/write operations.

use crate::config::{SecretsManager, Settings};
use crate::shortcuts;
use crate::AppState;
use tauri::{AppHandle, State};

/// Get current settings
#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Settings {
    state.config.read().clone()
}

/// Save settings
#[tauri::command]
pub async fn save_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: Settings,
) -> Result<(), String> {
    tracing::info!("Saving settings");

    // Validate settings
    settings.validate().map_err(|e| e.to_string())?;

    // Check if shortcuts have changed
    let old_shortcut = {
        let config = state.config.read();
        config.shortcuts.record.clone()
    };
    let shortcuts_changed = old_shortcut != settings.shortcuts.record;

    // Update in-memory state
    {
        let mut config = state.config.write();
        *config = settings.clone();
    }

    // Persist to disk
    settings.save().map_err(|e| e.to_string())?;

    // Re-register shortcuts if they changed
    if shortcuts_changed {
        tracing::info!("Shortcut changed, re-registering...");
        if let Err(e) = shortcuts::update_shortcuts(&app) {
            tracing::error!("Failed to update shortcuts: {}", e);
            return Err(format!("Settings saved but shortcut update failed: {}", e));
        }
        tracing::info!("Shortcuts re-registered successfully");
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // AudioDeviceDto Tests
    // =========================================================================

    #[test]
    fn test_audio_device_dto_serialization() {
        let device = AudioDeviceDto {
            id: "device-1".to_string(),
            name: "Test Microphone".to_string(),
            is_default: true,
        };

        let json = serde_json::to_string(&device).expect("Failed to serialize");
        assert!(json.contains("device-1"));
        assert!(json.contains("Test Microphone"));
        assert!(json.contains("true"));
    }

    #[test]
    fn test_audio_device_dto_fields() {
        let device = AudioDeviceDto {
            id: "id123".to_string(),
            name: "My Mic".to_string(),
            is_default: false,
        };

        assert_eq!(device.id, "id123");
        assert_eq!(device.name, "My Mic");
        assert!(!device.is_default);
    }

    // =========================================================================
    // get_audio_devices Tests
    // =========================================================================

    #[test]
    fn test_get_audio_devices_returns_ok() {
        // Should always return Ok (with at least default device fallback)
        let result = get_audio_devices();
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_audio_devices_has_devices() {
        let result = get_audio_devices();
        let devices = result.expect("Should return Ok");

        // Should have at least one device (default fallback)
        assert!(!devices.is_empty());
    }

    #[test]
    fn test_get_audio_devices_has_default() {
        let result = get_audio_devices();
        let devices = result.expect("Should return Ok");

        // At least one device should be marked as default
        let has_default = devices.iter().any(|d| d.is_default);
        assert!(has_default, "Should have at least one default device");
    }

    // =========================================================================
    // validate_groq_api_key Tests
    // =========================================================================

    #[test]
    fn test_validate_groq_api_key_valid() {
        let valid_key = "gsk_abcdefghijklmnopqrstuvwxyz123456789012345678901234".to_string();
        let result = validate_groq_api_key(valid_key);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_groq_api_key_empty() {
        let result = validate_groq_api_key("".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_groq_api_key_wrong_prefix() {
        let result = validate_groq_api_key("sk_test_1234567890".to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("gsk_"));
    }

    #[test]
    fn test_validate_groq_api_key_too_short() {
        let result = validate_groq_api_key("gsk_short".to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("short"));
    }

    #[test]
    fn test_validate_groq_api_key_too_long() {
        let long_key = format!("gsk_{}", "a".repeat(200));
        let result = validate_groq_api_key(long_key);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("long"));
    }

    #[test]
    fn test_validate_groq_api_key_invalid_chars() {
        let result = validate_groq_api_key("gsk_test!@#$%^&*()_invalid".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_groq_api_key_whitespace() {
        let result = validate_groq_api_key("   ".to_string());
        assert!(result.is_err());
    }

    // =========================================================================
    // has_groq_api_key Tests
    // =========================================================================

    #[test]
    fn test_has_groq_api_key_returns_bool() {
        // Should return a boolean without panicking
        let result = has_groq_api_key();
        let _: bool = result;
    }
}
