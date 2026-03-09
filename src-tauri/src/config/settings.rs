//! Settings Definition
//!
//! Application configuration schema.

use serde::{Deserialize, Serialize};

/// Current schema version for migration support
/// Increment this when making breaking changes to the settings structure
pub const CURRENT_SCHEMA_VERSION: u32 = 3;

/// Default schema version for new or migrated configs
fn default_schema_version() -> u32 {
    CURRENT_SCHEMA_VERSION
}

/// Main settings structure
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Schema version for migration support
    /// When this version is lower than CURRENT_SCHEMA_VERSION, migrations will be applied
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,

    pub recording: RecordingSettings,
    pub shortcuts: ShortcutSettings,
    pub transcription: TranscriptionSettings,
    pub audio: AudioSettings,
    pub output: OutputSettings,
    pub ui: UiSettings,
    pub premium: PremiumSettings,
    pub contexts: ContextsSettings,
    pub post_processing: PostProcessingSettings,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            recording: RecordingSettings::default(),
            shortcuts: ShortcutSettings::default(),
            transcription: TranscriptionSettings::default(),
            audio: AudioSettings::default(),
            output: OutputSettings::default(),
            ui: UiSettings::default(),
            premium: PremiumSettings::default(),
            contexts: ContextsSettings::default(),
            post_processing: PostProcessingSettings::default(),
        }
    }
}

impl Settings {
    /// Validate all settings and return detailed error information
    pub fn validate(&self) -> Result<(), SettingsError> {
        // Validate shortcuts are valid key combinations
        if self.shortcuts.record.is_empty() {
            return Err(SettingsError::InvalidShortcut(
                "record shortcut is empty".to_string(),
            ));
        }

        // Validate API key for cloud providers
        match self.transcription.provider {
            TranscriptionProvider::Groq if !self.transcription.groq.has_api_key() => {
                return Err(SettingsError::MissingApiKey);
            }
            TranscriptionProvider::OpenAi if !super::SecretsManager::has_openai_api_key() => {
                return Err(SettingsError::MissingApiKey);
            }
            TranscriptionProvider::Deepgram if !super::SecretsManager::has_deepgram_api_key() => {
                return Err(SettingsError::MissingApiKey);
            }
            _ => {}
        }

        // Validate recording settings
        self.recording.validate()?;

        // Validate audio settings
        self.audio.validate()?;

        // Validate transcription settings
        self.transcription.validate()?;

        // Validate output settings
        self.output.validate()?;

        Ok(())
    }

    /// Sanitize settings by clamping invalid values to valid ranges
    ///
    /// This is useful when loading settings that may have been manually edited.
    /// Returns a sanitized copy of the settings.
    pub fn sanitize(&self) -> Self {
        let mut sanitized = self.clone();
        sanitized.recording.sanitize();
        sanitized.audio.sanitize();
        sanitized.transcription.sanitize();
        sanitized.output.sanitize();
        sanitized
    }

    /// Load settings from disk
    pub fn load() -> Result<Self, SettingsError> {
        super::store::load_settings()
    }

    /// Save settings to disk
    pub fn save(&self) -> Result<(), SettingsError> {
        super::store::save_settings(self)
    }

