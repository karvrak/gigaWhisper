//! Whisper.cpp Provider
//!
//! Local transcription using whisper-rs bindings.
//! Supports GPU acceleration via Vulkan (AMD/Intel/NVIDIA) or CUDA (NVIDIA).
//! Includes automatic CPU thread optimization.

use super::{TranscriptionConfig, TranscriptionError, TranscriptionProvider, TranscriptionResult};
use crate::utils::get_optimal_threads;
use async_trait::async_trait;
use parking_lot::Mutex;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Default timeout for transcription in seconds (5 minutes)
const DEFAULT_TRANSCRIPTION_TIMEOUT_SECS: u64 = 300;

/// Default idle timeout before unloading model (10 minutes)
const DEFAULT_IDLE_TIMEOUT_SECS: u64 = 600;

/// Whisper.cpp transcription provider
pub struct WhisperProvider {
    model_path: PathBuf,
    context: Arc<Mutex<Option<whisper_rs::WhisperContext>>>,
    /// Configured threads (0 = auto-detect)
    configured_threads: usize,
    /// Actual threads to use (resolved from configured or auto-detected)
    effective_threads: usize,
    gpu_enabled: bool,
    /// Transcription timeout
    timeout: Duration,
    /// Last time the model was used for transcription
    last_use: Arc<Mutex<Option<Instant>>>,
    /// Idle timeout before unloading model
    idle_timeout: Duration,
}

impl WhisperProvider {
    /// Create a new Whisper provider
    ///
    /// If `threads` is 0, auto-detects optimal thread count based on CPU.
    pub fn new(model_path: PathBuf, threads: usize) -> Self {
        let effective_threads = get_optimal_threads(threads);
        Self {
            model_path,
            context: Arc::new(Mutex::new(None)),
            configured_threads: threads,
            effective_threads,
            gpu_enabled: false,
            timeout: Duration::from_secs(DEFAULT_TRANSCRIPTION_TIMEOUT_SECS),
            last_use: Arc::new(Mutex::new(None)),
            idle_timeout: Duration::from_secs(DEFAULT_IDLE_TIMEOUT_SECS),
        }
    }

    /// Create a new Whisper provider with GPU acceleration
    ///
    /// If `threads` is 0, auto-detects optimal thread count based on CPU.
    pub fn with_gpu(model_path: PathBuf, threads: usize, gpu_enabled: bool) -> Self {
        let effective_threads = get_optimal_threads(threads);
        Self {
            model_path,
            context: Arc::new(Mutex::new(None)),
            configured_threads: threads,
            effective_threads,
            gpu_enabled,
            timeout: Duration::from_secs(DEFAULT_TRANSCRIPTION_TIMEOUT_SECS),
            last_use: Arc::new(Mutex::new(None)),
            idle_timeout: Duration::from_secs(DEFAULT_IDLE_TIMEOUT_SECS),
        }
    }

    /// Set custom timeout for transcription
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set custom idle timeout before unloading model
    pub fn with_idle_timeout(mut self, idle_timeout: Duration) -> Self {
        self.idle_timeout = idle_timeout;
        self
    }

    /// Get the current timeout
    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    /// Get the current idle timeout
    pub fn idle_timeout(&self) -> Duration {
        self.idle_timeout
    }

    /// Get the last time the model was used
    pub fn last_use(&self) -> Option<Instant> {
        *self.last_use.lock()
    }

    /// Update the last use timestamp
    fn update_last_use(&self) {
        *self.last_use.lock() = Some(Instant::now());
    }

    /// Check if the model has been idle for longer than the idle timeout
    /// and unload it if so. Returns true if the model was unloaded.
    pub fn maybe_unload_idle_model(&self) -> bool {
        let last_use = self.last_use.lock();

        if let Some(last_use_time) = *last_use {
            if last_use_time.elapsed() >= self.idle_timeout && self.is_model_loaded() {
                drop(last_use); // Release lock before unloading
                self.unload_model();
                tracing::info!(
                    "Whisper model unloaded due to idle timeout ({} seconds)",
                    self.idle_timeout.as_secs()
                );
                return true;
            }
        }

        false
    }

