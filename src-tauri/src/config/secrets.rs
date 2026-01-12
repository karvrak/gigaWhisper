//! Secure Secrets Storage
//!
//! Uses Windows Credential Manager to securely store API keys and other secrets.

use keyring::Entry;
use thiserror::Error;

const SERVICE_NAME: &str = "gigawhisper";
const GROQ_API_KEY_NAME: &str = "groq_api_key";

/// Errors related to secret storage
#[derive(Debug, Error)]
pub enum SecretsError {
    #[error("Failed to access credential store: {0}")]
    CredentialStoreError(String),

    #[error("Secret not found: {0}")]
    NotFound(String),

    #[error("Invalid secret format: {0}")]
    InvalidFormat(String),
}

impl From<keyring::Error> for SecretsError {
    fn from(err: keyring::Error) -> Self {
        match err {
            keyring::Error::NoEntry => SecretsError::NotFound("No entry found".to_string()),
            _ => SecretsError::CredentialStoreError(err.to_string()),
        }
    }
}

/// Manages secure storage of application secrets
pub struct SecretsManager;

impl SecretsManager {
    /// Store the Groq API key securely
    pub fn set_groq_api_key(api_key: &str) -> Result<(), SecretsError> {
        // Validate before storing
        Self::validate_groq_api_key(api_key)?;

        let entry = Entry::new(SERVICE_NAME, GROQ_API_KEY_NAME)
            .map_err(|e| SecretsError::CredentialStoreError(e.to_string()))?;

        entry.set_password(api_key)?;
        tracing::info!("Groq API key stored securely in credential manager");
        Ok(())
    }

    /// Retrieve the Groq API key
    pub fn get_groq_api_key() -> Result<String, SecretsError> {
        let entry = Entry::new(SERVICE_NAME, GROQ_API_KEY_NAME)
            .map_err(|e| SecretsError::CredentialStoreError(e.to_string()))?;

        let password = entry.get_password()?;
        Ok(password)
    }

    /// Delete the Groq API key
    pub fn delete_groq_api_key() -> Result<(), SecretsError> {
        let entry = Entry::new(SERVICE_NAME, GROQ_API_KEY_NAME)
            .map_err(|e| SecretsError::CredentialStoreError(e.to_string()))?;

        entry.delete_credential()?;
        tracing::info!("Groq API key removed from credential manager");
        Ok(())
    }

    /// Check if Groq API key exists
    pub fn has_groq_api_key() -> bool {
        Self::get_groq_api_key().is_ok()
    }

    /// Validate Groq API key format
    /// Groq API keys start with "gsk_" and are 56 characters long
    pub fn validate_groq_api_key(api_key: &str) -> Result<(), SecretsError> {
        let api_key = api_key.trim();

        if api_key.is_empty() {
            return Err(SecretsError::InvalidFormat(
                "API key cannot be empty".to_string(),
            ));
        }

        // Groq API keys start with "gsk_"
        if !api_key.starts_with("gsk_") {
            return Err(SecretsError::InvalidFormat(
                "Groq API key must start with 'gsk_'".to_string(),
            ));
        }

        // Groq API keys are typically 56 characters
        if api_key.len() < 20 {
            return Err(SecretsError::InvalidFormat(
                "API key is too short".to_string(),
            ));
        }

        // Only alphanumeric and underscore allowed
        if !api_key.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return Err(SecretsError::InvalidFormat(
                "API key contains invalid characters".to_string(),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_empty_key() {
        assert!(SecretsManager::validate_groq_api_key("").is_err());
    }

    #[test]
    fn test_validate_invalid_prefix() {
        assert!(SecretsManager::validate_groq_api_key("invalid_key_here").is_err());
    }

    #[test]
    fn test_validate_too_short() {
        assert!(SecretsManager::validate_groq_api_key("gsk_abc").is_err());
    }

    #[test]
    fn test_validate_valid_format() {
        // Valid format (fake key for testing)
        let valid_key = "gsk_abcdefghijklmnopqrstuvwxyz123456789012345678901234";
        assert!(SecretsManager::validate_groq_api_key(valid_key).is_ok());
    }

    #[test]
    fn test_validate_invalid_characters() {
        assert!(SecretsManager::validate_groq_api_key("gsk_test!@#$%^&*()").is_err());
    }
}