    /// Sync api_key_configured flags with the actual Credential Manager state.
    ///
    /// This ensures that keys persisted in Windows Credential Manager across
    /// reinstalls are detected even when the config JSON has been reset.
    pub fn sync_api_key_flags(&mut self) {
        self.transcription.groq.api_key_configured = super::SecretsManager::has_groq_api_key();
        self.transcription.openai.api_key_configured = super::SecretsManager::has_openai_api_key();
        self.transcription.deepgram.api_key_configured =
            super::SecretsManager::has_deepgram_api_key();
        self.post_processing.openai.api_key_configured =
            super::SecretsManager::has_openai_api_key();
        self.post_processing.anthropic.api_key_configured =
            super::SecretsManager::has_anthropic_api_key();
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

/// Recording settings constraints
impl RecordingSettings {
    /// Maximum allowed recording duration (30 minutes)
    pub const MAX_DURATION_LIMIT: u32 = 1800;
    /// Maximum silence timeout (60 seconds)
    pub const MAX_SILENCE_TIMEOUT: u32 = 60000;

    /// Validate recording settings
    pub fn validate(&self) -> Result<(), SettingsError> {
        if self.max_duration > Self::MAX_DURATION_LIMIT {
            return Err(SettingsError::InvalidValue(format!(
                "max_duration {} exceeds limit of {} seconds",
                self.max_duration,
                Self::MAX_DURATION_LIMIT
            )));
        }
        if self.silence_timeout > Self::MAX_SILENCE_TIMEOUT {
            return Err(SettingsError::InvalidValue(format!(
                "silence_timeout {} exceeds limit of {} ms",
                self.silence_timeout,
                Self::MAX_SILENCE_TIMEOUT
            )));
        }
        Ok(())
    }

    /// Sanitize recording settings by clamping values to valid ranges
    pub fn sanitize(&mut self) {
        self.max_duration = self.max_duration.min(Self::MAX_DURATION_LIMIT);
        self.silence_timeout = self.silence_timeout.min(Self::MAX_SILENCE_TIMEOUT);
    }
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
    /// OpenAI Whisper API settings
    pub openai: OpenAiTranscriptionSettings,
    /// Deepgram API settings
    pub deepgram: DeepgramSettings,
}

impl Default for TranscriptionSettings {
    fn default() -> Self {
        Self {
            provider: TranscriptionProvider::Local,
            language: "auto".to_string(),
            local: LocalTranscriptionSettings::default(),
            groq: GroqSettings::default(),
            openai: OpenAiTranscriptionSettings::default(),
            deepgram: DeepgramSettings::default(),
        }
    }
}

impl TranscriptionSettings {
    /// Maximum thread count (reasonable limit)
    pub const MAX_THREADS: usize = 64;
    /// Maximum provider timeout (5 minutes)
    pub const MAX_TIMEOUT: u32 = 300;

    /// Validate transcription settings
    pub fn validate(&self) -> Result<(), SettingsError> {
        if self.local.threads > Self::MAX_THREADS {
            return Err(SettingsError::InvalidValue(format!(
                "threads {} exceeds limit of {}",
                self.local.threads,
                Self::MAX_THREADS
            )));
        }
        if self.groq.timeout_seconds == 0 {
            return Err(SettingsError::InvalidValue(
                "groq timeout_seconds cannot be 0".to_string(),
            ));
        }
        if self.groq.timeout_seconds > Self::MAX_TIMEOUT {
            return Err(SettingsError::InvalidValue(format!(
                "groq timeout_seconds {} exceeds limit of {} seconds",
                self.groq.timeout_seconds,
                Self::MAX_TIMEOUT
            )));
        }
        if self.openai.timeout_seconds == 0 {
            return Err(SettingsError::InvalidValue(
                "openai timeout_seconds cannot be 0".to_string(),
            ));
        }
        if self.openai.timeout_seconds > Self::MAX_TIMEOUT {
            return Err(SettingsError::InvalidValue(format!(
                "openai timeout_seconds {} exceeds limit of {} seconds",
                self.openai.timeout_seconds,
                Self::MAX_TIMEOUT
            )));
        }
        if self.deepgram.timeout_seconds == 0 {
            return Err(SettingsError::InvalidValue(
                "deepgram timeout_seconds cannot be 0".to_string(),
            ));
        }
        if self.deepgram.timeout_seconds > Self::MAX_TIMEOUT {
            return Err(SettingsError::InvalidValue(format!(
                "deepgram timeout_seconds {} exceeds limit of {} seconds",
                self.deepgram.timeout_seconds,
                Self::MAX_TIMEOUT
            )));
        }
        Ok(())
    }

    /// Sanitize transcription settings
    pub fn sanitize(&mut self) {
        self.local.threads = self.local.threads.min(Self::MAX_THREADS);
        if self.groq.timeout_seconds == 0 {
            self.groq.timeout_seconds = 30;
        }
        self.groq.timeout_seconds = self.groq.timeout_seconds.min(Self::MAX_TIMEOUT);
        if self.openai.timeout_seconds == 0 {
            self.openai.timeout_seconds = 30;
        }
        self.openai.timeout_seconds = self.openai.timeout_seconds.min(Self::MAX_TIMEOUT);
        if self.deepgram.timeout_seconds == 0 {
            self.deepgram.timeout_seconds = 30;
        }
        self.deepgram.timeout_seconds = self.deepgram.timeout_seconds.min(Self::MAX_TIMEOUT);
    }
}

/// Transcription provider selection
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TranscriptionProvider {
    Local,
    Groq,
    OpenAi,
    Deepgram,
}

/// GPU backend selection for whisper acceleration
///
/// With the single-installer approach, Vulkan is always compiled in.
/// CPU fallback is controlled by the `gpu_enabled` toggle, not by this enum.
/// The `Cuda` variant is kept for backwards compatibility with existing config files
/// but is treated as unavailable and will fall back to Vulkan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GpuBackend {
    /// CPU only (no GPU acceleration)
    Cpu,
    /// Vulkan backend (AMD, Intel, NVIDIA - always available)
    Vulkan,
    /// CUDA backend (deprecated - kept for config compatibility, treated as Vulkan)
    Cuda,
}

impl GpuBackend {
    /// Check if this backend is available in the current build.
    /// Vulkan is always available. CUDA is no longer supported and returns false.
    pub fn is_available(&self) -> bool {
        match self {
            GpuBackend::Cpu => true,
            GpuBackend::Vulkan => true,
            // CUDA is no longer supported in single-installer builds
            GpuBackend::Cuda => false,
        }
    }