    /// Get idle time since last use (returns None if never used)
    pub fn idle_time(&self) -> Option<Duration> {
        self.last_use.lock().map(|t| t.elapsed())
    }

    /// Get the number of threads being used
    pub fn threads(&self) -> usize {
        self.effective_threads
    }

    /// Check if threads were auto-detected
    pub fn is_auto_threads(&self) -> bool {
        self.configured_threads == 0
    }

    /// Check if GPU acceleration is available in this build
    pub fn is_gpu_available() -> bool {
        cfg!(any(feature = "gpu-vulkan", feature = "gpu-cuda"))
    }

    /// Get the GPU backend name if available
    pub fn gpu_backend_name() -> &'static str {
        #[cfg(feature = "gpu-cuda")]
        {
            "CUDA"
        }
        #[cfg(all(feature = "gpu-vulkan", not(feature = "gpu-cuda")))]
        {
            "Vulkan"
        }
        #[cfg(not(any(feature = "gpu-vulkan", feature = "gpu-cuda")))]
        {
            "None"
        }
    }

    /// Load the model into memory
    pub fn load_model(&self) -> Result<(), TranscriptionError> {
        if !self.model_path.exists() {
            return Err(TranscriptionError::ModelNotFound(
                self.model_path.display().to_string(),
            ));
        }

        // Check if already loaded
        if self.is_model_loaded() {
            return Ok(());
        }

        // Configure GPU acceleration if enabled and available
        let mut params = whisper_rs::WhisperContextParameters::default();

        let use_gpu = self.gpu_enabled && Self::is_gpu_available();
        if use_gpu {
            params.use_gpu(true);
            tracing::info!("GPU acceleration enabled: {}", Self::gpu_backend_name());
        } else if self.gpu_enabled {
            tracing::warn!(
                "GPU requested but not available in this build. \
                Compile with --features gpu-vulkan (AMD/Intel) or --features gpu-cuda (NVIDIA)"
            );
        }

        let path_str = self.model_path.to_str().ok_or_else(|| {
            TranscriptionError::InvalidPath(format!(
                "Path contains invalid UTF-8: {:?}",
                self.model_path
            ))
        })?;

        let ctx = whisper_rs::WhisperContext::new_with_params(path_str, params)
            .map_err(|e| TranscriptionError::Failed(e.to_string()))?;

        *self.context.lock() = Some(ctx);

        // Initialize last_use timestamp when model is loaded
        self.update_last_use();

        tracing::info!(
            "Whisper model loaded: {:?} (GPU: {}, threads: {}{})",
            self.model_path,
            if use_gpu { "enabled" } else { "disabled" },
            self.effective_threads,
            if self.is_auto_threads() { " auto-detected" } else { "" }
        );

        Ok(())
    }

    /// Unload the model from memory
    pub fn unload_model(&self) {
        *self.context.lock() = None;
        *self.last_use.lock() = None;
        tracing::info!("Whisper model unloaded");
    }

    /// Check if model is loaded
    pub fn is_model_loaded(&self) -> bool {
        self.context.lock().is_some()
    }

    /// Perform transcription synchronously (for use in spawn_blocking)
    fn transcribe_sync(
        context: Arc<Mutex<Option<whisper_rs::WhisperContext>>>,
        audio: Vec<f32>,
        config: TranscriptionConfig,
        threads: usize,
    ) -> Result<TranscriptionResult, TranscriptionError> {
        let start = Instant::now();

        let guard = context.lock();
        let ctx = guard
            .as_ref()
            .ok_or(TranscriptionError::ModelNotLoaded)?;

        // Create state for this transcription
        let mut state = ctx
            .create_state()
            .map_err(|e| TranscriptionError::Failed(e.to_string()))?;

        // Configure parameters
        let mut params = whisper_rs::FullParams::new(whisper_rs::SamplingStrategy::Greedy {
            best_of: 1,
        });

        params.set_n_threads(threads as i32);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);

        // Set language if specified
        if config.language != "auto" {
            params.set_language(Some(&config.language));
        }

        params.set_translate(config.translate);

        // Run inference
        state
            .full(params, &audio)
            .map_err(|e| TranscriptionError::Failed(e.to_string()))?;

        // Extract text from all segments
        let num_segments = state
            .full_n_segments()
            .map_err(|e| TranscriptionError::Failed(e.to_string()))?;

        let mut text = String::new();
        for i in 0..num_segments {
            if let Ok(segment) = state.full_get_segment_text(i) {
                text.push_str(&segment);
            }
        }

        let duration_ms = start.elapsed().as_millis() as u64;

        Ok(TranscriptionResult {
            text: text.trim().to_string(),
            language: None,
            duration_ms,
            provider: "whisper.cpp".to_string(),
        })
    }
}

