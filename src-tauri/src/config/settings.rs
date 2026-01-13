//! Settings Definition
//!
//! Application configuration schema.

use serde::{Deserialize, Serialize};

/// Main settings structure
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub recording: RecordingSettings,
    pub shortcuts: ShortcutSettings,
    pub transcription: TranscriptionSettings,
    pub audio: AudioSettings,
    pub output: OutputSettings,
    pub ui: UiSettings,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            recording: RecordingSettings::default(),
            shortcuts: ShortcutSettings::default(),
            transcription: TranscriptionSettings::default(),
            audio: AudioSettings::default(),
            output: OutputSettings::default(),
            ui: UiSettings::default(),
        }
    }
}

impl Settings {
    /// Validate settings
    pub fn validate(&self) -> Result<(), SettingsError> {
        // Validate shortcuts are valid key combinations
        if self.shortcuts.record.is_empty() {
            return Err(SettingsError::InvalidShortcut("record shortcut is empty".to_string()));
        }

        // Validate Groq API key if cloud provider selected
        if self.transcription.provider == TranscriptionProvider::Groq
            && !self.transcription.groq.has_api_key()
        {
            return Err(SettingsError::MissingApiKey);
        }

        Ok(())
    }

    /// Load settings from disk
    pub fn load() -> Result<Self, SettingsError> {
        super::store::load_settings()
    }

    /// Save settings to disk
    pub fn save(&self) -> Result<(), SettingsError> {
        super::store::save_settings(self)
    }
}

/// Recording behavior settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RecordingSettings {
    /// Recording mode: push-to-talk or toggle
    pub mode: RecordingMode,
    /// Maximum recording duration in seconds (0 = unlimited)
    pub max_duration: u32,
    /// Auto-stop after silence (milliseconds, 0 = disabled)
    pub silence_timeout: u32,
}

impl Default for RecordingSettings {
    fn default() -> Self {
        Self {
            mode: RecordingMode::PushToTalk,
            max_duration: 300, // 5 minutes
            silence_timeout: 0,
        }
    }
}

/// Recording mode
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RecordingMode {
    PushToTalk,
    Toggle,
}

/// Keyboard shortcut settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ShortcutSettings {
    /// Main recording shortcut
    pub record: String,
    /// Cancel recording shortcut
    pub cancel: String,
    /// Open settings shortcut
    pub settings: String,
}

impl Default for ShortcutSettings {
    fn default() -> Self {
        Self {
            record: "Ctrl+Space".to_string(),
            cancel: "Escape".to_string(),
            settings: "Ctrl+Shift+W".to_string(),
        }
    }
}

/// Transcription settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TranscriptionSettings {
    /// Active provider
    pub provider: TranscriptionProvider,
    /// Language code (ISO 639-1) or "auto"
    pub language: String,
    /// Local whisper.cpp settings
    pub local: LocalTranscriptionSettings,
    /// Groq API settings
    pub groq: GroqSettings,
}

impl Default for TranscriptionSettings {
    fn default() -> Self {
        Self {
            provider: TranscriptionProvider::Local,
            language: "auto".to_string(),
            local: LocalTranscriptionSettings::default(),
            groq: GroqSettings::default(),
        }
    }
}

/// Transcription provider selection
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TranscriptionProvider {
    Local,
    Groq,
}

/// GPU backend selection for whisper acceleration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GpuBackend {
    /// CPU only (no GPU acceleration)
    Cpu,
    /// Vulkan backend (AMD, Intel, NVIDIA - cross-platform)
    Vulkan,
    /// CUDA backend (NVIDIA only - best performance)
    Cuda,
}

impl GpuBackend {
    /// Check if this backend is available in the current build
    pub fn is_available(&self) -> bool {
        match self {
            GpuBackend::Cpu => true,
            #[cfg(feature = "gpu-vulkan")]
            GpuBackend::Vulkan => true,
            #[cfg(not(feature = "gpu-vulkan"))]
            GpuBackend::Vulkan => false,
            #[cfg(feature = "gpu-cuda")]
            GpuBackend::Cuda => true,
            #[cfg(not(feature = "gpu-cuda"))]
            GpuBackend::Cuda => false,
        }
    }

    /// Get display name for the backend
    pub fn display_name(&self) -> &'static str {
        match self {
            GpuBackend::Cpu => "CPU",
            GpuBackend::Vulkan => "Vulkan (AMD/Intel/NVIDIA)",
            GpuBackend::Cuda => "CUDA (NVIDIA)",
        }
    }
}