    /// Get the effective backend to use.
    /// CUDA configs are migrated to Vulkan at runtime.
    pub fn effective(&self) -> &GpuBackend {
        match self {
            GpuBackend::Cuda => &GpuBackend::Vulkan,
            other => other,
        }
    }

    /// Get display name for the backend
    pub fn display_name(&self) -> &'static str {
        match self {
            GpuBackend::Cpu => "CPU",
            GpuBackend::Vulkan => "Vulkan (AMD/Intel/NVIDIA)",
            GpuBackend::Cuda => "CUDA (deprecated, using Vulkan)",
        }
    }
}

/// Local whisper.cpp settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LocalTranscriptionSettings {
    /// Whisper model size
    pub model: WhisperModel,
    /// Model quantization level (F16, Q8_0, Q5_1)
    pub quantization: ModelQuantization,
    /// Number of CPU threads (0 = auto-detect optimal)
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
            quantization: ModelQuantization::F16,
            threads: 0, // Auto-detect
            gpu_enabled: true,
            gpu_backend: GpuBackend::Vulkan,
        }
    }
}

impl LocalTranscriptionSettings {
    /// Get the full model filename including quantization
    pub fn model_filename(&self) -> String {
        self.model.filename_with_quantization(&self.quantization)
    }

    /// Get estimated model size in bytes
    pub fn estimated_model_size(&self) -> u64 {
        self.model.size_bytes_with_quantization(&self.quantization)
    }
}

/// Quantization type for Whisper models
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ModelQuantization {
    /// Full precision (f16) - highest quality, largest size
    #[default]
    F16,
    /// 8-bit quantization - good quality/size balance
    Q8_0,
    /// 5-bit quantization - smallest size, slightly lower quality
    Q5_1,
}

