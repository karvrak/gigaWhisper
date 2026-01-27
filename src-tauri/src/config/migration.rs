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
    fn from_version(&self) -> u32;

    /// Target version after migration
    fn to_version(&self) -> u32;

    /// Apply the migration to raw TOML value
    fn migrate(&self, config: &mut toml::Value) -> Result<(), MigrationError>;

    /// Human-readable description
    fn description(&self) -> &'static str;
}

/// Migration registry that holds all migrations
pub struct MigrationRegistry {
    migrations: Vec<Box<dyn Migration>>,
}

impl MigrationRegistry {
    /// Create a new registry with all known migrations
    pub fn new() -> Self {
        let registry = Self {
            migrations: vec![
                // Register migrations here as they are created
                // Example: Box::new(MigrationV1ToV2),
            ],
        };

        // Sort migrations by version order
        registry
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

        // Apply migrations in order
        for migration in &self.migrations {
            let from = migration.from_version();
            let to = migration.to_version();

            // Only apply migrations that are in our version range
            if from >= current_version && to <= CURRENT_SCHEMA_VERSION {
                tracing::debug!(
                    "Applying migration v{} -> v{}: {}",
                    from,
                    to,
                    migration.description()
                );
                migration.migrate(config)?;
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
