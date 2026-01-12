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
pub fn load_settings() -> Result<Settings, SettingsError> {
    let path = config_file();

    if !path.exists() {
        tracing::info!("No settings file found, using defaults");
        return Ok(Settings::default());
    }

    let content = std::fs::read_to_string(&path)?;
    let settings: Settings = toml::from_str(&content)?;

    tracing::info!("Settings loaded from {:?}", path);
    Ok(settings)
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
