//! License Management
//!
//! Premium license activation, validation, and caching.
//! Stores license keys securely in the OS credential manager.

use keyring::Entry;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

const SERVICE_NAME: &str = "gigawhisper";
const LICENSE_KEY_NAME: &str = "license_key";
const API_BASE_URL: &str = "https://api.gigawhisper.com/v1/license";
const REVALIDATION_INTERVAL_SECS: u64 = 24 * 60 * 60; // 24 hours

/// License validation errors
#[derive(Debug, Error)]
pub enum LicenseError {
    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Invalid license key")]
    InvalidKey,

    #[error("License has expired")]
    Expired,

    #[error("Activation limit reached for this license")]
    ActivationLimit,

    #[error("Server error: {0}")]
    ServerError(String),

    #[error("Credential storage error: {0}")]
    CredentialError(String),
}

/// Cached license state (persisted locally)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseState {
    /// The license key (stored separately in credential manager, never serialized)
    #[serde(skip)]
    pub license_key: Option<String>,
    /// Whether the user currently has an active premium license
    pub is_premium: bool,
    /// Unix timestamp of last successful server validation
    pub last_validated: Option<u64>,
    /// Unix timestamp when the license expires
    pub expires_at: Option<u64>,
    /// Number of days the license remains valid offline after last validation
    pub offline_grace_days: u32,
    /// Unique machine identifier derived from hostname + username
    pub machine_id: String,
}

impl Default for LicenseState {
    fn default() -> Self {
        Self {
            license_key: None,
            is_premium: false,
            last_validated: None,
            expires_at: None,
            offline_grace_days: 7,
            machine_id: String::new(),
        }
    }
}

/// Server response for license activation/validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseActivation {
    /// Whether the license is valid
    pub valid: bool,
    /// Unix timestamp when the license expires
    pub expires_at: Option<u64>,
    /// License tier (e.g., "premium", "pro")
    pub tier: String,
}

/// Request payload sent to the license server
#[derive(Debug, Serialize)]
struct LicenseRequest {
    key: String,
    machine_id: String,
}

/// Manages license activation, validation, and credential storage
pub struct LicenseManager {
    state: LicenseState,
    client: reqwest::Client,
}

impl Default for LicenseManager {
    fn default() -> Self {
        Self::new()
    }
}

impl LicenseManager {
    /// Create a new LicenseManager with a generated machine_id
    pub fn new() -> Self {
        let machine_id = Self::generate_machine_id();
        Self {
            state: LicenseState {
                machine_id: machine_id.clone(),
                ..Default::default()
            },
            client: reqwest::Client::new(),
        }
    }

    /// Generate a deterministic machine identifier from hostname + username
    fn generate_machine_id() -> String {
        let hostname = std::env::var("COMPUTERNAME")
            .or_else(|_| std::env::var("HOSTNAME"))
            .unwrap_or_else(|_| "unknown-host".to_string());

        let username = std::env::var("USERNAME")
            .or_else(|_| std::env::var("USER"))
            .unwrap_or_else(|_| "unknown-user".to_string());

        let mut hasher = Sha256::new();
        hasher.update(format!("{}{}", hostname, username));
        let result = hasher.finalize();
        format!("{:x}", result)
    }

