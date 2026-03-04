//! Configuration Migration System
//!
//! Handles versioned migrations for settings schema changes.
//! See ADR-006 for design details.

use super::settings::CURRENT_SCHEMA_VERSION;
use std::path::Path;
use thiserror::Error;

/// Migration errors
#[derive(Debug, Error)]
pub enum MigrationError {
    #[error("Invalid configuration format")]
    InvalidConfig,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("TOML parse error: {0}")]
    TomlParse(#[from] toml::de::Error),

    #[error("TOML serialize error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),

    #[error("Migration from v{from} to v{to} failed: {reason}")]
    MigrationFailed { from: u32, to: u32, reason: String },
}

/// Represents a single migration step
pub trait Migration: Send + Sync {
    /// Source version this migration applies to
    fn source_version(&self) -> u32;

    /// Target version after migration
    fn to_version(&self) -> u32;

    /// Apply the migration to raw TOML value
    fn migrate(&self, config: &mut toml::Value) -> Result<(), MigrationError>;

    /// Human-readable description
    fn description(&self) -> &'static str;
}

/// Migration from schema v1 to v2: Add premium features
struct MigrationV1ToV2;

impl Migration for MigrationV1ToV2 {
    fn source_version(&self) -> u32 {
        1
    }

    fn to_version(&self) -> u32 {
        2
    }

    fn description(&self) -> &'static str {
        "Add premium settings, contexts, post-processing, and new cloud providers"
    }

    fn migrate(&self, config: &mut toml::Value) -> Result<(), MigrationError> {
        let table = config.as_table_mut().ok_or(MigrationError::InvalidConfig)?;

        // Add premium section if missing
        if !table.contains_key("premium") {
            let mut premium = toml::map::Map::new();
            premium.insert("is_premium".to_string(), toml::Value::Boolean(false));

            let mut credits = toml::map::Map::new();
            credits.insert("balance_eur".to_string(), toml::Value::Float(0.0));
            credits.insert("low_threshold_eur".to_string(), toml::Value::Float(5.0));
            premium.insert("credits".to_string(), toml::Value::Table(credits));

            table.insert("premium".to_string(), toml::Value::Table(premium));
        }

        // Add contexts section if missing
        if !table.contains_key("contexts") {
            let mut contexts = toml::map::Map::new();
            contexts.insert(
                "active_context".to_string(),
                toml::Value::String("default".to_string()),
            );

            let mut default_ctx = toml::map::Map::new();
            default_ctx.insert("id".to_string(), toml::Value::String("default".to_string()));
            default_ctx.insert(
                "name".to_string(),
                toml::Value::String("Default".to_string()),
            );
            default_ctx.insert("shortcut".to_string(), toml::Value::String(String::new()));
            default_ctx.insert(
                "language".to_string(),
                toml::Value::String("auto".to_string()),
            );
            default_ctx.insert(
                "provider".to_string(),
                toml::Value::String("local".to_string()),
            );

            contexts.insert(
                "contexts".to_string(),
                toml::Value::Array(vec![toml::Value::Table(default_ctx)]),
            );
            table.insert("contexts".to_string(), toml::Value::Table(contexts));
        }

        // Add post_processing section if missing
        if !table.contains_key("post_processing") {
            let mut pp = toml::map::Map::new();
            pp.insert("enabled".to_string(), toml::Value::Boolean(false));
            pp.insert(
                "default_provider".to_string(),
                toml::Value::String("groq-llm".to_string()),
            );
            pp.insert(
                "default_prompt".to_string(),
                toml::Value::String(
                    "Clean up and fix any errors in this transcription. \
                     Keep the original meaning and tone. \
                     Output only the corrected text."
                        .to_string(),
                ),
            );
            table.insert("post_processing".to_string(), toml::Value::Table(pp));
        }

        // Add openai and deepgram to transcription section
        if let Some(toml::Value::Table(transcription)) = table.get_mut("transcription") {
            if !transcription.contains_key("openai") {
                let mut openai = toml::map::Map::new();
                openai.insert(
                    "api_key_configured".to_string(),
                    toml::Value::Boolean(false),
                );
                openai.insert(
                    "model".to_string(),
                    toml::Value::String("whisper-1".to_string()),
                );
                openai.insert("timeout_seconds".to_string(), toml::Value::Integer(30));
                transcription.insert("openai".to_string(), toml::Value::Table(openai));
            }
            if !transcription.contains_key("deepgram") {
                let mut deepgram = toml::map::Map::new();
                deepgram.insert(
                    "api_key_configured".to_string(),
                    toml::Value::Boolean(false),
                );
                deepgram.insert(
                    "model".to_string(),
                    toml::Value::String("nova-2".to_string()),
                );
                deepgram.insert("timeout_seconds".to_string(), toml::Value::Integer(30));
                transcription.insert("deepgram".to_string(), toml::Value::Table(deepgram));
            }
        }

        // Add custom_theme to ui section
        // Option<String> = None is not serialized in TOML, so we leave it absent.
        // serde will deserialize a missing field as None with #[serde(default)].
        if let Some(toml::Value::Table(_ui)) = table.get_mut("ui") {
            // No action needed: absent key deserializes as None
        }

        tracing::info!("Migrated config from v1 to v2: added premium features");
        Ok(())
    }
}