/// Local whisper.cpp settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LocalTranscriptionSettings {
    /// Whisper model size
    pub model: WhisperModel,
    /// Number of CPU threads
    pub threads: usize,
    /// Enable GPU acceleration
    pub gpu_enabled: bool,
    /// GPU backend to use when gpu_enabled is true
    pub gpu_backend: GpuBackend,
}

impl Default for LocalTranscriptionSettings {
    fn default() -> Self {
        Self {
            model: WhisperModel::Small,
            threads: 4,
            gpu_enabled: false,
            gpu_backend: GpuBackend::Cpu,
        }
    }
}

/// Whisper model sizes
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WhisperModel {
    Tiny,
    Base,
    Small,
    Medium,
    Large,
}

impl WhisperModel {
    /// Get model filename
    pub fn filename(&self) -> &'static str {
        match self {
            Self::Tiny => "ggml-tiny.bin",
            Self::Base => "ggml-base.bin",
            Self::Small => "ggml-small.bin",
            Self::Medium => "ggml-medium.bin",
            Self::Large => "ggml-large.bin",
        }
    }

    /// Get approximate model size in bytes
    pub fn size_bytes(&self) -> u64 {
        match self {
            Self::Tiny => 75_000_000,
            Self::Base => 142_000_000,
            Self::Small => 466_000_000,
            Self::Medium => 1_500_000_000,
            Self::Large => 2_900_000_000,
        }
    }
}

/// Groq API settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GroqSettings {
    /// Whether an API key is configured (actual key stored in Windows Credential Manager)
    #[serde(default)]
    pub api_key_configured: bool,
    /// Model identifier
    pub model: String,
    /// Request timeout in seconds (default: 30)
    pub timeout_seconds: u32,
}

impl Default for GroqSettings {
    fn default() -> Self {
        Self {
            api_key_configured: false,
            model: "whisper-large-v3".to_string(),
            timeout_seconds: 30,
        }
    }
}

impl GroqSettings {
    /// Get the API key from secure storage
    pub fn get_api_key(&self) -> Option<String> {
        super::SecretsManager::get_groq_api_key().ok()
    }

    /// Set the API key in secure storage
    pub fn set_api_key(&mut self, api_key: &str) -> Result<(), super::SecretsError> {
        super::SecretsManager::set_groq_api_key(api_key)?;
        self.api_key_configured = true;
        Ok(())
    }

    /// Remove the API key from secure storage
    pub fn clear_api_key(&mut self) -> Result<(), super::SecretsError> {
        let _ = super::SecretsManager::delete_groq_api_key();
        self.api_key_configured = false;
        Ok(())
    }

    /// Check if API key is available
    pub fn has_api_key(&self) -> bool {
        self.api_key_configured && super::SecretsManager::has_groq_api_key()
    }
}

/// Audio input settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AudioSettings {
    /// Input device ID (None = default)
    pub input_device: Option<String>,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            input_device: None,
        }
    }
}

/// Output behavior settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct OutputSettings {
    /// Auto-capitalize first letter
    pub auto_capitalize: bool,
    /// Add punctuation automatically
    pub auto_punctuation: bool,
    /// Delay before paste (milliseconds)
    pub paste_delay: u32,
}

impl Default for OutputSettings {
    fn default() -> Self {
        Self {
            auto_capitalize: true,
            auto_punctuation: true,
            paste_delay: 50,
        }
    }
}

/// UI settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UiSettings {
    /// Show recording indicator
    pub show_indicator: bool,
    /// Indicator position
    pub indicator_position: IndicatorPosition,
    /// Application theme
    pub theme: Theme,
    /// Start minimized to tray
    pub start_minimized: bool,
    /// Minimize to tray instead of taskbar
    pub minimize_to_tray: bool,
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
            show_indicator: true,
            indicator_position: IndicatorPosition::Cursor,
            theme: Theme::System,
            start_minimized: false,
            minimize_to_tray: true,
        }
    }
}

/// Recording indicator position
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IndicatorPosition {
    Cursor,
    Center,
    Corner,
}

/// Application theme
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    System,
    Light,
    Dark,
}

/// Settings errors
#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    #[error("Invalid shortcut: {0}")]
    InvalidShortcut(String),

    #[error("Missing API key for cloud provider")]
    MissingApiKey,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] toml::ser::Error),

    #[error("Deserialization error: {0}")]
    Deserialization(#[from] toml::de::Error),
}