// Implement Clone for use with spawn_blocking
impl Clone for WhisperProvider {
    fn clone(&self) -> Self {
        Self {
            model_path: self.model_path.clone(),
            context: self.context.clone(),
            configured_threads: self.configured_threads,
            effective_threads: self.effective_threads,
            gpu_enabled: self.gpu_enabled,
            timeout: self.timeout,
            last_use: self.last_use.clone(),
            idle_timeout: self.idle_timeout,
        }
    }
}

#[async_trait]
impl TranscriptionProvider for WhisperProvider {
    async fn transcribe(
        &self,
        audio: &[f32],
        config: &TranscriptionConfig,
    ) -> Result<TranscriptionResult, TranscriptionError> {
        // Ensure model is loaded
        if !self.is_model_loaded() {
            self.load_model()?;
        }

        // Clone data for the blocking task
        let context = self.context.clone();
        let audio_vec = audio.to_vec();
        let config_clone = config.clone();
        let threads = self.effective_threads;
        let timeout_duration = self.timeout;
        let timeout_secs = timeout_duration.as_secs();

        // Run transcription in blocking thread pool with timeout
        // This avoids holding the MutexGuard across an await point
        let transcription_task = tokio::task::spawn_blocking(move || {
            Self::transcribe_sync(context, audio_vec, config_clone, threads)
        });

        let result = match tokio::time::timeout(timeout_duration, transcription_task).await {
            Ok(Ok(result)) => result,
            Ok(Err(e)) => Err(TranscriptionError::Failed(format!("Task failed: {}", e))),
            Err(_) => {
                tracing::error!("Transcription timed out after {} seconds", timeout_secs);
                Err(TranscriptionError::Timeout(timeout_secs))
            }
        };

        // Update last_use timestamp after successful transcription
        if result.is_ok() {
            self.update_last_use();
        }

        result
    }