    /// Activate a license key with the server
    ///
    /// Sends the key and machine_id to the validation endpoint.
    /// On success, stores the key in the OS credential manager.
    pub async fn activate(&mut self, key: &str) -> Result<LicenseActivation, LicenseError> {
        let request = LicenseRequest {
            key: key.to_string(),
            machine_id: self.state.machine_id.clone(),
        };

        let response = self
            .client
            .post(format!("{}/validate", API_BASE_URL))
            .json(&request)
            .send()
            .await
            .map_err(|e| LicenseError::NetworkError(e.to_string()))?;

        let status = response.status();
        if status == reqwest::StatusCode::FORBIDDEN {
            return Err(LicenseError::InvalidKey);
        }
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(LicenseError::ActivationLimit);
        }
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(LicenseError::ServerError(format!(
                "HTTP {}: {}",
                status, body
            )));
        }

        let activation: LicenseActivation = response
            .json()
            .await
            .map_err(|e| LicenseError::ServerError(e.to_string()))?;

        if !activation.valid {
            return Err(LicenseError::InvalidKey);
        }

        // Store the key in the OS credential manager
        Self::store_license_key(key)?;

        // Update cached state
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        self.state.license_key = Some(key.to_string());
        self.state.is_premium = true;
        self.state.last_validated = Some(now);
        self.state.expires_at = activation.expires_at;

        tracing::info!("License activated successfully (tier: {})", activation.tier);
        Ok(activation)
    }

    /// Deactivate the current license
    ///
    /// Notifies the server and removes the key from the credential manager.
    pub async fn deactivate(&mut self) -> Result<(), LicenseError> {
        // Attempt to notify server (best-effort)
        if let Some(ref key) = self.state.license_key {
            let request = LicenseRequest {
                key: key.clone(),
                machine_id: self.state.machine_id.clone(),
            };

            let result = self
                .client
                .post(format!("{}/deactivate", API_BASE_URL))
                .json(&request)
                .send()
                .await;

            if let Err(e) = result {
                tracing::warn!("Failed to notify server of deactivation: {}", e);
            }
        }

        // Remove from credential manager regardless of server response
        Self::remove_license_key()?;

        // Clear cached state
        self.state.license_key = None;
        self.state.is_premium = false;
        self.state.last_validated = None;
        self.state.expires_at = None;

        tracing::info!("License deactivated");
        Ok(())
    }

    /// Re-validate the cached license with the server
    ///
    /// Checks expiration, grace period, and contacts the server if needed.
    pub async fn validate(&mut self) -> Result<bool, LicenseError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Check if license has expired
        if let Some(expires_at) = self.state.expires_at {
            if now > expires_at {
                self.state.is_premium = false;
                return Err(LicenseError::Expired);
            }
        }

        // If no key is stored, not premium
        let key = match self.state.license_key {
            Some(ref k) => k.clone(),
            None => {
                self.state.is_premium = false;
                return Ok(false);
            }
        };

        // Try to re-validate with server
        let request = LicenseRequest {
            key,
            machine_id: self.state.machine_id.clone(),
        };

        match self
            .client
            .post(format!("{}/validate", API_BASE_URL))
            .json(&request)
            .send()
            .await
        {
            Ok(response) => {
                if response.status().is_success() {
                    if let Ok(activation) = response.json::<LicenseActivation>().await {
                        self.state.is_premium = activation.valid;
                        self.state.last_validated = Some(now);
                        self.state.expires_at = activation.expires_at;
                        return Ok(activation.valid);
                    }
                }
                // Server returned error -- fall through to grace period check
            }
            Err(e) => {
                tracing::warn!("License validation network error: {}", e);
                // Network error -- fall through to grace period check
            }
        }

        // Offline: check grace period
        if self.is_in_grace_period() {
            tracing::info!("License validation offline, within grace period");
            Ok(self.state.is_premium)
        } else {
            self.state.is_premium = false;
            Err(LicenseError::NetworkError(
                "Unable to validate license and grace period has expired".to_string(),
            ))
        }
    }

    /// Quick check for premium status from cached state
    pub fn is_premium(&self) -> bool {
        self.state.is_premium
    }

    /// Check if re-validation is needed (last validation > 24h ago)
    pub fn needs_revalidation(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        match self.state.last_validated {
            Some(last) => now.saturating_sub(last) > REVALIDATION_INTERVAL_SECS,
            None => true,
        }
    }

    /// Check if within the offline grace period since last validation
    pub fn is_in_grace_period(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        match self.state.last_validated {
            Some(last) => {
                let grace_secs = u64::from(self.state.offline_grace_days) * 24 * 60 * 60;
                now.saturating_sub(last) <= grace_secs
            }
            None => false,
        }
    }

    /// Get a reference to the current license state
    pub fn state(&self) -> &LicenseState {
        &self.state
    }

    /// Restore state from a previously persisted LicenseState
    pub fn restore_state(&mut self, state: LicenseState) {
        self.state = state;
        // Ensure machine_id is always current
        self.state.machine_id = Self::generate_machine_id();
    }

    // -- Credential manager helpers --

    /// Store the license key in the OS credential manager
    fn store_license_key(key: &str) -> Result<(), LicenseError> {
        let entry = Entry::new(SERVICE_NAME, LICENSE_KEY_NAME)
            .map_err(|e| LicenseError::CredentialError(e.to_string()))?;

        entry
            .set_password(key)
            .map_err(|e| LicenseError::CredentialError(e.to_string()))?;

        tracing::info!("License key stored in credential manager");
        Ok(())
    }

    /// Remove the license key from the OS credential manager
    fn remove_license_key() -> Result<(), LicenseError> {
        let entry = Entry::new(SERVICE_NAME, LICENSE_KEY_NAME)
            .map_err(|e| LicenseError::CredentialError(e.to_string()))?;

        entry
            .delete_credential()
            .map_err(|e| LicenseError::CredentialError(e.to_string()))?;

        tracing::info!("License key removed from credential manager");
        Ok(())
    }

    /// Retrieve the license key from the OS credential manager
    pub fn get_stored_license_key() -> Result<Option<String>, LicenseError> {
        let entry = Entry::new(SERVICE_NAME, LICENSE_KEY_NAME)
            .map_err(|e| LicenseError::CredentialError(e.to_string()))?;

        match entry.get_password() {
            Ok(key) => Ok(Some(key)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(LicenseError::CredentialError(e.to_string())),
        }
    }
}
