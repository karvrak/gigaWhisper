# ADR-006: Configuration Schema Migration System

## Status

Accepted

## Date

2026-01-26

## Context

GigaWhisper stores user settings in a TOML file (`settings.toml`) in the application's config directory. Currently, when the application is updated:

1. New settings fields are handled by `#[serde(default)]` annotations
2. Removed settings are silently ignored during deserialization
3. There is no mechanism to transform existing values when their semantics change
4. Settings corruption or incompatibility can cause application crashes

As the application evolves, we need a robust system to:
- Version the configuration schema
- Migrate settings between versions automatically
- Handle breaking changes gracefully
- Protect against data loss

## Decision

We will implement a schema versioning and migration system with the following components:

### 1. Schema Version Field

Add a `schema_version` field to the root of the Settings struct:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// Schema version for migration support
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,

    // ... existing fields
}

fn default_schema_version() -> u32 {
    CURRENT_SCHEMA_VERSION
}

const CURRENT_SCHEMA_VERSION: u32 = 1;
```

### 2. Migration System Architecture

```rust
/// Represents a single migration step
pub trait Migration {
    /// Source version this migration applies to
    fn from_version(&self) -> u32;

    /// Target version after migration
    fn to_version(&self) -> u32;

    /// Apply the migration to raw TOML value
    fn migrate(&self, config: &mut toml::Value) -> Result<(), MigrationError>;

    /// Human-readable description
    fn description(&self) -> &'static str;
}

/// Migration registry
pub struct MigrationRegistry {
    migrations: Vec<Box<dyn Migration>>,
}

impl MigrationRegistry {
    pub fn new() -> Self {
        let mut registry = Self { migrations: vec![] };
        // Register all migrations
        // registry.register(Box::new(MigrationV1ToV2));
        registry
    }

    pub fn migrate_to_current(&self, config: &mut toml::Value) -> Result<(), MigrationError> {
        let current_version = config
            .get("schema_version")
            .and_then(|v| v.as_integer())
            .unwrap_or(0) as u32;

        for migration in &self.migrations {
            if migration.from_version() >= current_version
               && migration.to_version() <= CURRENT_SCHEMA_VERSION {
                migration.migrate(config)?;
            }
        }

        // Update schema version
        config["schema_version"] = toml::Value::Integer(CURRENT_SCHEMA_VERSION as i64);
        Ok(())
    }
}
```

### 3. Backup Strategy

Before any migration:

```rust
fn backup_config(path: &Path) -> Result<PathBuf, MigrationError> {
    let backup_path = path.with_extension(format!(
        "toml.backup.{}",
        chrono::Utc::now().format("%Y%m%d_%H%M%S")
    ));
    std::fs::copy(path, &backup_path)?;
    Ok(backup_path)
}
```

### 4. Rollback Mechanism

```rust
pub struct MigrationContext {
    backup_path: Option<PathBuf>,
    config_path: PathBuf,
}

impl MigrationContext {
    pub fn rollback(&self) -> Result<(), MigrationError> {
        if let Some(backup) = &self.backup_path {
            std::fs::copy(backup, &self.config_path)?;
        }
        Ok(())
    }
}
```

### 5. Load Settings Flow

```rust
pub fn load_settings() -> Result<Settings, SettingsError> {
    let path = config_file();

    if !path.exists() {
        return Ok(Settings::default());
    }

    // Read raw TOML
    let content = std::fs::read_to_string(&path)?;
    let mut config: toml::Value = toml::from_str(&content)?;

    // Check if migration needed
    let version = config
        .get("schema_version")
        .and_then(|v| v.as_integer())
        .unwrap_or(0) as u32;

    if version < CURRENT_SCHEMA_VERSION {
        // Create backup
        let backup = backup_config(&path)?;
        tracing::info!("Config backup created: {:?}", backup);

        // Run migrations
        let registry = MigrationRegistry::new();
        if let Err(e) = registry.migrate_to_current(&mut config) {
            tracing::error!("Migration failed: {}, restoring backup", e);
            std::fs::copy(&backup, &path)?;
            return Err(e.into());
        }

        // Save migrated config
        let migrated_content = toml::to_string_pretty(&config)?;
        std::fs::write(&path, migrated_content)?;
        tracing::info!("Config migrated from v{} to v{}", version, CURRENT_SCHEMA_VERSION);
    }

    // Parse as Settings struct
    let settings: Settings = toml::from_str(&toml::to_string(&config)?)?;
    Ok(settings.sanitize())
}
```

## Example Migration

```rust
pub struct MigrationV1ToV2;

impl Migration for MigrationV1ToV2 {
    fn from_version(&self) -> u32 { 1 }
    fn to_version(&self) -> u32 { 2 }

    fn description(&self) -> &'static str {
        "Rename 'hotkey' to 'shortcuts.record'"
    }

    fn migrate(&self, config: &mut toml::Value) -> Result<(), MigrationError> {
        // Example: Move a field to a new location
        if let Some(hotkey) = config.get("hotkey").cloned() {
            config.as_table_mut()
                .ok_or(MigrationError::InvalidConfig)?
                .remove("hotkey");

            config["shortcuts"]["record"] = hotkey;
        }
        Ok(())
    }
}
```

## Consequences

### Positive

- **Forward compatibility**: Users can upgrade without losing settings
- **Data safety**: Automatic backups prevent data loss
- **Debuggability**: Schema version makes support easier
- **Flexibility**: Can handle complex transformations, not just additive changes
- **Rollback support**: Failed migrations can be recovered

### Negative

- **Complexity**: Adds migration infrastructure to maintain
- **Testing burden**: Each migration needs thorough testing
- **Backward incompatibility**: Old app versions cannot read new schema

### Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| Migration bug corrupts config | Automatic backup before migration |
| Partial migration failure | Transaction-like approach with rollback |
| Version conflicts in concurrent access | File locking during migration |
| Backup disk space | Limit to last 5 backups, clean old ones |

## Implementation Plan

1. **Phase 1**: Add `schema_version` field with default value 1
2. **Phase 2**: Implement Migration trait and registry
3. **Phase 3**: Add backup/rollback functionality
4. **Phase 4**: Integrate into `load_settings()` flow
5. **Phase 5**: Add cleanup for old backups

## Alternatives Considered

### 1. Always use defaults for missing fields

Current approach. Simple but cannot handle renamed fields, changed semantics, or breaking changes.

### 2. Store settings in SQLite

Would provide built-in migration support but adds complexity for simple key-value settings.

### 3. Use a settings service/daemon

Overkill for a desktop application with simple settings needs.

## References

- [semver for configuration](https://semver.org/)
- [Django migrations](https://docs.djangoproject.com/en/4.2/topics/migrations/)
- [Rust config crate](https://docs.rs/config/latest/config/)