impl ModelQuantization {
    /// Get the filename suffix for this quantization type
    pub fn filename_suffix(&self) -> &'static str {
        match self {
            Self::F16 => "",
            Self::Q8_0 => "-q8_0",
            Self::Q5_1 => "-q5_1",
        }
    }

    /// Get display name for UI
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::F16 => "Full Precision (F16)",
            Self::Q8_0 => "8-bit Quantized (Q8_0)",
            Self::Q5_1 => "5-bit Quantized (Q5_1)",
        }
    }

    /// Get memory reduction factor compared to F16
    pub fn memory_factor(&self) -> f32 {
        match self {
            Self::F16 => 1.0,
            Self::Q8_0 => 0.5,  // ~50% of original
            Self::Q5_1 => 0.35, // ~35% of original
        }
    }

    /// Get all available quantization types
    pub fn all() -> &'static [ModelQuantization] {
        &[Self::F16, Self::Q8_0, Self::Q5_1]
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
    /// Get model filename (for F16/standard model)
    pub fn filename(&self) -> &'static str {
        match self {
            Self::Tiny => "ggml-tiny.bin",
            Self::Base => "ggml-base.bin",
            Self::Small => "ggml-small.bin",
            Self::Medium => "ggml-medium.bin",
            Self::Large => "ggml-large.bin",
        }
    }

    /// Get model filename with specific quantization
    pub fn filename_with_quantization(&self, quant: &ModelQuantization) -> String {
        let base_name = match self {
            Self::Tiny => "ggml-tiny",
            Self::Base => "ggml-base",
            Self::Small => "ggml-small",
            Self::Medium => "ggml-medium",
            Self::Large => "ggml-large",
        };
        format!("{}{}.bin", base_name, quant.filename_suffix())
    }

    /// Get approximate model size in bytes for F16
    pub fn size_bytes(&self) -> u64 {
        match self {
            Self::Tiny => 75_000_000,
            Self::Base => 142_000_000,
            Self::Small => 466_000_000,
            Self::Medium => 1_500_000_000,
            Self::Large => 2_900_000_000,
        }
    }

    /// Get approximate model size with quantization
    pub fn size_bytes_with_quantization(&self, quant: &ModelQuantization) -> u64 {
        (self.size_bytes() as f64 * quant.memory_factor() as f64) as u64
    }

    /// Get model display name
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Tiny => "Tiny (~75MB)",
            Self::Base => "Base (~142MB)",
            Self::Small => "Small (~466MB)",
            Self::Medium => "Medium (~1.5GB)",
            Self::Large => "Large (~2.9GB)",
        }
    }

    /// Get all model sizes
    pub fn all() -> &'static [WhisperModel] {
        &[
            Self::Tiny,
            Self::Base,
            Self::Small,
            Self::Medium,
            Self::Large,
        ]
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

    /// Check if API key is available in secure storage (Credential Manager)
    pub fn has_api_key(&self) -> bool {
        super::SecretsManager::has_groq_api_key()
    }
}

/// Audio input settings
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AudioSettings {
    /// Input device ID (None = default)
    pub input_device: Option<String>,
    /// Voice Activity Detection settings
    pub vad: VadSettings,
}

impl AudioSettings {
    /// Validate audio settings
    pub fn validate(&self) -> Result<(), SettingsError> {
        self.vad.validate()
    }

    /// Sanitize audio settings
    pub fn sanitize(&mut self) {
        self.vad.sanitize();
    }
}

/// Voice Activity Detection settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct VadSettings {
    /// Enable VAD filtering before transcription
    pub enabled: bool,
    /// VAD aggressiveness (0-3, higher = more aggressive)
    pub aggressiveness: u8,
    /// Minimum speech segment duration in ms
    pub min_speech_duration_ms: u32,
    /// Padding around speech segments in ms
    pub padding_ms: u32,
}

impl Default for VadSettings {
    fn default() -> Self {
        Self {
            enabled: true,     // Enable by default for performance
            aggressiveness: 2, // Aggressive mode
            min_speech_duration_ms: 100,
            padding_ms: 300,
        }
    }
}

impl VadSettings {
    /// Maximum VAD aggressiveness (webrtc-vad limit)
    pub const MAX_AGGRESSIVENESS: u8 = 3;
    /// Maximum speech duration threshold (10 seconds)
    pub const MAX_MIN_SPEECH_DURATION_MS: u32 = 10000;
    /// Maximum padding (5 seconds)
    pub const MAX_PADDING_MS: u32 = 5000;

