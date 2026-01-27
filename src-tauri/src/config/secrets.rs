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
    /// Groq API keys start with "gsk_" and are typically 56 characters long
    pub fn validate_groq_api_key(api_key: &str) -> Result<(), SecretsError> {
        let api_key = api_key.trim();

        // Maximum length to prevent abuse (Groq keys are ~56 chars, allow some margin)
        const MAX_API_KEY_LENGTH: usize = 100;

        if api_key.is_empty() {
            return Err(SecretsError::InvalidFormat(
                "API key cannot be empty".to_string(),
            ));
        }

        // Check maximum length to prevent abuse
        if api_key.len() > MAX_API_KEY_LENGTH {
            return Err(SecretsError::InvalidFormat(
                format!("API key is too long (max {} characters)", MAX_API_KEY_LENGTH),
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

    // ============================================================================
    // VALIDATION TESTS - Happy Path
    // ============================================================================

    #[test]
    fn test_validate_valid_format_typical_groq_key() {
        // Valid format - typical Groq API key (56 characters)
        let valid_key = "gsk_abcdefghijklmnopqrstuvwxyz123456789012345678901234";
        assert!(SecretsManager::validate_groq_api_key(valid_key).is_ok());
    }

    #[test]
    fn test_validate_valid_format_minimum_length() {
        // Minimum valid length (20 characters)
        let min_key = "gsk_1234567890123456"; // 20 chars
        assert!(SecretsManager::validate_groq_api_key(min_key).is_ok());
    }

    #[test]
    fn test_validate_valid_format_exactly_max_length() {
        // Create a key that's exactly 100 chars (max allowed)
        let key = format!("gsk_{}", "a".repeat(96)); // gsk_ = 4 chars + 96 = 100
        assert!(SecretsManager::validate_groq_api_key(&key).is_ok());
    }

    #[test]
    fn test_validate_valid_format_with_underscores() {
        // Keys can contain underscores
        let key = "gsk_abc_def_ghi_jkl_mno";
        assert!(SecretsManager::validate_groq_api_key(key).is_ok());
    }

    #[test]
    fn test_validate_valid_format_mixed_case() {
        // Keys can contain uppercase and lowercase
        let key = "gsk_AbCdEfGhIjKlMnOpQrSt";
        assert!(SecretsManager::validate_groq_api_key(key).is_ok());
    }

    #[test]
    fn test_validate_valid_format_with_numbers() {
        // Keys can contain numbers
        let key = "gsk_0123456789abcdef0123";
        assert!(SecretsManager::validate_groq_api_key(key).is_ok());
    }

    #[test]
    fn test_validate_trims_whitespace() {
        // Should trim whitespace before validation
        let key_with_spaces = "  gsk_abcdefghijklmnopqrstuvwxyz123456789012345678901234  ";
        assert!(SecretsManager::validate_groq_api_key(key_with_spaces).is_ok());
    }

    #[test]
    fn test_validate_trims_newlines() {
        // Should trim newlines before validation
        let key_with_newline = "gsk_abcdefghijklmnopqrstuvwxyz123456789012345678901234\n";
        assert!(SecretsManager::validate_groq_api_key(key_with_newline).is_ok());
    }

    // ============================================================================
    // VALIDATION TESTS - Empty and Missing
    // ============================================================================

    #[test]
    fn test_validate_empty_key() {
        let result = SecretsManager::validate_groq_api_key("");
        assert!(result.is_err());
        match result {
            Err(SecretsError::InvalidFormat(msg)) => {
                assert!(msg.contains("empty"), "Expected 'empty' in message, got: {}", msg);
            }
            _ => panic!("Expected InvalidFormat error for empty key"),
        }
    }

    #[test]
    fn test_validate_whitespace_only() {
        let result = SecretsManager::validate_groq_api_key("   ");
        assert!(result.is_err());
        match result {
            Err(SecretsError::InvalidFormat(msg)) => {
                assert!(msg.contains("empty"), "Expected 'empty' in message, got: {}", msg);
            }
            _ => panic!("Expected InvalidFormat error for whitespace-only key"),
        }
    }

    #[test]
    fn test_validate_tabs_only() {
        let result = SecretsManager::validate_groq_api_key("\t\t\t");
        assert!(result.is_err());
        match result {
            Err(SecretsError::InvalidFormat(msg)) => {
                assert!(msg.contains("empty"), "Expected 'empty' in message, got: {}", msg);
            }
            _ => panic!("Expected InvalidFormat error for tab-only key"),
        }
    }

    // ============================================================================
    // VALIDATION TESTS - Invalid Prefix
    // ============================================================================

    #[test]
    fn test_validate_invalid_prefix_no_prefix() {
        let result = SecretsManager::validate_groq_api_key("invalid_key_here_abcdefgh");
        assert!(result.is_err());
        match result {
            Err(SecretsError::InvalidFormat(msg)) => {
                assert!(msg.contains("gsk_"), "Expected 'gsk_' in message, got: {}", msg);
            }
            _ => panic!("Expected InvalidFormat error for invalid prefix"),
        }
    }

    #[test]
    fn test_validate_invalid_prefix_wrong_prefix() {
        let result = SecretsManager::validate_groq_api_key("sk_abcdefghijklmnopqrstuvwxyz");
        assert!(result.is_err());
        match result {
            Err(SecretsError::InvalidFormat(msg)) => {
                assert!(msg.contains("gsk_"), "Expected 'gsk_' in message, got: {}", msg);
            }
            _ => panic!("Expected InvalidFormat error for wrong prefix"),
        }
    }

    #[test]
    fn test_validate_invalid_prefix_case_sensitive() {
        // Prefix must be lowercase "gsk_"
        let result = SecretsManager::validate_groq_api_key("GSK_abcdefghijklmnopqrstuvwxyz");
        assert!(result.is_err());
        match result {
            Err(SecretsError::InvalidFormat(msg)) => {
                assert!(msg.contains("gsk_"), "Expected 'gsk_' in message, got: {}", msg);
            }
            _ => panic!("Expected InvalidFormat error for uppercase prefix"),
        }
    }

    #[test]
    fn test_validate_invalid_prefix_partial() {
        // "gsk" without underscore
        let result = SecretsManager::validate_groq_api_key("gskabcdefghijklmnopqrstuvwxyz");
        assert!(result.is_err());
    }

    // ============================================================================
    // VALIDATION TESTS - Length Boundaries
    // ============================================================================

    #[test]
    fn test_validate_too_short_just_prefix() {
        let result = SecretsManager::validate_groq_api_key("gsk_");
        assert!(result.is_err());
        match result {
            Err(SecretsError::InvalidFormat(msg)) => {
                assert!(msg.contains("short"), "Expected 'short' in message, got: {}", msg);
            }
            _ => panic!("Expected InvalidFormat error for too short key"),
        }
    }

    #[test]
    fn test_validate_too_short_boundary() {
        // 19 characters (just under minimum of 20)
        let result = SecretsManager::validate_groq_api_key("gsk_123456789012345"); // 19 chars
        assert!(result.is_err());
        match result {
            Err(SecretsError::InvalidFormat(msg)) => {
                assert!(msg.contains("short"), "Expected 'short' in message, got: {}", msg);
            }
            _ => panic!("Expected InvalidFormat error for boundary short key"),
        }
    }

    #[test]
    fn test_validate_too_long() {
        // Create a key that's too long (> 100 chars)
        let long_key = format!("gsk_{}", "a".repeat(200));
        let result = SecretsManager::validate_groq_api_key(&long_key);
        assert!(result.is_err());
        match result {
            Err(SecretsError::InvalidFormat(msg)) => {
                assert!(msg.contains("too long"), "Expected 'too long' in message, got: {}", msg);
            }
            _ => panic!("Expected InvalidFormat error for too long key"),
        }
    }

    #[test]
    fn test_validate_too_long_boundary() {
        // 101 characters (just over maximum of 100)
        let key = format!("gsk_{}", "a".repeat(97)); // 101 chars total
        let result = SecretsManager::validate_groq_api_key(&key);
        assert!(result.is_err());
        match result {
            Err(SecretsError::InvalidFormat(msg)) => {
                assert!(msg.contains("too long"), "Expected 'too long' in message, got: {}", msg);
            }
            _ => panic!("Expected InvalidFormat error for boundary long key"),
        }
    }

    // ============================================================================
    // VALIDATION TESTS - Invalid Characters
    // ============================================================================

    #[test]
    fn test_validate_invalid_characters_special() {
        let result = SecretsManager::validate_groq_api_key("gsk_test!@#$%^&*()test");
        assert!(result.is_err());
        match result {
            Err(SecretsError::InvalidFormat(msg)) => {
                assert!(msg.contains("invalid characters"), "Expected 'invalid characters' in message, got: {}", msg);
            }
            _ => panic!("Expected InvalidFormat error for special characters"),
        }
    }

    #[test]
    fn test_validate_invalid_characters_space_in_middle() {
        let result = SecretsManager::validate_groq_api_key("gsk_test key with spaces");
        assert!(result.is_err());
        match result {
            Err(SecretsError::InvalidFormat(msg)) => {
                assert!(msg.contains("invalid characters"), "Expected 'invalid characters' in message, got: {}", msg);
            }
            _ => panic!("Expected InvalidFormat error for spaces in middle"),
        }
    }

    #[test]
    fn test_validate_invalid_characters_hyphen() {
        let result = SecretsManager::validate_groq_api_key("gsk_test-key-with-hyphens");
        assert!(result.is_err());
        match result {
            Err(SecretsError::InvalidFormat(msg)) => {
                assert!(msg.contains("invalid characters"), "Expected 'invalid characters' in message, got: {}", msg);
            }
            _ => panic!("Expected InvalidFormat error for hyphens"),
        }
    }

    #[test]
    fn test_validate_invalid_characters_unicode() {
        let result = SecretsManager::validate_groq_api_key("gsk_test\u{00E9}accent");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_invalid_characters_emoji() {
        // Key without emoji should pass (alphanumeric + underscore)
        let valid_key = "gsk_test_key_with_emoji_here";
        assert!(SecretsManager::validate_groq_api_key(valid_key).is_ok());

        // Key with actual emoji should fail
        let result_emoji = SecretsManager::validate_groq_api_key("gsk_test\u{1F600}emoji");
        assert!(result_emoji.is_err());
    }

    #[test]
    fn test_validate_invalid_characters_null_byte() {
        let result = SecretsManager::validate_groq_api_key("gsk_test\0null_byte_test");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_invalid_characters_dot() {
        let result = SecretsManager::validate_groq_api_key("gsk_test.key.with.dots");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_invalid_characters_sql_injection_attempt() {
        let result = SecretsManager::validate_groq_api_key("gsk_test'; DROP TABLE users;--");
        assert!(result.is_err());
    }

    // ============================================================================
    // ERROR TYPE TESTS
    // ============================================================================

    #[test]
    fn test_secrets_error_display_credential_store_error() {
        let error = SecretsError::CredentialStoreError("test error".to_string());
        let display = format!("{}", error);
        assert!(display.contains("credential store"));
        assert!(display.contains("test error"));
    }

    #[test]
    fn test_secrets_error_display_not_found() {
        let error = SecretsError::NotFound("test key".to_string());
        let display = format!("{}", error);
        assert!(display.contains("not found"));
        assert!(display.contains("test key"));
    }

    #[test]
    fn test_secrets_error_display_invalid_format() {
        let error = SecretsError::InvalidFormat("bad format".to_string());
        let display = format!("{}", error);
        assert!(display.contains("Invalid"));
        assert!(display.contains("bad format"));
    }

    #[test]
    fn test_secrets_error_debug_format() {
        let error = SecretsError::CredentialStoreError("test".to_string());
        let debug = format!("{:?}", error);
        assert!(debug.contains("CredentialStoreError"));
    }

    // ============================================================================
    // KEYRING ERROR CONVERSION TESTS
    // ============================================================================

    #[test]
    fn test_keyring_error_conversion_no_entry() {
        let keyring_error = keyring::Error::NoEntry;
        let secrets_error: SecretsError = keyring_error.into();
        match secrets_error {
            SecretsError::NotFound(_) => {} // Expected
            _ => panic!("Expected NotFound error for NoEntry"),
        }
    }

    #[test]
    fn test_keyring_error_conversion_other_errors() {
        // Test that other keyring errors map to CredentialStoreError
        // Use Invalid error variant which is simpler to construct
        let keyring_error = keyring::Error::Invalid("test_param".into(), "test_reason".into());
        let secrets_error: SecretsError = keyring_error.into();
        match secrets_error {
            SecretsError::CredentialStoreError(_) => {} // Expected
            _ => panic!("Expected CredentialStoreError for Invalid error"),
        }
    }

    // ============================================================================
    // INTEGRATION TESTS (with real credential store)
    // These tests interact with the actual Windows Credential Manager.
    // They use a unique test key name to avoid conflicts.
    // ============================================================================

    #[cfg(test)]
    mod integration_tests {
        use super::*;

        // Use a unique test key to avoid conflicts with real keys
        const TEST_SERVICE: &str = "gigawhisper_test";
        const TEST_KEY_NAME: &str = "test_api_key";

        /// Helper to create a test entry
        fn test_entry() -> Result<Entry, keyring::Error> {
            Entry::new(TEST_SERVICE, TEST_KEY_NAME)
        }

        /// Clean up test entry
        fn cleanup_test_entry() {
            if let Ok(entry) = test_entry() {
                let _ = entry.delete_credential();
            }
        }

        #[test]
        fn test_credential_store_roundtrip() {
            // This test requires actual credential store access
            // Skip if not available (e.g., in CI without credential store)
            let entry = match test_entry() {
                Ok(e) => e,
                Err(_) => {
                    eprintln!("Skipping test: credential store not available");
                    return;
                }
            };

            // Clean up any previous test data
            cleanup_test_entry();

            let test_password = "gsk_test_password_for_roundtrip_test";

            // Set password
            let set_result = entry.set_password(test_password);
            if set_result.is_err() {
                eprintln!("Skipping test: cannot write to credential store");
                return;
            }

            // Get password
            let retrieved = entry.get_password();
            assert!(retrieved.is_ok(), "Failed to retrieve password");
            assert_eq!(retrieved.unwrap(), test_password);

            // Clean up
            cleanup_test_entry();
        }

        #[test]
        fn test_credential_store_delete() {
            let entry = match test_entry() {
                Ok(e) => e,
                Err(_) => {
                    eprintln!("Skipping test: credential store not available");
                    return;
                }
            };

            cleanup_test_entry();

            let test_password = "gsk_test_password_for_delete_test";

            // Set password
            if entry.set_password(test_password).is_err() {
                eprintln!("Skipping test: cannot write to credential store");
                return;
            }

            // Delete
            let delete_result = entry.delete_credential();
            assert!(delete_result.is_ok(), "Failed to delete credential");

            // Verify deleted
            let get_result = entry.get_password();
            assert!(get_result.is_err(), "Password should not exist after deletion");
        }

        #[test]
        fn test_credential_store_overwrite() {
            let entry = match test_entry() {
                Ok(e) => e,
                Err(_) => {
                    eprintln!("Skipping test: credential store not available");
                    return;
                }
            };

            cleanup_test_entry();

            let first_password = "gsk_first_password_test_overwrite";
            let second_password = "gsk_second_password_test_overwrite";

            // Set first password
            if entry.set_password(first_password).is_err() {
                eprintln!("Skipping test: cannot write to credential store");
                return;
            }

            // Overwrite with second password
            let overwrite_result = entry.set_password(second_password);
            assert!(overwrite_result.is_ok(), "Failed to overwrite password");

            // Verify it's the second password
            let retrieved = entry.get_password();
            assert!(retrieved.is_ok(), "Failed to retrieve password");
            assert_eq!(retrieved.unwrap(), second_password);

            cleanup_test_entry();
        }

        #[test]
        fn test_credential_store_not_found() {
            let entry = match test_entry() {
                Ok(e) => e,
                Err(_) => {
                    eprintln!("Skipping test: credential store not available");
                    return;
                }
            };

            // Ensure no entry exists
            cleanup_test_entry();

            // Try to get non-existent entry
            let result = entry.get_password();
            assert!(result.is_err(), "Should fail for non-existent entry");

            match result {
                Err(keyring::Error::NoEntry) => {} // Expected
                Err(e) => panic!("Expected NoEntry error, got: {:?}", e),
                Ok(_) => panic!("Expected error for non-existent entry"),
            }
        }
    }

    // ============================================================================
    // SECRETS MANAGER FULL WORKFLOW TESTS
    // These test the SecretsManager methods end-to-end
    // ============================================================================

    #[cfg(test)]
    mod secrets_manager_tests {
        use super::*;

        /// Test that set_groq_api_key validates before storing
        #[test]
        fn test_set_groq_api_key_validates_input() {
            // Invalid key should fail validation before reaching credential store
            let result = SecretsManager::set_groq_api_key("invalid_key");
            assert!(result.is_err());
            match result {
                Err(SecretsError::InvalidFormat(_)) => {} // Expected
                _ => panic!("Expected InvalidFormat error for invalid key"),
            }
        }

        /// Test that empty key is rejected
        #[test]
        fn test_set_groq_api_key_rejects_empty() {
            let result = SecretsManager::set_groq_api_key("");
            assert!(result.is_err());
            match result {
                Err(SecretsError::InvalidFormat(_)) => {} // Expected
                _ => panic!("Expected InvalidFormat error for empty key"),
            }
        }

        /// Test validation with key that has leading/trailing whitespace
        #[test]
        fn test_set_groq_api_key_handles_whitespace() {
            // The validation should trim whitespace, so this might fail
            // at credential store level (which is acceptable) but not at validation
            let key_with_spaces = "  gsk_abcdefghijklmnopqrstuvwxyz123456789012345678901234  ";

            // This should pass validation (whitespace is trimmed)
            let validation_result = SecretsManager::validate_groq_api_key(key_with_spaces);
            assert!(validation_result.is_ok(), "Validation should pass with trimmed whitespace");
        }
    }

    // ============================================================================
    // CONSTANTS AND BOUNDARY TESTS
    // ============================================================================

    #[test]
    fn test_service_name_constant() {
        assert_eq!(SERVICE_NAME, "gigawhisper");
    }

    #[test]
    fn test_groq_api_key_name_constant() {
        assert_eq!(GROQ_API_KEY_NAME, "groq_api_key");
    }

    #[test]
    fn test_max_api_key_length_is_reasonable() {
        // Ensure the max length constant is set appropriately
        // Groq keys are ~56 chars, so 100 should be plenty
        const MAX_API_KEY_LENGTH: usize = 100;
        assert!(MAX_API_KEY_LENGTH >= 56, "Max length should accommodate typical Groq keys");
        assert!(MAX_API_KEY_LENGTH <= 500, "Max length should not be excessively large");
    }

    // ============================================================================
    // EDGE CASE TESTS
    // ============================================================================

    #[test]
    fn test_validate_key_with_only_underscores_after_prefix() {
        let key = "gsk_________________"; // 20 chars total
        assert!(SecretsManager::validate_groq_api_key(key).is_ok());
    }

    #[test]
    fn test_validate_key_with_only_numbers_after_prefix() {
        let key = "gsk_1234567890123456"; // 20 chars total
        assert!(SecretsManager::validate_groq_api_key(key).is_ok());
    }

    #[test]
    fn test_validate_key_exactly_20_chars() {
        let key = "gsk_1234567890123456"; // exactly 20 chars
        assert_eq!(key.len(), 20);
        assert!(SecretsManager::validate_groq_api_key(key).is_ok());
    }

    #[test]
    fn test_validate_key_exactly_19_chars() {
        let key = "gsk_123456789012345"; // exactly 19 chars
        assert_eq!(key.len(), 19);
        assert!(SecretsManager::validate_groq_api_key(key).is_err());
    }

    #[test]
    fn test_validate_realistic_groq_key_format() {
        // Realistic Groq API key format (typically looks like this)
        let realistic_key = "gsk_Abc123Def456Ghi789Jkl012Mno345Pqr678Stu901Vwx234Y";
        assert!(SecretsManager::validate_groq_api_key(realistic_key).is_ok());
    }
}