/// Migration registry that holds all migrations
pub struct MigrationRegistry {
    migrations: Vec<Box<dyn Migration>>,
}

impl MigrationRegistry {
    /// Create a new registry with all known migrations
    pub fn new() -> Self {
        Self {
            migrations: vec![Box::new(MigrationV1ToV2)],
        }
    }

    /// Get the current schema version from a TOML config
    pub fn get_version(config: &toml::Value) -> u32 {
        config
            .get("schema_version")
            .and_then(|v| v.as_integer())
            .unwrap_or(0) as u32
    }

    /// Check if migration is needed
    pub fn needs_migration(config: &toml::Value) -> bool {
        Self::get_version(config) < CURRENT_SCHEMA_VERSION
    }

    /// Migrate configuration to current version
    pub fn migrate_to_current(&self, config: &mut toml::Value) -> Result<(), MigrationError> {
        let current_version = Self::get_version(config);

        if current_version >= CURRENT_SCHEMA_VERSION {
            return Ok(()); // Already up to date
        }

        tracing::info!(
            "Migrating config from v{} to v{}",
            current_version,
            CURRENT_SCHEMA_VERSION
        );

        // Apply migrations in order, chaining from one version to the next
        let mut current_version = current_version;
        for migration in &self.migrations {
            let from = migration.source_version();
            let to = migration.to_version();

            // Only apply the migration whose source matches our current version
            if from == current_version && to <= CURRENT_SCHEMA_VERSION {
                tracing::debug!(
                    "Applying migration v{} -> v{}: {}",
                    from,
                    to,
                    migration.description()
                );
                migration.migrate(config)?;
                current_version = to;
            }
        }

        // Update schema version
        if let Some(table) = config.as_table_mut() {
            table.insert(
                "schema_version".to_string(),
                toml::Value::Integer(CURRENT_SCHEMA_VERSION as i64),
            );
        }

        Ok(())
    }
}

impl Default for MigrationRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a timestamped backup of the config file
pub fn backup_config(path: &Path) -> Result<std::path::PathBuf, MigrationError> {
    let timestamp = chrono_lite_timestamp();
    let backup_path = path.with_extension(format!("toml.backup.{}", timestamp));

    std::fs::copy(path, &backup_path)?;
    tracing::info!("Config backup created: {:?}", backup_path);

    Ok(backup_path)
}

/// Restore config from a backup
pub fn restore_from_backup(backup_path: &Path, config_path: &Path) -> Result<(), MigrationError> {
    std::fs::copy(backup_path, config_path)?;
    tracing::info!("Config restored from backup: {:?}", backup_path);
    Ok(())
}