    /// Validate VAD settings
    pub fn validate(&self) -> Result<(), SettingsError> {
        if self.aggressiveness > Self::MAX_AGGRESSIVENESS {
            return Err(SettingsError::InvalidValue(format!(
                "VAD aggressiveness {} exceeds limit of {}",
                self.aggressiveness,
                Self::MAX_AGGRESSIVENESS
            )));
        }
        if self.min_speech_duration_ms > Self::MAX_MIN_SPEECH_DURATION_MS {
            return Err(SettingsError::InvalidValue(format!(
                "min_speech_duration_ms {} exceeds limit of {} ms",
                self.min_speech_duration_ms,
                Self::MAX_MIN_SPEECH_DURATION_MS
            )));
        }
        if self.padding_ms > Self::MAX_PADDING_MS {
            return Err(SettingsError::InvalidValue(format!(
                "padding_ms {} exceeds limit of {} ms",
                self.padding_ms,
                Self::MAX_PADDING_MS
            )));
        }
        Ok(())
    }

    /// Sanitize VAD settings
    pub fn sanitize(&mut self) {
        self.aggressiveness = self.aggressiveness.min(Self::MAX_AGGRESSIVENESS);
        self.min_speech_duration_ms = self
            .min_speech_duration_ms
            .min(Self::MAX_MIN_SPEECH_DURATION_MS);
        self.padding_ms = self.padding_ms.min(Self::MAX_PADDING_MS);
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

impl OutputSettings {
    /// Maximum paste delay (2 seconds)
    pub const MAX_PASTE_DELAY: u32 = 2000;

    /// Validate output settings
    pub fn validate(&self) -> Result<(), SettingsError> {
        if self.paste_delay > Self::MAX_PASTE_DELAY {
            return Err(SettingsError::InvalidValue(format!(
                "paste_delay {} exceeds limit of {} ms",
                self.paste_delay,
                Self::MAX_PASTE_DELAY
            )));
        }
        Ok(())
    }

    /// Sanitize output settings
    pub fn sanitize(&mut self) {
        self.paste_delay = self.paste_delay.min(Self::MAX_PASTE_DELAY);
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
    /// Auto-start with Windows
    #[serde(default)]
    pub auto_start: bool,
    /// Minimize to tray instead of taskbar
    pub minimize_to_tray: bool,
    /// Automatically install updates without prompting
    #[serde(default)]
    pub auto_update: bool,
    /// Custom theme name (premium feature)
    #[serde(default)]
    pub custom_theme: Option<String>,
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
            show_indicator: true,
            indicator_position: IndicatorPosition::Cursor,
            theme: Theme::System,
            start_minimized: false,
            auto_start: true,
            minimize_to_tray: true,
            auto_update: true,
            custom_theme: None,
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

    #[error("Invalid value: {0}")]
    InvalidValue(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] toml::ser::Error),

    #[error("Deserialization error: {0}")]
    Deserialization(#[from] toml::de::Error),
}

/// Premium license settings (cached state)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PremiumSettings {
    /// Whether premium is active (cached)
    pub is_premium: bool,
    /// License expiration timestamp
    pub expires_at: Option<u64>,
    /// Last successful validation timestamp
    pub last_validated: Option<u64>,
    /// Credit balance cached state
    pub credits: CreditsCacheSettings,
}

/// Cached credit state in settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CreditsCacheSettings {
    /// Cached balance in EUR
    pub balance_eur: f64,
    /// Low balance threshold
    pub low_threshold_eur: f64,
}

impl Default for CreditsCacheSettings {
    fn default() -> Self {
        Self {
            balance_eur: 0.0,
            low_threshold_eur: 5.0,
        }
    }
}

/// Multi-context transcription settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ContextsSettings {
    /// Active context ID
    pub active_context: String,
    /// All defined contexts
    pub contexts: Vec<TranscriptionContext>,
}

impl Default for ContextsSettings {
    fn default() -> Self {
        Self {
            active_context: "default".to_string(),
            contexts: vec![TranscriptionContext::default()],
        }
    }
}

/// A transcription context with its own settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionContext {
    /// Unique identifier
    pub id: String,
    /// Display name
    pub name: String,
    /// Keyboard shortcut
    pub shortcut: String,
    /// Language override (ISO 639-1 or "auto")
    pub language: String,
    /// Provider override
    pub provider: TranscriptionProvider,
    /// Model override
    pub model: Option<String>,
    /// LLM post-processing config for this context
    pub post_processing: Option<ContextPostProcessing>,
    /// Badge color (hex)
    pub color: Option<String>,
    /// Icon emoji
    pub icon: Option<String>,
    /// Custom vocabulary words (comma-separated) used as Whisper prompt
    #[serde(default)]
    pub custom_vocabulary: Option<String>,
    /// App patterns for auto-switching (process names, case-insensitive)
    #[serde(default)]
    pub app_patterns: Vec<String>,
}