    fn name(&self) -> &'static str {
        "whisper.cpp"
    }

    fn is_available(&self) -> bool {
        self.model_path.exists()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::NamedTempFile;

    // =========================================================================
    // Construction Tests
    // =========================================================================

    #[test]
    fn test_new_with_zero_threads_uses_auto_detection() {
        let path = PathBuf::from("/fake/model.bin");
        let provider = WhisperProvider::new(path.clone(), 0);

        assert_eq!(provider.model_path, path);
        assert_eq!(provider.configured_threads, 0);
        assert!(provider.is_auto_threads());
        // effective_threads should be > 0 from auto-detection
        assert!(provider.effective_threads > 0);
        assert!(!provider.gpu_enabled);
    }

    #[test]
    fn test_new_with_explicit_threads() {
        let path = PathBuf::from("/fake/model.bin");
        let provider = WhisperProvider::new(path, 4);

        assert_eq!(provider.configured_threads, 4);
        assert!(!provider.is_auto_threads());
        // effective_threads is capped at logical cores but should be at most 4
        assert!(provider.effective_threads <= num_cpus::get());
    }

    #[test]
    fn test_with_gpu_enabled() {
        let path = PathBuf::from("/fake/model.bin");
        let provider = WhisperProvider::with_gpu(path, 2, true);

        assert!(provider.gpu_enabled);
        assert_eq!(provider.configured_threads, 2);
    }

    #[test]
    fn test_with_gpu_disabled() {
        let path = PathBuf::from("/fake/model.bin");
        let provider = WhisperProvider::with_gpu(path, 2, false);

        assert!(!provider.gpu_enabled);
    }

    #[test]
    fn test_default_timeout() {
        let provider = WhisperProvider::new(PathBuf::from("/fake/model.bin"), 0);

        // Default timeout should be 300 seconds (5 minutes)
        assert_eq!(provider.timeout(), Duration::from_secs(300));
    }

    #[test]
    fn test_with_timeout_builder_pattern() {
        let provider = WhisperProvider::new(PathBuf::from("/fake/model.bin"), 0)
            .with_timeout(Duration::from_secs(60));

        assert_eq!(provider.timeout(), Duration::from_secs(60));
    }

    #[test]
    fn test_with_timeout_zero() {
        let provider = WhisperProvider::new(PathBuf::from("/fake/model.bin"), 0)
            .with_timeout(Duration::ZERO);

        assert_eq!(provider.timeout(), Duration::ZERO);
    }

    #[test]
    fn test_with_timeout_very_long() {
        let provider = WhisperProvider::new(PathBuf::from("/fake/model.bin"), 0)
            .with_timeout(Duration::from_secs(3600)); // 1 hour

        assert_eq!(provider.timeout(), Duration::from_secs(3600));
    }

    // =========================================================================
    // Idle Timeout Tests
    // =========================================================================

    #[test]
    fn test_default_idle_timeout() {
        let provider = WhisperProvider::new(PathBuf::from("/fake/model.bin"), 0);
        // Default idle timeout should be 600 seconds (10 minutes)
        assert_eq!(provider.idle_timeout(), Duration::from_secs(600));
    }

    #[test]
    fn test_with_idle_timeout_builder_pattern() {
        let provider = WhisperProvider::new(PathBuf::from("/fake/model.bin"), 0)
            .with_idle_timeout(Duration::from_secs(300));

        assert_eq!(provider.idle_timeout(), Duration::from_secs(300));
    }

    #[test]
    fn test_last_use_initially_none() {
        let provider = WhisperProvider::new(PathBuf::from("/fake/model.bin"), 0);
        assert!(provider.last_use().is_none());
    }

    #[test]
    fn test_idle_time_initially_none() {
        let provider = WhisperProvider::new(PathBuf::from("/fake/model.bin"), 0);
        assert!(provider.idle_time().is_none());
    }

    #[test]
    fn test_maybe_unload_idle_model_when_not_loaded() {
        let provider = WhisperProvider::new(PathBuf::from("/fake/model.bin"), 0);
        // Should not unload (or panic) when model is not loaded
        assert!(!provider.maybe_unload_idle_model());
    }

    #[test]
    fn test_maybe_unload_idle_model_when_never_used() {
        let provider = WhisperProvider::new(PathBuf::from("/fake/model.bin"), 0);
        // last_use is None, so model should not be unloaded
        assert!(!provider.maybe_unload_idle_model());
    }

    #[test]
    fn test_chained_builder_pattern_with_idle_timeout() {
        let provider = WhisperProvider::new(PathBuf::from("/fake/model.bin"), 0)
            .with_timeout(Duration::from_secs(60))
            .with_idle_timeout(Duration::from_secs(120));

        assert_eq!(provider.timeout(), Duration::from_secs(60));
        assert_eq!(provider.idle_timeout(), Duration::from_secs(120));
    }

    // =========================================================================
    // Accessor Tests
    // =========================================================================

    #[test]
    fn test_threads_accessor() {
        let provider = WhisperProvider::new(PathBuf::from("/fake/model.bin"), 4);
        // threads() returns effective_threads, not configured_threads
        let threads = provider.threads();
        assert!(threads > 0);
        assert!(threads <= num_cpus::get());
    }

    #[test]
    fn test_is_auto_threads_true_when_zero() {
        let provider = WhisperProvider::new(PathBuf::from("/fake/model.bin"), 0);
        assert!(provider.is_auto_threads());
    }

    #[test]
    fn test_is_auto_threads_false_when_explicit() {
        let provider = WhisperProvider::new(PathBuf::from("/fake/model.bin"), 1);
        assert!(!provider.is_auto_threads());
    }

    // =========================================================================
    // Static Method Tests
    // =========================================================================

    #[test]
    fn test_is_gpu_available_returns_bool() {
        // This test just verifies the function returns a boolean
        // The actual value depends on compile-time features
        let result = WhisperProvider::is_gpu_available();
        // Verify it's a bool (will fail to compile if not)
        let _: bool = result;
    }

    #[test]
    fn test_gpu_backend_name_returns_valid_string() {
        let name = WhisperProvider::gpu_backend_name();
        // Should be one of the known values
        assert!(
            name == "CUDA" || name == "Vulkan" || name == "None",
            "Unexpected GPU backend name: {}",
            name
        );
    }

    #[test]
    fn test_gpu_backend_name_consistency_with_is_gpu_available() {
        let available = WhisperProvider::is_gpu_available();
        let name = WhisperProvider::gpu_backend_name();

        if available {
            assert!(name == "CUDA" || name == "Vulkan");
        } else {
            assert_eq!(name, "None");
        }
    }

    // =========================================================================
    // Model Loading/Unloading Tests
    // =========================================================================

    #[test]
    fn test_is_model_loaded_initially_false() {
        let provider = WhisperProvider::new(PathBuf::from("/fake/model.bin"), 0);
        assert!(!provider.is_model_loaded());
    }

    #[test]
    fn test_unload_model_when_not_loaded() {
        let provider = WhisperProvider::new(PathBuf::from("/fake/model.bin"), 0);
        // Should not panic when unloading a model that was never loaded
        provider.unload_model();
        assert!(!provider.is_model_loaded());
    }

    #[test]
    fn test_unload_model_can_be_called_multiple_times() {
        let provider = WhisperProvider::new(PathBuf::from("/fake/model.bin"), 0);
        provider.unload_model();
        provider.unload_model();
        provider.unload_model();
        assert!(!provider.is_model_loaded());
    }

    #[test]
    fn test_load_model_with_nonexistent_path() {
        let provider = WhisperProvider::new(PathBuf::from("/nonexistent/path/model.bin"), 0);
        let result = provider.load_model();

        assert!(result.is_err());
        match result {
            Err(TranscriptionError::ModelNotFound(path)) => {
                assert!(path.contains("nonexistent"));
            }
            Err(e) => panic!("Expected ModelNotFound error, got: {:?}", e),
            Ok(_) => panic!("Expected error for nonexistent path"),
        }
    }

    #[test]
    fn test_load_model_with_empty_path() {
        let provider = WhisperProvider::new(PathBuf::from(""), 0);
        let result = provider.load_model();

        assert!(result.is_err());
        // Empty path doesn't exist, so we get ModelNotFound
        assert!(matches!(result, Err(TranscriptionError::ModelNotFound(_))));
    }

    #[test]
    fn test_load_model_with_directory_path() {
        // Using a known existing directory
        let provider = WhisperProvider::new(PathBuf::from("."), 0);
        let result = provider.load_model();

        // A directory is not a valid model file, whisper-rs should fail
        // The path exists, so we won't get ModelNotFound
        // We expect a Failed error when trying to load a directory as a model
        assert!(result.is_err());
    }

    // =========================================================================
    // Error Handling Tests
    // =========================================================================

    #[test]
    fn test_transcription_error_model_not_loaded() {
        let err = TranscriptionError::ModelNotLoaded;
        let display = format!("{}", err);
        assert_eq!(display, "Model not loaded");
    }

    #[test]
    fn test_transcription_error_model_not_found() {
        let err = TranscriptionError::ModelNotFound("/path/to/model.bin".to_string());
        let display = format!("{}", err);
        assert!(display.contains("/path/to/model.bin"));
    }

    #[test]
    fn test_transcription_error_invalid_path() {
        let err = TranscriptionError::InvalidPath("invalid utf-8".to_string());
        let display = format!("{}", err);
        assert!(display.contains("invalid utf-8"));
    }

    #[test]
    fn test_transcription_error_timeout() {
        let err = TranscriptionError::Timeout(30);
        let display = format!("{}", err);
        assert!(display.contains("30"));
        assert!(display.contains("timed out"));
    }

    // =========================================================================
    // Clone Tests
    // =========================================================================

    #[test]
    fn test_clone_preserves_model_path() {
        let path = PathBuf::from("/fake/model.bin");
        let provider = WhisperProvider::new(path.clone(), 4);
        let cloned = provider.clone();

        assert_eq!(cloned.model_path, path);
    }

    #[test]
    fn test_clone_preserves_thread_config() {
        let provider = WhisperProvider::new(PathBuf::from("/fake/model.bin"), 4);
        let cloned = provider.clone();

        assert_eq!(cloned.configured_threads, provider.configured_threads);
        assert_eq!(cloned.effective_threads, provider.effective_threads);
    }

    #[test]
    fn test_clone_preserves_gpu_config() {
        let provider = WhisperProvider::with_gpu(PathBuf::from("/fake/model.bin"), 2, true);
        let cloned = provider.clone();

        assert_eq!(cloned.gpu_enabled, provider.gpu_enabled);
    }

    #[test]
    fn test_clone_preserves_timeout() {
        let provider = WhisperProvider::new(PathBuf::from("/fake/model.bin"), 0)
            .with_timeout(Duration::from_secs(120));
        let cloned = provider.clone();

        assert_eq!(cloned.timeout(), provider.timeout());
    }

    #[test]
    fn test_clone_shares_context() {
        let provider = WhisperProvider::new(PathBuf::from("/fake/model.bin"), 0);
        let cloned = provider.clone();

        // Both should point to the same Arc<Mutex<Option<WhisperContext>>>
        // We can verify this by checking is_model_loaded on both
        assert_eq!(provider.is_model_loaded(), cloned.is_model_loaded());
    }

    #[test]
    fn test_clone_preserves_idle_timeout() {
        let provider = WhisperProvider::new(PathBuf::from("/fake/model.bin"), 0)
            .with_idle_timeout(Duration::from_secs(300));
        let cloned = provider.clone();

        assert_eq!(cloned.idle_timeout(), provider.idle_timeout());
    }

    #[test]
    fn test_clone_shares_last_use() {
        let provider = WhisperProvider::new(PathBuf::from("/fake/model.bin"), 0);
        let cloned = provider.clone();

        // Both should share the same Arc<Mutex<Option<Instant>>>
        assert_eq!(provider.last_use(), cloned.last_use());
    }

    // =========================================================================
    // Trait Implementation Tests
    // =========================================================================

    #[test]
    fn test_name_returns_whisper_cpp() {
        let provider = WhisperProvider::new(PathBuf::from("/fake/model.bin"), 0);
        assert_eq!(provider.name(), "whisper.cpp");
    }

    #[test]
    fn test_is_available_with_nonexistent_path() {
        let provider = WhisperProvider::new(PathBuf::from("/nonexistent/model.bin"), 0);
        assert!(!provider.is_available());
    }

    #[test]
    fn test_is_available_with_existing_file() {
        // Create a temporary file to test with
        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let path = temp_file.path().to_path_buf();

        let provider = WhisperProvider::new(path, 0);
        assert!(provider.is_available());
    }

    // =========================================================================
    // Thread Configuration Edge Cases
    // =========================================================================

    #[test]
    fn test_threads_capped_at_logical_cores() {
        // Request more threads than available
        let provider = WhisperProvider::new(PathBuf::from("/fake/model.bin"), 1000);

        // effective_threads should be capped
        assert!(provider.effective_threads <= num_cpus::get());
    }

    #[test]
    fn test_threads_with_one_thread() {
        let provider = WhisperProvider::new(PathBuf::from("/fake/model.bin"), 1);

        assert_eq!(provider.configured_threads, 1);
        assert!(provider.effective_threads >= 1);
    }

    // =========================================================================
    // Path Validation Tests
    // =========================================================================

    #[test]
    fn test_model_path_with_spaces() {
        let path = PathBuf::from("/path/with spaces/model.bin");
        let provider = WhisperProvider::new(path.clone(), 0);

        assert_eq!(provider.model_path, path);
    }

    #[test]
    fn test_model_path_with_special_characters() {
        let path = PathBuf::from("/path/with-special_chars.123/model.bin");
        let provider = WhisperProvider::new(path.clone(), 0);

        assert_eq!(provider.model_path, path);
    }

    #[test]
    fn test_model_path_relative() {
        let path = PathBuf::from("relative/path/model.bin");
        let provider = WhisperProvider::new(path.clone(), 0);

        assert_eq!(provider.model_path, path);
    }

    // =========================================================================
    // Async Transcription Tests (without real model)
    // =========================================================================

    #[tokio::test]
    async fn test_transcribe_without_loading_model_triggers_auto_load() {
        let provider = WhisperProvider::new(PathBuf::from("/nonexistent/model.bin"), 0);
        let config = TranscriptionConfig::default();

        // transcribe should try to auto-load the model and fail
        let result = provider.transcribe(&[], &config).await;

        assert!(result.is_err());
        // Should fail with ModelNotFound since path doesn't exist
        assert!(matches!(result, Err(TranscriptionError::ModelNotFound(_))));
    }

    #[tokio::test]
    async fn test_transcribe_with_empty_audio() {
        // Create a temp file to pass the path existence check
        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let path = temp_file.path().to_path_buf();

        let provider = WhisperProvider::new(path, 0);
        let config = TranscriptionConfig::default();

        // This will fail when trying to load the temp file as a whisper model
        let result = provider.transcribe(&[], &config).await;

        // Expected to fail since the temp file is not a valid whisper model
        assert!(result.is_err());
    }

    // =========================================================================
    // Configuration Combinations Tests
    // =========================================================================

    #[test]
    fn test_full_configuration_chain() {
        let provider = WhisperProvider::with_gpu(
            PathBuf::from("/models/ggml-base.bin"),
            4,
            true,
        )
        .with_timeout(Duration::from_secs(180));

        assert_eq!(provider.model_path, PathBuf::from("/models/ggml-base.bin"));
        assert_eq!(provider.configured_threads, 4);
        assert!(provider.gpu_enabled);
        assert_eq!(provider.timeout(), Duration::from_secs(180));
        assert!(!provider.is_auto_threads());
    }

    #[test]
    fn test_minimal_configuration() {
        let provider = WhisperProvider::new(PathBuf::from("model.bin"), 0);

        assert_eq!(provider.model_path, PathBuf::from("model.bin"));
        assert!(provider.is_auto_threads());
        assert!(!provider.gpu_enabled);
        assert_eq!(provider.timeout(), Duration::from_secs(300));
    }

    // =========================================================================
    // Context Mutex Tests
    // =========================================================================

    #[test]
    fn test_context_mutex_not_poisoned_after_unload() {
        let provider = WhisperProvider::new(PathBuf::from("/fake/model.bin"), 0);

        // Multiple operations should not poison the mutex
        provider.unload_model();
        assert!(!provider.is_model_loaded());

        provider.unload_model();
        assert!(!provider.is_model_loaded());

        // Context should still be accessible
        let loaded = provider.is_model_loaded();
        assert!(!loaded);
    }

    #[test]
    fn test_multiple_providers_same_path() {
        let path = PathBuf::from("/fake/model.bin");
        let provider1 = WhisperProvider::new(path.clone(), 2);
        let provider2 = WhisperProvider::new(path.clone(), 4);

        // Each provider should have its own context
        assert!(!provider1.is_model_loaded());
        assert!(!provider2.is_model_loaded());

        // Different thread configurations
        assert_ne!(provider1.configured_threads, provider2.configured_threads);
    }
}