/// Clean up old backup files, keeping only the most recent ones
pub fn cleanup_old_backups(config_path: &Path, keep_count: usize) -> Result<(), MigrationError> {
    let parent = config_path.parent().ok_or_else(|| {
        MigrationError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "No parent directory",
        ))
    })?;

    let stem = config_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("settings");

    // Find all backup files
    let mut backups: Vec<_> = std::fs::read_dir(parent)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_str()
                .map(|name| name.starts_with(stem) && name.contains(".backup."))
                .unwrap_or(false)
        })
        .collect();

    // Sort by modification time (newest first)
    backups.sort_by(|a, b| {
        let a_time = a.metadata().and_then(|m| m.modified()).ok();
        let b_time = b.metadata().and_then(|m| m.modified()).ok();
        b_time.cmp(&a_time)
    });

    // Remove old backups beyond keep_count
    for backup in backups.iter().skip(keep_count) {
        if let Err(e) = std::fs::remove_file(backup.path()) {
            tracing::warn!("Failed to remove old backup {:?}: {}", backup.path(), e);
        } else {
            tracing::debug!("Removed old backup: {:?}", backup.path());
        }
    }

    Ok(())
}

/// Generate a timestamp string without heavy dependencies
fn chrono_lite_timestamp() -> String {
    use std::time::SystemTime;

    let duration = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();

    let secs = duration.as_secs();

    // Convert to rough date/time (not accurate for timezones, but sufficient for uniqueness)
    let days = secs / 86400;
    let years_since_1970 = days / 365;
    let year = 1970 + years_since_1970;
    let day_of_year = days % 365;
    let month = (day_of_year / 30).min(11) + 1;
    let day = (day_of_year % 30) + 1;

    let time_of_day = secs % 86400;
    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let seconds = time_of_day % 60;

    format!(
        "{:04}{:02}{:02}_{:02}{:02}{:02}",
        year, month, day, hours, minutes, seconds
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_version_missing() {
        let config = toml::Value::Table(toml::map::Map::new());
        assert_eq!(MigrationRegistry::get_version(&config), 0);
    }

    #[test]
    fn test_get_version_present() {
        let mut config = toml::map::Map::new();
        config.insert("schema_version".to_string(), toml::Value::Integer(5));
        let config = toml::Value::Table(config);
        assert_eq!(MigrationRegistry::get_version(&config), 5);
    }

    #[test]
    fn test_needs_migration_true() {
        let config = toml::Value::Table(toml::map::Map::new());
        assert!(MigrationRegistry::needs_migration(&config));
    }

    #[test]
    fn test_needs_migration_false() {
        let mut config = toml::map::Map::new();
        config.insert(
            "schema_version".to_string(),
            toml::Value::Integer(CURRENT_SCHEMA_VERSION as i64),
        );
        let config = toml::Value::Table(config);
        assert!(!MigrationRegistry::needs_migration(&config));
    }

    #[test]
    fn test_migrate_to_current_already_current() {
        let registry = MigrationRegistry::new();
        let mut config = toml::map::Map::new();
        config.insert(
            "schema_version".to_string(),
            toml::Value::Integer(CURRENT_SCHEMA_VERSION as i64),
        );
        let mut config = toml::Value::Table(config);

        let result = registry.migrate_to_current(&mut config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_migrate_to_current_updates_version() {
        let registry = MigrationRegistry::new();
        let config = toml::map::Map::new();
        let mut config = toml::Value::Table(config);

        let result = registry.migrate_to_current(&mut config);
        assert!(result.is_ok());
        assert_eq!(
            MigrationRegistry::get_version(&config),
            CURRENT_SCHEMA_VERSION
        );
    }

    #[test]
    fn test_chrono_lite_timestamp_format() {
        let timestamp = chrono_lite_timestamp();
        // Should be in format YYYYMMDD_HHMMSS (15 chars)
        assert_eq!(timestamp.len(), 15);
        assert!(timestamp.contains('_'));
    }
}