impl Default for TranscriptionContext {
    fn default() -> Self {
        Self {
            id: "default".to_string(),
            name: "Default".to_string(),
            shortcut: String::new(),
            language: "auto".to_string(),
            provider: TranscriptionProvider::Local,
            model: None,
            post_processing: None,
            color: None,
            icon: None,
            custom_vocabulary: None,
            app_patterns: Vec::new(),
        }
    }
}

/// Per-context post-processing config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextPostProcessing {
    /// Whether post-processing is enabled
    pub enabled: bool,
    /// LLM provider to use
    pub llm_provider: LlmProviderType,
    /// System prompt for the LLM
    pub system_prompt: String,
}

/// Post-processing settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PostProcessingSettings {
    /// Whether post-processing is globally enabled
    pub enabled: bool,
    /// Default LLM provider
    pub default_provider: LlmProviderType,
    /// Default system prompt
    pub default_prompt: String,
    /// OpenAI settings
    pub openai: OpenAiLlmSettings,
    /// Anthropic settings
    pub anthropic: AnthropicLlmSettings,
    /// Groq LLM settings
    pub groq_llm: GroqLlmSettings,
    /// Automatically remove filler words (um, uh, etc.)
    #[serde(default)]
    pub remove_filler_words: bool,
}

impl Default for PostProcessingSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            default_provider: LlmProviderType::GroqLlm,
            default_prompt: "Clean up and fix any errors in this transcription. Keep the original meaning and tone. Output only the corrected text.".to_string(),
            openai: OpenAiLlmSettings::default(),
            anthropic: AnthropicLlmSettings::default(),
            groq_llm: GroqLlmSettings::default(),
            remove_filler_words: false,
        }
    }
}

/// LLM provider types for post-processing
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LlmProviderType {
    OpenAi,
    Anthropic,
    GroqLlm,
}

/// OpenAI LLM settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct OpenAiLlmSettings {
    pub api_key_configured: bool,
    pub model: String,
}

impl Default for OpenAiLlmSettings {
    fn default() -> Self {
        Self {
            api_key_configured: false,
            model: "gpt-4o-mini".to_string(),
        }
    }
}

/// Anthropic LLM settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AnthropicLlmSettings {
    pub api_key_configured: bool,
    pub model: String,
}

impl Default for AnthropicLlmSettings {
    fn default() -> Self {
        Self {
            api_key_configured: false,
            model: "claude-haiku-4-5-20251001".to_string(),
        }
    }
}

/// Groq LLM settings (for post-processing, not transcription)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GroqLlmSettings {
    pub model: String,
}

impl Default for GroqLlmSettings {
    fn default() -> Self {
        Self {
            model: "llama-3.1-8b-instant".to_string(),
        }
    }
}

/// OpenAI Whisper transcription settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct OpenAiTranscriptionSettings {
    pub api_key_configured: bool,
    pub model: String,
    pub timeout_seconds: u32,
}

impl Default for OpenAiTranscriptionSettings {
    fn default() -> Self {
        Self {
            api_key_configured: false,
            model: "whisper-1".to_string(),
            timeout_seconds: 30,
        }
    }
}

/// Deepgram transcription settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DeepgramSettings {
    pub api_key_configured: bool,
    pub model: String,
    pub timeout_seconds: u32,
}

