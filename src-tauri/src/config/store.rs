//! Configuration Storage
//!
//! Persist settings to disk.

use super::{Settings, SettingsError};
use std::path::PathBuf;

/// Get the configuration directory path
pub fn config_dir() -> PathBuf {
    directories::ProjectDirs::from("com", "gigawhisper", "GigaWhisper")
        .map(|dirs| dirs.config_dir().to_path_buf())
        .unwrap_or_else(|| {
            // Fallback to current directory
            std::env::current_dir().unwrap_or_default().join("config")
        })
}

/// Get the configuration file path
pub fn config_file() -> PathBuf {
    config_dir().join("settings.toml")
}

/// Get the models directory path
pub fn models_dir() -> PathBuf {
    directories::ProjectDirs::from("com", "gigawhisper", "GigaWhisper")
        .map(|dirs| dirs.data_dir().to_path_buf())
        .unwrap_or_else(|| {
            std::env::current_dir().unwrap_or_default().join("models")
        })
        .join("models")
}

/// Load settings from disk
///
/// Settings are automatically sanitized to ensure values are within valid ranges.
/// This prevents issues from manually edited configuration files.
pub fn load_settings() -> Result<Settings, SettingsError> {
    let path = config_file();

    if !path.exists() {
        tracing::info!("No settings file found, using defaults");
        return Ok(Settings::default());
    }

    let content = std::fs::read_to_string(&path)?;
    let settings: Settings = toml::from_str(&content)?;

    // Sanitize settings to ensure values are within valid ranges
    // This prevents crashes or unexpected behavior from manually edited config files
    let sanitized = settings.sanitize();

    // Log if any values were sanitized
    if settings.recording.max_duration != sanitized.recording.max_duration
        || settings.recording.silence_timeout != sanitized.recording.silence_timeout
        || settings.transcription.local.threads != sanitized.transcription.local.threads
        || settings.transcription.groq.timeout_seconds != sanitized.transcription.groq.timeout_seconds
        || settings.audio.vad.aggressiveness != sanitized.audio.vad.aggressiveness
        || settings.output.paste_delay != sanitized.output.paste_delay
    {
        tracing::warn!("Some settings values were out of range and have been sanitized");
    }

    tracing::info!("Settings loaded from {:?}", path);
    Ok(sanitized)
}

/// Save settings to disk
pub fn save_settings(settings: &Settings) -> Result<(), SettingsError> {
    let path = config_file();

    // Ensure config directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let content = toml::to_string_pretty(settings)?;
    std::fs::write(&path, content)?;

    tracing::info!("Settings saved to {:?}", path);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_settings_roundtrip() {
        let settings = Settings::default();
        let serialized = toml::to_string_pretty(&settings).unwrap();
        let deserialized: Settings = toml::from_str(&serialized).unwrap();

        assert_eq!(
            settings.shortcuts.record,
            deserialized.shortcuts.record
        );
    }
}