impl Default for DeepgramSettings {
    fn default() -> Self {
        Self {
            api_key_configured: false,
            model: "nova-2".to_string(),
            timeout_seconds: 30,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_settings() {
        let settings = Settings::default();

        assert_eq!(settings.recording.mode, RecordingMode::PushToTalk);
        assert_eq!(settings.recording.max_duration, 300);
        assert_eq!(settings.shortcuts.record, "Ctrl+Space");
        assert_eq!(
            settings.transcription.provider,
            TranscriptionProvider::Local
        );
        assert_eq!(settings.transcription.language, "auto");
        assert!(settings.ui.show_indicator);
    }

    #[test]
    fn test_recording_settings_validation_valid() {
        let settings = RecordingSettings {
            mode: RecordingMode::Toggle,
            max_duration: 600,
            silence_timeout: 5000,
        };
        assert!(settings.validate().is_ok());
    }

    #[test]
    fn test_recording_settings_validation_invalid_max_duration() {
        let settings = RecordingSettings {
            mode: RecordingMode::PushToTalk,
            max_duration: 10000, // Exceeds limit
            silence_timeout: 0,
        };
        assert!(settings.validate().is_err());
    }

    #[test]
    fn test_recording_settings_sanitize() {
        let mut settings = RecordingSettings {
            mode: RecordingMode::Toggle,
            max_duration: 10000,     // Should be clamped
            silence_timeout: 100000, // Should be clamped
        };
        settings.sanitize();

        assert_eq!(settings.max_duration, RecordingSettings::MAX_DURATION_LIMIT);
        assert_eq!(
            settings.silence_timeout,
            RecordingSettings::MAX_SILENCE_TIMEOUT
        );
    }

    #[test]
    fn test_whisper_model_filename() {
        assert_eq!(WhisperModel::Tiny.filename(), "ggml-tiny.bin");
        assert_eq!(WhisperModel::Base.filename(), "ggml-base.bin");
        assert_eq!(WhisperModel::Small.filename(), "ggml-small.bin");
        assert_eq!(WhisperModel::Medium.filename(), "ggml-medium.bin");
        assert_eq!(WhisperModel::Large.filename(), "ggml-large.bin");
    }

    #[test]
    fn test_whisper_model_filename_with_quantization() {
        assert_eq!(
            WhisperModel::Small.filename_with_quantization(&ModelQuantization::F16),
            "ggml-small.bin"
        );
        assert_eq!(
            WhisperModel::Small.filename_with_quantization(&ModelQuantization::Q8_0),
            "ggml-small-q8_0.bin"
        );
        assert_eq!(
            WhisperModel::Small.filename_with_quantization(&ModelQuantization::Q5_1),
            "ggml-small-q5_1.bin"
        );
    }

    #[test]
    fn test_whisper_model_size_bytes() {
        assert!(WhisperModel::Tiny.size_bytes() < WhisperModel::Base.size_bytes());
        assert!(WhisperModel::Base.size_bytes() < WhisperModel::Small.size_bytes());
        assert!(WhisperModel::Small.size_bytes() < WhisperModel::Medium.size_bytes());
        assert!(WhisperModel::Medium.size_bytes() < WhisperModel::Large.size_bytes());
    }

    #[test]
    fn test_quantization_memory_factor() {
        assert_eq!(ModelQuantization::F16.memory_factor(), 1.0);
        assert!(ModelQuantization::Q8_0.memory_factor() < 1.0);
        assert!(ModelQuantization::Q5_1.memory_factor() < ModelQuantization::Q8_0.memory_factor());
    }

    #[test]
    fn test_vad_settings_validation_valid() {
        let settings = VadSettings {
            enabled: true,
            aggressiveness: 2,
            min_speech_duration_ms: 100,
            padding_ms: 300,
        };
        assert!(settings.validate().is_ok());
    }

    #[test]
    fn test_vad_settings_validation_invalid_aggressiveness() {
        let settings = VadSettings {
            enabled: true,
            aggressiveness: 10, // Exceeds limit
            min_speech_duration_ms: 100,
            padding_ms: 300,
        };
        assert!(settings.validate().is_err());
    }

    #[test]
    fn test_vad_settings_sanitize() {
        let mut settings = VadSettings {
            enabled: true,
            aggressiveness: 10,
            min_speech_duration_ms: 100000,
            padding_ms: 100000,
        };
        settings.sanitize();

        assert_eq!(settings.aggressiveness, VadSettings::MAX_AGGRESSIVENESS);
        assert_eq!(
            settings.min_speech_duration_ms,
            VadSettings::MAX_MIN_SPEECH_DURATION_MS
        );
        assert_eq!(settings.padding_ms, VadSettings::MAX_PADDING_MS);
    }

    #[test]
    fn test_output_settings_validation_valid() {
        let settings = OutputSettings {
            auto_capitalize: true,
            auto_punctuation: true,
            paste_delay: 100,
        };
        assert!(settings.validate().is_ok());
    }

    #[test]
    fn test_output_settings_validation_invalid_paste_delay() {
        let settings = OutputSettings {
            auto_capitalize: true,
            auto_punctuation: true,
            paste_delay: 10000, // Exceeds limit
        };
        assert!(settings.validate().is_err());
    }

    #[test]
    fn test_transcription_settings_validation_invalid_threads() {
        let mut settings = TranscriptionSettings::default();
        settings.local.threads = 100; // Exceeds limit
        assert!(settings.validate().is_err());
    }

    #[test]
    fn test_transcription_settings_validation_invalid_groq_timeout() {
        let mut settings = TranscriptionSettings::default();
        settings.groq.timeout_seconds = 0;
        assert!(settings.validate().is_err());

        settings.groq.timeout_seconds = 1000; // Exceeds limit
        assert!(settings.validate().is_err());
    }

    #[test]
    fn test_transcription_settings_sanitize() {
        let mut settings = TranscriptionSettings::default();
        settings.local.threads = 100;
        settings.groq.timeout_seconds = 0;
        settings.sanitize();

        assert_eq!(settings.local.threads, TranscriptionSettings::MAX_THREADS);
        assert_eq!(settings.groq.timeout_seconds, 30); // Reset to default
    }

    #[test]
    fn test_settings_sanitize_full() {
        let mut settings = Settings::default();
        settings.recording.max_duration = 10000;
        settings.audio.vad.aggressiveness = 10;
        settings.output.paste_delay = 10000;
        settings.transcription.local.threads = 100;

        let sanitized = settings.sanitize();

        assert_eq!(
            sanitized.recording.max_duration,
            RecordingSettings::MAX_DURATION_LIMIT
        );
        assert_eq!(
            sanitized.audio.vad.aggressiveness,
            VadSettings::MAX_AGGRESSIVENESS
        );
        assert_eq!(
            sanitized.output.paste_delay,
            OutputSettings::MAX_PASTE_DELAY
        );
        assert_eq!(
            sanitized.transcription.local.threads,
            TranscriptionSettings::MAX_THREADS
        );
    }

    #[test]
    fn test_gpu_backend_cpu_always_available() {
        assert!(GpuBackend::Cpu.is_available());
    }

    #[test]
    fn test_gpu_backend_display_names() {
        assert_eq!(GpuBackend::Cpu.display_name(), "CPU");
        assert!(!GpuBackend::Vulkan.display_name().is_empty());
        assert!(!GpuBackend::Cuda.display_name().is_empty());
    }

    #[test]
    fn test_recording_mode_serialization() {
        let push = RecordingMode::PushToTalk;
        let toggle = RecordingMode::Toggle;

        let push_json = serde_json::to_string(&push).unwrap();
        let toggle_json = serde_json::to_string(&toggle).unwrap();

        assert_eq!(push_json, "\"push-to-talk\"");
        assert_eq!(toggle_json, "\"toggle\"");
    }

    #[test]
    fn test_theme_serialization() {
        let system = Theme::System;
        let light = Theme::Light;
        let dark = Theme::Dark;

        assert_eq!(serde_json::to_string(&system).unwrap(), "\"system\"");
        assert_eq!(serde_json::to_string(&light).unwrap(), "\"light\"");
        assert_eq!(serde_json::to_string(&dark).unwrap(), "\"dark\"");
    }

    #[test]
    fn test_indicator_position_serialization() {
        assert_eq!(
            serde_json::to_string(&IndicatorPosition::Cursor).unwrap(),
            "\"cursor\""
        );
        assert_eq!(
            serde_json::to_string(&IndicatorPosition::Center).unwrap(),
            "\"center\""
        );
        assert_eq!(
            serde_json::to_string(&IndicatorPosition::Corner).unwrap(),
            "\"corner\""
        );
    }
}
